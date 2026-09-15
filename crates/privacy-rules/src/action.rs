use serde::{Deserialize, Serialize};

/// 对命中实体的处理方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// 放行：该片段按原文转发给上游。
    Release,
    /// 抹去：该片段替换为占位文本后再转发。
    Redact,
}
