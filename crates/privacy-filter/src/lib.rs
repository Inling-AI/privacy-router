//! 基于 OpenAI Privacy Filter 与 Burn 的本地隐私实体识别。
//!
//! 使用 [`PrivacyFilter::from_dir`] 加载包含 `config.json`、`tokenizer.json`
//! 和 `model.safetensors` 的目录。此 crate 不发起网络请求，
//! 推理支持 Burn NdArray CPU 与 WGPU GPU，提供批量推理和可选观测接口。

#![doc = include_str!("../README.md")]

mod batch;
mod config;
mod decode;
mod filter;
mod inference;
mod label;
mod model;
pub mod observation;
mod pattern;
pub mod performance;
mod prediction;
mod registration;
mod weights;

pub use burn::backend::NdArray as Cpu;
#[cfg(feature = "gpu")]
pub use burn::backend::{Wgpu as Gpu, wgpu::WgpuDevice as GpuDevice};
use burn::tensor::backend::Backend;
use privacy_model::ModelFile;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokenizers::Tokenizer;

pub use batch::BatchLimits;
pub use decode::Decoding;
pub use filter::{Filter, FilterKind, Filters};
pub use label::{Boundary, EntityGroup, ModelLabel, TokenLabel};
pub use pattern::PatternFilter;
pub use prediction::TokenPrediction;
pub use registration::Registration;

/// 连续实体片段，序列化字段与 JS pipeline 示例一致。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    pub entity_group: EntityGroup,
    /// 实体内各 token 所选标签概率的算术平均值。
    pub score: f64,
    /// 实体在输入文本中的字节范围，已完成 UTF-8 字符边界对齐。
    /// 上层适配器据此定位并替换原文，无需搜索 `word` 子串。
    pub start: usize,
    pub end: usize,
    /// 输入文本的原始子串，保留 token 覆盖的空白字符；等于 `text[start..end]`。
    pub word: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid runtime options: {0}")]
    InvalidOptions(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("tokenizer error: {0}")]
    Tokenizer(String),
    #[error("unsupported or invalid model: {0}")]
    InvalidModel(String),
    #[error("input has {actual} tokens, exceeding limit {limit}")]
    InputTooLong { actual: usize, limit: usize },
    #[error("inference error: {0}")]
    Inference(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// 运行选项。超出长度限制时返回错误，不静默截断输入。
#[derive(Debug, Clone)]
pub struct Options {
    pub max_tokens: usize,
    pub decoding: Decoding,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            decoding: Decoding::Simple,
        }
    }
}

/// 可重复使用的推理引擎；权重加载一次并常驻所选设备。
/// 默认使用 CPU；启用 gpu feature 后可选择 WGPU / Metal。
pub struct PrivacyFilter<B: Backend = Cpu<f32>> {
    model: model::Model<B>,
    tokenizer: Tokenizer,
    options: Options,
}

impl PrivacyFilter<Cpu<f32>> {
    pub fn from_dir(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_dir_with_options(path, Options::default())
    }

    pub fn from_dir_with_options(path: impl AsRef<Path>, options: Options) -> Result<Self> {
        Self::from_dir_on_device(path, options, &Default::default())
    }
}

impl<B: Backend> PrivacyFilter<B> {
    /// 在指定 Burn 设备上创建模型；权重常驻该设备，后续调用不重复上传。
    pub fn from_dir_on_device(
        path: impl AsRef<Path>,
        options: Options,
        device: &B::Device,
    ) -> Result<Self> {
        let path = path.as_ref();
        let config: config::Config =
            serde_json::from_reader(std::fs::File::open(ModelFile::Config.path(path))?)?;
        config.validate()?;
        if options.max_tokens == 0 || options.max_tokens > config.max_position_embeddings {
            return Err(Error::InvalidModel(
                "max_tokens must be positive and within max_position_embeddings".into(),
            ));
        }
        let mut tokenizer = Tokenizer::from_file(ModelFile::Tokenizer.path(path))
            .map_err(|e| Error::Tokenizer(e.to_string()))?;
        tokenizer
            .with_truncation(None)
            .map_err(|e| Error::Tokenizer(e.to_string()))?;
        tokenizer.with_padding(None);
        let model = model::Model::load(ModelFile::Weights.path(path), config, device)?;
        Ok(Self {
            model,
            tokenizer,
            options,
        })
    }

    /// 识别单段文本中的实体；空输入返回空列表。
    pub fn classify(&self, text: &str) -> Result<Vec<Entity>> {
        self.classify_batch(&[text], self.single_limits())
            .map(|mut output| output.remove(0))
    }

    /// 批量识别，输出顺序与输入一致。不同输入的注意力与位置编码相互隔离。
    pub fn classify_batch(&self, texts: &[&str], limits: BatchLimits) -> Result<Vec<Vec<Entity>>> {
        self.infer::<Entity>(texts, limits, None)
    }

    /// 按本次调用注入观测器；首个结果时间在实体聚合完成后记录。
    pub fn classify_observed(
        &self,
        text: &str,
        observation: &observation::Observation<'_>,
    ) -> Result<Vec<Entity>> {
        self.classify_batch_observed(&[text], self.single_limits(), observation)
            .map(|mut output| output.remove(0))
    }

    /// 带观测的批量实体识别；返回错误时也会发出一个终止失败事件。
    pub fn classify_batch_observed(
        &self,
        texts: &[&str],
        limits: BatchLimits,
        observation: &observation::Observation<'_>,
    ) -> Result<Vec<Vec<Entity>>> {
        self.infer::<Entity>(texts, limits, Some(observation))
    }

    /// 返回每个 token 的字节偏移及原始分类分数。
    pub fn predict_tokens(&self, text: &str) -> Result<Vec<TokenPrediction>> {
        self.predict_tokens_batch(&[text], self.single_limits())
            .map(|mut output| output.remove(0))
    }

    /// 真正打包执行的批量推理；超出预算返回错误，不截断输入。
    pub fn predict_tokens_batch(
        &self,
        texts: &[&str],
        limits: BatchLimits,
    ) -> Result<Vec<Vec<TokenPrediction>>> {
        self.infer::<TokenPrediction>(texts, limits, None)
    }

    /// 带观测的单输入 token 分类；首个结果时间对应分类分数准备就绪。
    pub fn predict_tokens_observed(
        &self,
        text: &str,
        observation: &observation::Observation<'_>,
    ) -> Result<Vec<TokenPrediction>> {
        self.predict_tokens_batch_observed(&[text], self.single_limits(), observation)
            .map(|mut output| output.remove(0))
    }

    /// 带观测的批量 token 分类，进度中的下标对应本次输入位置。
    pub fn predict_tokens_batch_observed(
        &self,
        texts: &[&str],
        limits: BatchLimits,
        observation: &observation::Observation<'_>,
    ) -> Result<Vec<Vec<TokenPrediction>>> {
        self.infer::<TokenPrediction>(texts, limits, Some(observation))
    }

    fn single_limits(&self) -> BatchLimits {
        BatchLimits {
            max_sequences: 1,
            max_tokens: self.options.max_tokens,
        }
    }
}
