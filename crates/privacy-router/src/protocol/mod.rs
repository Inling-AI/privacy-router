//! 协议适配：在三类报文中定位需要脱敏的文本，并把替换结果写回。
//!
//! 适配器只负责「哪些字段是内容」，判定与替换由 [`crate::redaction`] 完成。三个实现共用
//! 同一套收集与回写机制，不复制脱敏逻辑。文本字段保留 JSON Pointer 与已知输出包装，
//! 回写时只替换正文，不改动工具元数据。
//!
//! 内容是**模型自己写下的文本之外**的一切：客户端给的指令与消息，以及工具返回的结果。
//! 模型自己的输出不参与判定（见 [`is_model_output`]）。

mod anthropic;
mod openai;

use privacy_store::ApiFormat;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 报文中的一段文本及其位置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextField {
    /// RFC 6901 JSON Pointer，同时用于日志定位与回写。
    pub pointer: String,
    pub text: String,
    envelope: Option<Value>,
}

impl TextField {
    /// 恢复原始包装后生成可由 `apply` 写回的替换项。
    pub fn replacement(&self, text: String) -> (String, String) {
        let text = match &self.envelope {
            Some(envelope) => {
                let mut envelope = envelope.clone();
                envelope["output"] = Value::String(text);
                envelope.to_string()
            }
            None => text,
        };
        (self.pointer.clone(), text)
    }

    pub(super) fn collect_exec_output(body: &Value, pointer: &str, out: &mut Vec<Self>) {
        let start = out.len();
        push_text_or_blocks(body, pointer, out);
        let fields = out.drain(start..).collect::<Vec<_>>();
        for mut field in fields {
            if let Ok(value) = serde_json::from_str::<Value>(&field.text)
                && value.get("chunk_id").is_some_and(Value::is_string)
                && value.get("wall_time_seconds").is_some_and(Value::is_number)
                && let Some(text) = value.get("output").and_then(Value::as_str)
            {
                field.text = text.to_owned();
                field.envelope = Some(value);
            }
            // exec emits a separate status block with no tool output.
            let lines = field.text.lines().collect::<Vec<_>>();
            let status = field.envelope.is_none()
                && lines.len() == 3
                && lines[0] == "Script completed"
                && lines[1]
                    .strip_prefix("Wall time ")
                    .and_then(|s| s.strip_suffix(" seconds"))
                    .is_some_and(|s| s.parse::<f64>().is_ok())
                && lines[2] == "Output:";
            if !status && !field.text.is_empty() {
                out.push(field);
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("request body must be a JSON object")]
    NotAnObject,
    #[error("malformed field at {pointer}: {reason}")]
    Malformed { pointer: String, reason: String },
    #[error("field {pointer} disappeared before it could be rewritten")]
    MissingField { pointer: String },
}

/// 一类上游报文的字段布局。
pub trait ApiProtocol: Send + Sync {
    fn format(&self) -> ApiFormat;

    /// 收集本次请求中所有需要判定的文本。顺序即回写顺序，不要求调用方理解报文结构。
    fn collect(&self, body: &Value) -> Result<Vec<TextField>, ProtocolError>;
}

/// 取得协议对应的适配器。三个实现都是无状态单例。
pub fn adapter(format: ApiFormat) -> &'static dyn ApiProtocol {
    match format {
        ApiFormat::OpenAiResponses => &openai::Responses,
        ApiFormat::OpenAiChat => &openai::Chat,
        ApiFormat::AnthropicMessages => &anthropic::Messages,
    }
}

/// 按指针把替换后的文本写回报文。
pub fn apply(body: &mut Value, replacements: &[(String, String)]) -> Result<(), ProtocolError> {
    for (pointer, text) in replacements {
        match body.pointer_mut(pointer) {
            Some(slot @ Value::String(_)) => *slot = Value::String(text.clone()),
            _ => {
                return Err(ProtocolError::MissingField {
                    pointer: pointer.clone(),
                });
            }
        }
    }
    Ok(())
}

/// 若该指针指向字符串，收下它。
fn push_string(body: &Value, pointer: &str, out: &mut Vec<TextField>) {
    if let Some(Value::String(text)) = body.pointer(pointer) {
        out.push(TextField {
            pointer: pointer.to_owned(),
            text: text.clone(),
            envelope: None,
        });
    }
}

/// 收集数组元素里的 `text` 字段，用于 content / system 这类「字符串或块数组」的字段。
fn push_block_text(body: &Value, pointer: &str, out: &mut Vec<TextField>) {
    let Some(Value::Array(items)) = body.pointer(pointer) else {
        return;
    };
    for index in 0..items.len() {
        push_string(body, &format!("{pointer}/{index}/text"), out);
    }
}

/// 「字符串或块数组」两种形态都要覆盖，调用方不必先判断类型。
fn push_text_or_blocks(body: &Value, pointer: &str, out: &mut Vec<TextField>) {
    push_string(body, pointer, out);
    push_block_text(body, pointer, out);
}

/// 适配器共用的入口检查：报文必须是一个 JSON 对象。
fn require_object(body: &Value) -> Result<(), ProtocolError> {
    if body.is_object() {
        Ok(())
    } else {
        Err(ProtocolError::NotAnObject)
    }
}

/// 这一段文本是不是模型自己写下的。
///
/// 模型自己的输出不参与脱敏：这些内容本来就是我们从上游放过去的，再审查一遍既拦不住新的
/// 泄露，又让上游看到的上下文与模型真正写下的不一致。剩下的都在保护范围内——客户端提供的
/// 指令与消息（`user`、`system`、`developer`），以及模型探索时工具返回的结果。
///
/// 条目没有角色时看类型：模型发起的调用（`*_call`）与推理摘要都是模型产物，工具结果按
/// 约定以 `_output` 结尾。
fn is_model_output(item: &Value) -> bool {
    match item.get("role").and_then(Value::as_str) {
        Some("assistant") => true,
        Some(_) => false,
        None => item
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind.ends_with("_call") || kind == "reasoning"),
    }
}
