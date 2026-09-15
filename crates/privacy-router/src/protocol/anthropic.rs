//! Anthropic Messages 报文。
//!
//! 与 OpenAI 的差别主要在两处：顶层用 `system`（字符串或块数组），内容块里还有
//! `tool_result.content` 这种嵌套内容。工具调用参数与 assistant 消息都是模型写下的原文，
//! 不参与识别或替换。

use super::{
    ApiProtocol, ProtocolError, TextField, is_model_output, push_text_or_blocks, require_object,
};
use privacy_store::ApiFormat;
use serde_json::Value;

pub(super) struct Messages;

impl ApiProtocol for Messages {
    fn format(&self) -> ApiFormat {
        ApiFormat::AnthropicMessages
    }

    fn collect(&self, body: &Value) -> Result<Vec<TextField>, ProtocolError> {
        require_object(body)?;
        let mut fields = Vec::new();

        push_text_or_blocks(body, "/system", &mut fields);

        if let Some(Value::Array(messages)) = body.pointer("/messages") {
            for (index, message) in messages.iter().enumerate() {
                if is_model_output(message) {
                    continue;
                }
                let base = format!("/messages/{index}");
                push_text_or_blocks(body, &format!("{base}/content"), &mut fields);

                let Some(Value::Array(blocks)) = body.pointer(&format!("{base}/content")) else {
                    continue;
                };
                for block in 0..blocks.len() {
                    let block = format!("{base}/content/{block}");
                    // tool_result 的内容可以再嵌一层字符串或块数组。
                    push_text_or_blocks(body, &format!("{block}/content"), &mut fields);
                }
            }
        }

        Ok(fields)
    }
}
