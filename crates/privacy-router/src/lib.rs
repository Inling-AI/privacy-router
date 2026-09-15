//! 隐私保护代理：识别上游请求中的隐私内容，按规则判定，把抹去后的文本转发出去。
//!
//! 分层的边界是隐私保证的核心：
//!
//! - [`protocol`] 只回答「报文里哪些字段是内容」，不碰判定；
//! - [`redaction`] 只回答「这些内容该被抹去还是放行」，不做 IO；
//! - 推理、存储、转发与控制台在各自模块内，均不允许绕过上面两层直接改动报文。
//!
//! 任何一步失败都不转发原文：解析失败、判定失败、存储不可用都不会降级成透传。
#![doc = include_str!("../README.md")]

pub mod classifier;
pub mod config;
pub mod console;
mod content;
pub mod error;
pub mod inference;
pub mod pipeline;
pub mod prefix;
pub mod protocol;
pub mod proxy;
pub mod redaction;
pub mod server;
pub mod state;
pub mod web;

pub use classifier::{Classification, Classifier};
pub use inference::InferenceEngine;
pub use pipeline::{Processed, RequestContext, Workload, process};
pub use protocol::{ApiProtocol, ProtocolError, TextField, adapter, apply as apply_replacements};
pub use redaction::{DecidedSpan, Redaction, placeholder, redact};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("store error: {0}")]
    Store(#[from] privacy_store::Error),
    #[error("inference error: {0}")]
    Inference(#[from] privacy_filter::Error),
    #[error("classifier returned {results} results for {fields} content fields")]
    ClassificationShape { fields: usize, results: usize },
}

pub type Result<T> = std::result::Result<T, Error>;
