//! 本地推理：在后端之上加结果缓存、串行化与超长文本分块。

use crate::classifier::{Classification, ClassifiedField, Classifier};
use burn::tensor::StreamId;
use privacy_filter::observation::{EventKind, InferenceEvent, Observation, Observer};
use privacy_filter::performance::ModelPerformance;
use privacy_filter::{BatchLimits, Entity, Error, PrivacyFilter, Result};
use privacy_filter_cache::{CacheUsage, CachedBatch, CachedPrivacyFilter};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::Instant;

struct RequestObserver {
    started: Instant,
    performance: RefCell<ModelPerformance>,
}

impl Observer for RequestObserver {
    fn on_event(&self, event: &InferenceEvent) {
        if matches!(event.kind, EventKind::InputCompleted(_))
            && event.metrics.progress.completed_tokens > 0
        {
            let mut performance = self.performance.borrow_mut();
            if performance.first_result_ms.is_none() {
                performance.first_result_ms = Some(self.started.elapsed().as_secs_f64() * 1000.0);
            }
        }
        if matches!(event.kind, EventKind::Completed | EventKind::Failed { .. }) {
            self.performance.borrow_mut().record(&event.metrics, None);
        }
    }
}

/// 单进程内的推理引擎。
///
/// 后端不是为并发重入设计的，权重又常驻设备，因此这里用互斥锁把识别串行化；等待锁的时间
/// 会在调用方被单独计为排队时间，不会被混入推理耗时。
/// 同时固定后端 stream：串行不代表线程固定，Tokio 的 blocking pool 会更换线程，
/// 而 CubeCL 按 stream 保留设备内存池，不能让每个调用线程各自积累一份推理工作区。
pub struct InferenceEngine<B: burn::tensor::backend::Backend> {
    classifier: Mutex<CachedPrivacyFilter<B>>,
    stream: StreamId,
    limits: BatchLimits,
    identity: Option<String>,
}

impl<B: burn::tensor::backend::Backend> InferenceEngine<B> {
    pub fn new(classifier: CachedPrivacyFilter<B>, limits: BatchLimits) -> Self {
        Self {
            classifier: Mutex::new(classifier),
            stream: StreamId::current(),
            limits,
            identity: None,
        }
    }

    /// 批量识别并按需分块。返回缓存用量与推理耗时。
    fn run(&self, texts: &[&str]) -> Result<Classification> {
        let prepared: Vec<_> = texts
            .iter()
            .map(|text| crate::content::Content::new(text).inference_text())
            .collect();
        let texts: Vec<_> = prepared.iter().map(|text| text.as_ref()).collect();
        let texts = texts.as_slice();
        // 锁中毒说明上一次识别 panic 了，此时继续复用可能已损坏的状态是危险的。
        let mut classifier = self.classifier.lock().map_err(|_| {
            Error::Inference("inference lock poisoned by an earlier failure".into())
        })?;
        // 必须在持锁期间进入 stream；executes 在返回、错误或 unwind 后恢复调用方的 stream。
        self.stream
            .executes(|| self.run_on_stream(texts, &mut classifier))
    }

    fn run_on_stream(
        &self,
        texts: &[&str],
        classifier: &mut CachedPrivacyFilter<B>,
    ) -> Result<Classification> {
        let started = Instant::now();
        let observer = RequestObserver {
            started,
            performance: RefCell::new(ModelPerformance::default()),
        };
        let observation = Observation::new(&observer);

        let (entities, usage) =
            match classifier.classify_batch_observed(texts, self.limits, &observation) {
                Ok(batch) => (
                    batch.entities.into_iter().map(|arc| arc.to_vec()).collect(),
                    batch.usage,
                ),
                Err(Error::InputTooLong { .. }) => {
                    // 批次里含有超长输入。缓存对单条输入仍然有效，因此退化成逐条处理，
                    // 只对确实超长的那条分块。
                    let mut usage = CacheUsage::default();
                    let mut entities = Vec::with_capacity(texts.len());
                    for text in texts {
                        let single =
                            classifier.classify_batch_observed(&[*text], self.limits, &observation);
                        match single {
                            Ok(batch) => {
                                usage.cached_fragments += batch.usage.cached_fragments;
                                usage.deduplicated_fragments += batch.usage.deduplicated_fragments;
                                usage.inferred_fragments += batch.usage.inferred_fragments;
                                entities.push(
                                    batch
                                        .entities
                                        .into_iter()
                                        .next()
                                        .unwrap_or_default()
                                        .to_vec(),
                                );
                            }
                            Err(Error::InputTooLong { .. }) => {
                                let batch = ContextualText::new(text).classify_with(|chunks| {
                                    classifier.classify_batch_observed(
                                        chunks,
                                        self.limits,
                                        &observation,
                                    )
                                })?;
                                usage.cached_fragments += batch.usage.cached_fragments;
                                usage.deduplicated_fragments += batch.usage.deduplicated_fragments;
                                usage.inferred_fragments += batch.usage.inferred_fragments;
                                entities.push(batch.entities[0].to_vec());
                            }
                            Err(other) => return Err(other),
                        }
                    }
                    (entities, usage)
                }
                Err(other) => return Err(other),
            };

        Ok(Classification {
            fields: entities
                .into_iter()
                .map(ClassifiedField::inferred)
                .collect(),
            usage,
            elapsed_ms: started.elapsed().as_millis() as i64,
            performance: observer.performance.into_inner(),
        })
    }
}

impl<B: burn::tensor::backend::Backend> Classifier for InferenceEngine<B> {
    fn cache_identity(&self) -> Option<&str> {
        self.identity.as_deref()
    }
    fn classify(&self, texts: &[&str]) -> Result<Vec<Vec<Entity>>> {
        Ok(self
            .run(texts)?
            .fields
            .into_iter()
            .map(|field| field.entities)
            .collect())
    }

    fn classify_recorded(&self, texts: &[&str]) -> Result<Classification> {
        self.run(texts)
    }
}

/// 单段文本的有界推理：保持 UTF-8 坐标，以半窗口步长提供双向上下文。
///
/// 后端以批次回调注入，本类型负责超限重试、实体重组和工作量统计。
/// 重叠宽度以内的连续文本必然完整落入某个窗口；更长实体仍受模型上下文上限约束。
pub struct ContextualText<'a> {
    text: &'a str,
}

impl<'a> ContextualText<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text }
    }

    pub fn classify_with(
        &self,
        mut classify: impl FnMut(&[&str]) -> Result<CachedBatch>,
    ) -> Result<CachedBatch> {
        let (actual, limit) = match classify(&[self.text]) {
            Ok(batch) => return Ok(batch),
            Err(Error::InputTooLong { actual, limit }) => (actual, limit),
            Err(error) => return Err(error),
        };
        let mut boundaries: Vec<usize> = self.text.char_indices().map(|(i, _)| i).collect();
        boundaries.push(self.text.len());
        let characters = boundaries.len() - 1;
        let mut budget =
            ((characters as f64 * limit as f64 / actual.max(1) as f64) * 0.9).floor() as usize;
        budget = budget.max(1).min(characters.max(1));

        let initial_budget = budget;
        let mut start = 0;
        let mut entities = Vec::new();
        let mut usage = CacheUsage::default();
        while start < characters {
            let end = (start + budget).min(characters);
            let range = boundaries[start]..boundaries[end];
            let chunk = &self.text[range.clone()];
            let batch = match classify(&[chunk]) {
                Ok(batch) => batch,
                // 仅收窄超限的局部窗口，避免密集文字使后面的低密度文本也丢失上下文。
                Err(Error::InputTooLong { actual, limit }) if budget > 1 => {
                    budget = ((budget as f64 * limit as f64 / actual.max(1) as f64) * 0.9).floor()
                        as usize;
                    budget = budget.clamp(1, (end - start).saturating_sub(1).max(1));
                    continue;
                }
                Err(error) => return Err(error),
            };
            if batch.entities.len() != 1 {
                return Err(Error::Inference("chunk result count mismatch".into()));
            }
            usage.cached_fragments += batch.usage.cached_fragments;
            usage.deduplicated_fragments += batch.usage.deduplicated_fragments;
            usage.inferred_fragments += batch.usage.inferred_fragments;
            for entity in batch.entities[0].iter() {
                if entity.start >= entity.end || chunk.get(entity.start..entity.end).is_none() {
                    return Err(Error::Inference("invalid chunk entity offsets".into()));
                }
                let mut entity = entity.clone();
                entity.start += range.start;
                entity.end += range.start;
                entity.word = self.text[entity.start..entity.end].to_owned();
                entities.push(entity);
            }
            if end == characters {
                break;
            }
            start += (budget / 2).max(1);
            budget = initial_budget;
        }
        Ok(CachedBatch {
            entities: vec![Arc::from(self.reconcile(entities))],
            usage,
        })
    }

    /// 同类别重叠观察合并范围，置信度取最高观察值；相邻但不重叠的实体保持独立。
    /// 不同类别保留各自判定，最终抹去范围由 redaction 合并，放行不会盖掉抹去。
    fn reconcile(&self, mut entities: Vec<Entity>) -> Vec<Entity> {
        entities.sort_by(|a, b| {
            a.entity_group
                .as_ref()
                .cmp(b.entity_group.as_ref())
                .then_with(|| (a.start, a.end).cmp(&(b.start, b.end)))
        });
        let mut merged: Vec<Entity> = Vec::new();
        for entity in entities {
            if let Some(previous) = merged.last_mut()
                && previous.entity_group == entity.entity_group
                && entity.start < previous.end
            {
                previous.end = previous.end.max(entity.end);
                previous.score = previous.score.max(entity.score);
                previous.word = self.text[previous.start..previous.end].to_owned();
            } else {
                merged.push(entity);
            }
        }
        merged.sort_by_key(|entity| (entity.start, entity.end));
        merged
    }
}

/// 加载模型并装配引擎。加载一次性完成，权重常驻设备。
pub fn load<B: burn::tensor::backend::Backend>(
    model_dir: &std::path::Path,
    device: &B::Device,
    options: privacy_filter::Options,
    cache: privacy_filter_cache::CacheLimits,
    limits: BatchLimits,
) -> Result<InferenceEngine<B>> {
    let mut identity = blake3::Hasher::new();
    identity.update(b"privacy-inference-v2-image-isolation");
    identity.update(format!("{options:?}/{limits:?}/{}", std::any::type_name::<B>()).as_bytes());
    for file in privacy_model::ModelFile::all() {
        identity.update(file.as_ref().as_bytes());
        identity.update_reader(std::fs::File::open(file.path(model_dir))?)?;
    }
    let filter = PrivacyFilter::<B>::from_dir_on_device(model_dir, options, device)?;
    let cached = CachedPrivacyFilter::new(filter, cache)?;
    let mut engine = InferenceEngine::new(cached, limits);
    engine.identity = Some(identity.finalize().to_hex().to_string());
    Ok(engine)
}
