//! OpenAI 两种报文形态：Responses 与 Chat Completions。
//!
//! 两者都把内容放在多个可选位置（字符串或块数组），因此逐个位置尝试收集，而不是先判断
//! 类型再分支——报文里同一字段在不同版本里形态不同，宽松收集比严格判别更不易漏字段。
//! 条目先过一遍模型产物的筛子：模型自己写下的消息、工具调用参数与推理摘要是放过去的原文，
//! 不参与脱敏。

use super::{
    ApiProtocol, ProtocolError, TextField, is_model_output, push_string, push_text_or_blocks,
    require_object,
};
use privacy_store::ApiFormat;
use serde_json::Value;

/// `/v1/responses`
pub(super) struct Responses;

impl ApiProtocol for Responses {
    fn format(&self) -> ApiFormat {
        ApiFormat::OpenAiResponses
    }

    fn collect(&self, body: &Value) -> Result<Vec<TextField>, ProtocolError> {
        require_object(body)?;
        let mut fields = Vec::new();

        push_text_or_blocks(body, "/instructions", &mut fields);
        push_text_or_blocks(body, "/input", &mut fields);

        // input 既可以是字符串，也可以是条目数组；条目内部还可能嵌套内容块。
        if let Some(Value::Array(items)) = body.pointer("/input") {
            for (index, item) in items.iter().enumerate() {
                if is_model_output(item) {
                    continue;
                }
                let base = format!("/input/{index}");
                push_string(body, &base, &mut fields);
                push_text_or_blocks(body, &format!("{base}/content"), &mut fields);
                if is_exec_output(items, item) {
                    TextField::collect_exec_output(body, &format!("{base}/output"), &mut fields);
                } else {
                    push_text_or_blocks(body, &format!("{base}/output"), &mut fields);
                }
            }
        }

        Ok(fields)
    }
}

/// exec 的工具输出是一个 JSON 信封，正文在它的 `output` 字段里，需要单独拆包。
///
/// 判据是「同名 exec 调用的返回」：别的工具返回的 JSON 原样当文本处理。
fn is_exec_output(items: &[Value], item: &Value) -> bool {
    item.get("type").and_then(Value::as_str) == Some("custom_tool_call_output")
        && item
            .get("call_id")
            .and_then(Value::as_str)
            .is_some_and(|id| {
                items.iter().any(|call| {
                    call.get("type").and_then(Value::as_str) == Some("custom_tool_call")
                        && call.get("name").and_then(Value::as_str) == Some("exec")
                        && call.get("call_id").and_then(Value::as_str) == Some(id)
                })
            })
}

/// `/v1/chat/completions`
pub(super) struct Chat;

impl ApiProtocol for Chat {
    fn format(&self) -> ApiFormat {
        ApiFormat::OpenAiChat
    }

    fn collect(&self, body: &Value) -> Result<Vec<TextField>, ProtocolError> {
        require_object(body)?;
        let mut fields = Vec::new();

        if let Some(Value::Array(messages)) = body.pointer("/messages") {
            for (index, message) in messages.iter().enumerate() {
                if is_model_output(message) {
                    continue;
                }
                push_text_or_blocks(body, &format!("/messages/{index}/content"), &mut fields);
            }
        }

        Ok(fields)
    }
}
