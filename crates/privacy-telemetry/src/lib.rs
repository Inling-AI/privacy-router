//! 进程内唯一的日志入口：所有日志经此安装的 `tracing` 订阅器统一格式化。
//!
//! 本 crate 不依赖推理、存储或 HTTP，也不定义业务事件类型；它只回答「日志长什么样、
//! 写到哪里、如何收尾」。调用方在 `main` 开头安装一次并持有 [`TelemetryGuard`]。
#![doc = include_str!("../README.md")]

mod tee;

use std::io::IsTerminal;
use std::path::PathBuf;
use tracing::Subscriber;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub use tee::{Tee, TeeWriter};

/// 日志的编码方式。两种格式共用同一份字段来源，只是序列化方式不同。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// 面向终端阅读的单行文本。
    Human,
    /// 每行一个 JSON 对象，供日志采集使用。
    Json,
}

/// 安装参数。
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub format: LogFormat,
    /// `tracing-subscriber` 的 `EnvFilter` 指令串，例如 `info,privacy_router=debug`。
    pub filter: String,
    /// 同时写入的滚动日志目录；`None` 表示只写终端。
    pub log_dir: Option<PathBuf>,
    /// 是否着色；`None` 表示按终端能力自动判断。
    pub ansi: Option<bool>,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::Human,
            filter: "info".to_owned(),
            log_dir: None,
            ansi: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid log filter directive: {0}")]
    InvalidFilter(String),
    #[error("log directory error: {0}")]
    LogDirectory(#[from] std::io::Error),
    #[error("a global tracing subscriber is already installed: {0}")]
    AlreadyInstalled(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// 持有非阻塞写入器的收尾责任。进程退出前不得丢弃。
#[derive(Debug)]
pub struct TelemetryGuard {
    _worker: Option<WorkerGuard>,
}

/// 安装全局订阅器的唯一入口。
pub struct Telemetry;

impl Telemetry {
    /// 安装全局订阅器。重复安装返回 [`Error::AlreadyInstalled`] 而不是 panic。
    ///
    /// 必须在创建任何异步运行时任务之前调用，否则早期日志会丢失。
    pub fn init(config: &TelemetryConfig) -> Result<TelemetryGuard> {
        let (writer, worker) = Self::writer(config)?;
        Self::subscriber(config, writer)?
            .try_init()
            .map_err(|error| Error::AlreadyInstalled(error.to_string()))?;
        Ok(TelemetryGuard { _worker: worker })
    }

    /// 用给定 writer 构造订阅器但不安装，供测试与嵌入场景断言格式契约。
    pub fn subscriber<W>(
        config: &TelemetryConfig,
        writer: W,
    ) -> Result<Box<dyn Subscriber + Send + Sync>>
    where
        W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
    {
        // 非法指令必须变成错误而不是 panic：过滤串来自配置文件与命令行。
        let filter = EnvFilter::try_new(&config.filter)
            .map_err(|error| Error::InvalidFilter(error.to_string()))?;
        let registry = tracing_subscriber::registry().with(filter);
        let ansi = config
            .ansi
            .unwrap_or_else(|| std::io::stderr().is_terminal());

        Ok(match config.format {
            LogFormat::Human => Box::new(
                registry.with(
                    fmt::layer()
                        .with_writer(writer)
                        .with_ansi(ansi)
                        .with_target(true),
                ),
            ),
            LogFormat::Json => Box::new(
                registry.with(
                    fmt::layer()
                        .json()
                        .with_writer(writer)
                        // JSON 里的转义序列只会妨碍采集端解析。
                        .with_ansi(false)
                        .with_current_span(true)
                        .with_span_list(true),
                ),
            ),
        })
    }

    /// 组装最终写入目标：终端，外加可选的滚动文件。两者共用同一个格式化层。
    fn writer(config: &TelemetryConfig) -> Result<(BoxMakeWriter, Option<WorkerGuard>)> {
        let Some(directory) = &config.log_dir else {
            return Ok((BoxMakeWriter::new(std::io::stderr), None));
        };
        std::fs::create_dir_all(directory)?;
        let file = tracing_appender::rolling::daily(directory, "privacy-router.log");
        let (file, worker) = tracing_appender::non_blocking(file);
        Ok((
            BoxMakeWriter::new(Tee::new(std::io::stderr, file)),
            Some(worker),
        ))
    }
}
