//! 协议适配器的行为契约：收哪些字段、回写只改这些字段。

use privacy_router::protocol::{adapter, apply};
use privacy_store::ApiFormat;
use rstest::rstest;
use serde_json::{Value, json};

fn collect(format: ApiFormat, body: &Value) -> Vec<(String, String)> {
    adapter(format)
        .collect(body)
        .expect("合法报文必须可收集")
        .into_iter()
        .map(|field| (field.pointer, field.text))
        .collect()
}

#[rstest]
fn non_object_bodies_are_rejected(
    #[values(
        ApiFormat::OpenAiResponses,
        ApiFormat::OpenAiChat,
        ApiFormat::AnthropicMessages
    )]
    format: ApiFormat,
) {
    for body in [json!("a string"), json!([1, 2]), json!(42)] {
        assert!(
            adapter(format).collect(&body).is_err(),
            "{format:?} 必须拒绝非对象报文：{body}"
        );
    }
}

// ---------------------------------------------------------------------------
// OpenAI Responses
// ---------------------------------------------------------------------------

#[rstest]
fn responses_collects_instructions_and_string_input() {
    let body = json!({
        "model": "gpt-5",
        "instructions": "You are an assistant for alice@corp.com",
        "input": "my email is bob@corp.com"
    });

    let fields = collect(ApiFormat::OpenAiResponses, &body);

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].0, "/instructions");
    assert_eq!(fields[0].1, "You are an assistant for alice@corp.com");
    assert_eq!(fields[1].0, "/input");
    assert_eq!(fields[1].1, "my email is bob@corp.com");
}

#[rstest]
fn responses_collects_input_items_contents_and_outputs() {
    let body = json!({
        "input": [
            {"type": "message", "role": "user", "content": "plain string content"},
            {"type": "message", "role": "user", "content": [
                {"type": "input_text", "text": "block text"},
                {"type": "input_image", "image_url": "https://example.com/a.png"}
            ]},
            {"type": "function_call_output", "call_id": "c1", "output": "tool output text"}
        ]
    });

    let fields = collect(ApiFormat::OpenAiResponses, &body);
    let paths: Vec<&str> = fields.iter().map(|field| field.0.as_str()).collect();

    assert_eq!(
        paths,
        [
            "/input/0/content",
            "/input/1/content/0/text",
            "/input/2/output"
        ]
    );
    assert_eq!(fields[1].1, "block text");
}

#[rstest]
fn responses_ignores_fields_that_are_not_content() {
    let body = json!({
        "model": "gpt-5",
        "temperature": 0.7,
        "tools": [{"type": "function", "name": "lookup", "parameters": {"type": "object"}}],
        "input": []
    });

    assert!(collect(ApiFormat::OpenAiResponses, &body).is_empty());
}

// ---------------------------------------------------------------------------
// OpenAI Chat Completions
// ---------------------------------------------------------------------------

#[rstest]
fn chat_collects_message_content_in_both_shapes() {
    let body = json!({
        "messages": [
            {"role": "system", "content": "system prompt"},
            {"role": "user", "content": [{"type": "text", "text": "part text"}]},
            {"role": "assistant", "content": null}
        ]
    });

    let fields = collect(ApiFormat::OpenAiChat, &body);

    assert_eq!(
        fields,
        [
            ("/messages/0/content".to_owned(), "system prompt".to_owned()),
            (
                "/messages/1/content/0/text".to_owned(),
                "part text".to_owned()
            ),
        ]
    );
}

#[rstest]
fn chat_excludes_tool_call_arguments() {
    let body = json!({
        "messages": [
            {"role": "assistant", "tool_calls": [
                {"id": "c1", "type": "function", "function": {
                    "name": "send",
                    "arguments": "{\"to\":\"alice@corp.com\"}"
                }}
            ]},
            {"role": "assistant", "function_call": {"name": "send", "arguments": "{\"to\":\"bob@corp.com\"}"}}
        ]
    });

    let fields = collect(ApiFormat::OpenAiChat, &body);

    assert!(fields.is_empty());
}

#[rstest]
fn chat_ignores_model_parameters_and_tool_schemas() {
    let body = json!({
        "model": "gpt-5",
        "temperature": 1,
        "tools": [{"type": "function", "function": {"name": "x", "description": "nothing"}}],
        "messages": []
    });

    assert!(collect(ApiFormat::OpenAiChat, &body).is_empty());
}

// ---------------------------------------------------------------------------
// 模型自己的输出
// ---------------------------------------------------------------------------

/// 模型写下的文本不参与判定。
///
/// 这些内容本来就是我们从上游放过去的：再审查一遍拦不住任何新的泄露，却会让上游看到的上下文
/// 与模型真正写下的不一致。三个协议都必须把 assistant 的正文与工具调用参数排除在外。
#[rstest]
#[case(ApiFormat::OpenAiResponses)]
#[case(ApiFormat::OpenAiChat)]
#[case(ApiFormat::AnthropicMessages)]
fn model_written_text_is_not_collected(#[case] format: ApiFormat) {
    let body = match format {
        ApiFormat::OpenAiResponses => json!({"input": [
            {"type": "message", "role": "user", "content": "user text"},
            {"type": "message", "role": "assistant", "content": [
                {"type": "output_text", "text": "assistant text"}
            ]}
        ]}),
        ApiFormat::OpenAiChat => json!({"messages": [
            {"role": "user", "content": "user text"},
            {"role": "assistant", "content": "assistant text"}
        ]}),
        ApiFormat::AnthropicMessages => json!({"messages": [
            {"role": "user", "content": "user text"},
            {"role": "assistant", "content": [
                {"type": "text", "text": "assistant text"},
                {"type": "tool_use", "id": "t1", "name": "send", "input": {"to": "assistant args"}}
            ]}
        ]}),
    };

    let fields = collect(format, &body);

    assert_eq!(
        fields,
        [(field_of(format, 0), "user text".to_owned())],
        "{format:?} 只应收下客户端提供的那一条"
    );
}

/// 客户端提供的那一条在各自协议里的位置。
fn field_of(format: ApiFormat, index: usize) -> String {
    match format {
        ApiFormat::OpenAiResponses => format!("/input/{index}/content"),
        ApiFormat::OpenAiChat | ApiFormat::AnthropicMessages => {
            format!("/messages/{index}/content")
        }
    }
}

/// 模型发起的调用、推理摘要和它的工具调用参数同样是模型产物；工具返回的结果继续脱敏。
#[rstest]
fn responses_skips_model_tool_calls_and_reasoning() {
    let body = json!({"input": [
        {"type": "reasoning", "summary": [{"type": "summary_text", "text": "plan"}]},
        {"type": "function_call", "call_id": "c1", "name": "lookup", "arguments": "{\"q\":1}"},
        {"type": "function_call_output", "call_id": "c1", "output": "tool result"},
        {"type": "local_shell_call", "call_id": "c2", "action": {"command": ["ls"]}},
        {"type": "local_shell_call_output", "call_id": "c2", "output": "shell result"}
    ]});

    assert_eq!(
        collect(ApiFormat::OpenAiResponses, &body),
        [
            ("/input/2/output".to_owned(), "tool result".to_owned()),
            ("/input/4/output".to_owned(), "shell result".to_owned()),
        ]
    );
}

/// 工具返回的结果属于要保护的内容：它是模型探索外部世界拿回来的原文，不是模型的输出。
#[rstest]
fn tool_results_are_still_collected() {
    let chat = json!({"messages": [
        {"role": "tool", "tool_call_id": "c1", "content": "file contents"}
    ]});
    assert_eq!(
        collect(ApiFormat::OpenAiChat, &chat),
        [("/messages/0/content".to_owned(), "file contents".to_owned())]
    );
}

// ---------------------------------------------------------------------------
// Anthropic Messages
// ---------------------------------------------------------------------------

#[rstest]
fn anthropic_collects_system_in_both_shapes() {
    let as_string = json!({"system": "be helpful", "messages": []});
    assert_eq!(
        collect(ApiFormat::AnthropicMessages, &as_string),
        [("/system".to_owned(), "be helpful".to_owned())]
    );

    let as_blocks = json!({
        "system": [{"type": "text", "text": "block system"}],
        "messages": []
    });
    assert_eq!(
        collect(ApiFormat::AnthropicMessages, &as_blocks),
        [("/system/0/text".to_owned(), "block system".to_owned())]
    );
}

#[rstest]
fn anthropic_collects_message_content_blocks() {
    let body = json!({
        "messages": [
            {"role": "user", "content": "plain"},
            {"role": "user", "content": [{"type": "text", "text": "block"}]}
        ]
    });

    assert_eq!(
        collect(ApiFormat::AnthropicMessages, &body),
        [
            ("/messages/0/content".to_owned(), "plain".to_owned()),
            ("/messages/1/content/0/text".to_owned(), "block".to_owned()),
        ]
    );
}

#[rstest]
fn anthropic_collects_tool_results_but_excludes_tool_inputs() {
    let body = json!({
        "messages": [
            {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "t1", "content": "result text"}
            ]},
            {"role": "assistant", "content": [
                {"type": "tool_use", "id": "t2", "name": "send", "input": {
                    "to": "alice@corp.com",
                    "headers": {"x-note": "bob@corp.com"},
                    "count": 3,
                    "flag": true
                }}
            ]}
        ]
    });

    let fields = collect(ApiFormat::AnthropicMessages, &body);

    assert_eq!(
        fields,
        [(
            "/messages/0/content/0/content".to_owned(),
            "result text".to_owned()
        ),]
    );
}

#[rstest]
fn anthropic_collects_nested_tool_result_content_blocks() {
    let body = json!({
        "messages": [
            {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "t1", "content": [
                    {"type": "text", "text": "nested block"}
                ]}
            ]}
        ]
    });

    assert_eq!(
        collect(ApiFormat::AnthropicMessages, &body),
        [(
            "/messages/0/content/0/content/0/text".to_owned(),
            "nested block".to_owned()
        )]
    );
}

// ---------------------------------------------------------------------------
// 回写
// ---------------------------------------------------------------------------

#[rstest]
fn rewriting_touches_only_the_collected_fields(
    #[values(
        ApiFormat::OpenAiResponses,
        ApiFormat::OpenAiChat,
        ApiFormat::AnthropicMessages
    )]
    format: ApiFormat,
) {
    let original = match format {
        ApiFormat::OpenAiResponses => json!({
            "model": "gpt-5",
            "temperature": 0.3,
            "instructions": "secret instructions",
            "input": "secret input",
            "metadata": {"trace": "keep-me"}
        }),
        ApiFormat::OpenAiChat => json!({
            "model": "gpt-5",
            "max_tokens": 128,
            "messages": [
                {"role": "system", "content": "secret system"},
                {"role": "user", "content": "secret user"},
                {"role": "assistant", "tool_calls": [
                    {"id": "c1", "type": "function", "function": {"name": "f", "arguments": "secret args"}}
                ]}
            ]
        }),
        ApiFormat::AnthropicMessages => json!({
            "model": "claude",
            "max_tokens": 128,
            "system": "secret system",
            "messages": [{"role": "user", "content": "secret user"}],
            "metadata": {"user_id": "keep-me"}
        }),
    };

    let fields = adapter(format).collect(&original).expect("可收集");
    assert!(!fields.is_empty(), "{format:?} 应当收集到内容字段");

    let mut body = original.clone();
    let replacements: Vec<(String, String)> = fields
        .iter()
        .map(|field| (field.pointer.clone(), format!("<{}>", field.text)))
        .collect();
    apply(&mut body, &replacements).expect("可回写");

    // 收集到的位置确实被换掉了。
    for (pointer, _) in &replacements {
        assert_ne!(
            body.pointer(pointer),
            original.pointer(pointer),
            "{pointer} 应当被改写"
        );
    }

    // 其余字段逐字节保持原样。
    let mut expected = original.clone();
    let mut actual = body.clone();
    for (pointer, _) in &replacements {
        *expected.pointer_mut(pointer).expect("存在") = Value::Null;
        *actual.pointer_mut(pointer).expect("存在") = Value::Null;
    }
    assert_eq!(actual, expected, "{format:?} 只允许改动内容字段");
}

#[rstest]
fn rewriting_a_field_that_vanished_reports_an_error() {
    let mut body = json!({"messages": [{"role": "user", "content": "text"}]});
    body["messages"] = json!([]);

    let error = apply(
        &mut body,
        &[("/messages/0/content".to_owned(), "x".to_owned())],
    );
    assert!(error.is_err(), "定位失效必须报错而不是静默跳过");
}

#[rstest]
fn collected_pointers_are_usable_as_json_pointer_lookups(
    #[values(
        ApiFormat::OpenAiResponses,
        ApiFormat::OpenAiChat,
        ApiFormat::AnthropicMessages
    )]
    format: ApiFormat,
) {
    let body = match format {
        ApiFormat::OpenAiResponses => json!({"input": "text"}),
        ApiFormat::OpenAiChat => json!({"messages": [{"content": "text"}]}),
        ApiFormat::AnthropicMessages => json!({"messages": [{"content": [{"text": "text"}]}]}),
    };

    for field in collect(format, &body) {
        assert_eq!(
            body.pointer(&field.0),
            Some(&Value::String(field.1)),
            "{format:?} 的指针必须能定位回原值"
        );
    }
}

#[rstest]
fn unrelated_custom_tools_do_not_have_their_json_outputs_reinterpreted() {
    let text = json!({"chunk_id":"c", "wall_time_seconds":1, "output":"body"}).to_string();
    let body = json!({"input":[
        {"type":"custom_tool_call", "name":"other", "call_id":"c1", "input":"ignored"},
        {"type":"custom_tool_call_output", "call_id":"c1", "output":text}
    ]});
    assert_eq!(
        collect(ApiFormat::OpenAiResponses, &body),
        [("/input/1/output".to_owned(), text)]
    );
}

#[rstest]
fn keys_outside_the_known_content_fields_are_not_collected() {
    let body = json!({"system": [{"a~b": "text"}]});
    assert!(collect(ApiFormat::AnthropicMessages, &body).is_empty());
}
