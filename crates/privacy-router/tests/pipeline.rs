//! 管线端到端：报文进、脱敏后的报文出，同时留下可审计的记录。

mod common;

use common::{FailingClassifier, FakeClassifier, MismatchedClassifier, release_keyword};
use privacy_router::pipeline::{RequestContext, process};
use privacy_rules::{EntityGroup, Rule, RuleId, RuleSet};
use privacy_store::ApiFormat;
use rstest::rstest;
use serde_json::{Value, json};

fn context(format: ApiFormat) -> RequestContext {
    RequestContext {
        request_id: "req-1".to_owned(),
        api_format: format,
        path: "/v1/chat/completions".to_owned(),
    }
}

fn leaky_text() -> &'static str {
    "im AlexExample and my email is alex@example.com"
}

fn classifier() -> FakeClassifier {
    FakeClassifier::new(vec![
        ("AlexExample", EntityGroup::PrivatePerson, 0.99),
        ("alex@example.com", EntityGroup::PrivateEmail, 0.98),
    ])
}

#[rstest]
fn user_content_is_redacted_before_it_can_be_forwarded() {
    let body = json!({
        "model": "gpt-5",
        "temperature": 0.2,
        "messages": [{"role": "user", "content": leaky_text()}]
    });

    let processed = process(
        &body,
        &context(ApiFormat::OpenAiChat),
        &classifier(),
        &RuleSet::builtin(),
    )
    .expect("处理成功");

    let forwarded = processed.body.to_string();
    assert!(
        !forwarded.contains("AlexExample"),
        "上游报文不得含原始姓名：{forwarded}"
    );
    assert!(
        !forwarded.contains("alex@example.com"),
        "上游报文不得含原始邮箱：{forwarded}"
    );
    assert_eq!(
        processed.body.pointer("/messages/0/content"),
        Some(&json!(
            "im <redacted_private_person> and my email is <redacted_private_email>"
        ))
    );
    // 与内容无关的字段逐字节保留。
    assert_eq!(processed.body.pointer("/temperature"), Some(&json!(0.2)));
    assert_eq!(processed.body.pointer("/model"), Some(&json!("gpt-5")));
    assert_eq!(processed.redacted, 2);
}

#[rstest]
#[case(ApiFormat::OpenAiResponses, json!({"input": leaky_text()}), "/input")]
#[case(ApiFormat::OpenAiChat, json!({"messages": [{"content": leaky_text()}]}), "/messages/0/content")]
#[case(ApiFormat::AnthropicMessages, json!({"messages": [{"content": leaky_text()}]}), "/messages/0/content")]
fn every_protocol_redacts_user_content(
    #[case] format: ApiFormat,
    #[case] body: Value,
    #[case] pointer: &str,
) {
    let processed =
        process(&body, &context(format), &classifier(), &RuleSet::builtin()).expect("处理成功");

    let rewritten = processed.body.pointer(pointer).and_then(Value::as_str);
    assert!(
        rewritten.is_some_and(|text| text.contains("<redacted_private_person>")
            && text.contains("<redacted_private_email>")),
        "{format:?} 未完成脱敏：{:?}",
        processed.body.pointer(pointer)
    );
}

#[rstest]
fn released_content_is_forwarded_but_still_recorded() {
    let body = json!({"messages": [{"content": "mail user@example.com or alice@corp.com"}]});
    let classifier = FakeClassifier::new(vec![
        ("user@example.com", EntityGroup::PrivateEmail, 0.99),
        ("alice@corp.com", EntityGroup::PrivateEmail, 0.99),
    ]);
    let rules = RuleSet::new(
        RuleSet::builtin()
            .rules()
            .iter()
            .cloned()
            .chain([release_keyword("user@example.com")]),
    );

    let processed =
        process(&body, &context(ApiFormat::OpenAiChat), &classifier, &rules).expect("处理成功");

    // 被批准的内容原样上行，其余照常抹去。
    assert_eq!(
        processed.body.pointer("/messages/0/content"),
        Some(&json!("mail user@example.com or <redacted_private_email>"))
    );
    // 放行同样是一次判定，必须留痕。
    assert_eq!(processed.released, 1);
    assert_eq!(processed.redacted, 1);
    assert_eq!(processed.fragments.len(), 1);
    assert_eq!(processed.fragments[0].spans.len(), 2);
}

#[rstest]
fn content_without_any_finding_is_not_written_to_the_pool() {
    let body = json!({"messages": [{"content": "nothing sensitive at all"}]});
    let processed = process(
        &body,
        &context(ApiFormat::OpenAiChat),
        &FakeClassifier::blind(),
        &RuleSet::builtin(),
    )
    .expect("处理成功");

    assert!(processed.fragments.is_empty(), "无判定的文本不入池");
    assert_eq!(processed.body, body);
}

#[rstest]
fn bodies_without_content_fields_pass_through_untouched() {
    let body = json!({"model": "gpt-5", "max_tokens": 10, "messages": []});
    let processed = process(
        &body,
        &context(ApiFormat::OpenAiChat),
        &classifier(),
        &RuleSet::builtin(),
    )
    .expect("处理成功");

    assert_eq!(processed.body, body);
    assert!(processed.fragments.is_empty());
}

#[rstest]
fn records_keep_both_the_original_and_the_replacement() {
    let body = json!({"input": leaky_text()});
    let processed = process(
        &body,
        &context(ApiFormat::OpenAiResponses),
        &classifier(),
        &RuleSet::builtin(),
    )
    .expect("处理成功");

    let fragment = &processed.fragments[0];
    assert_eq!(fragment.original_text, leaky_text());
    assert_eq!(
        fragment.redacted_text,
        "im <redacted_private_person> and my email is <redacted_private_email>"
    );
    assert_eq!(fragment.spans.len(), 2);
    // 审计里的位置必须指回原文。
    for span in &fragment.spans {
        assert_eq!(
            &fragment.original_text[span.byte_start..span.byte_end],
            span.original_text
        );
    }
    assert_eq!(fragment.request_id, "req-1");
}

#[rstest]
fn inference_failure_returns_no_body_at_all() {
    let body = json!({"messages": [{"content": leaky_text()}]});
    let outcome = process(
        &body,
        &context(ApiFormat::OpenAiChat),
        &FailingClassifier,
        &RuleSet::builtin(),
    );

    // 关键不变量：失败时没有可供转发的报文，因此不可能退化成原样透传。
    assert!(outcome.is_err(), "识别失败必须整体失败");
}

#[rstest]
fn a_result_count_mismatch_is_refused_rather_than_used() {
    let body = json!({
        "messages": [
            {"content": "first"},
            {"content": "second"}
        ]
    });
    let outcome = process(
        &body,
        &context(ApiFormat::OpenAiChat),
        &MismatchedClassifier,
        &RuleSet::builtin(),
    );

    assert!(matches!(
        outcome,
        Err(privacy_router::Error::ClassificationShape {
            fields: 2,
            results: 0
        })
    ));
}

#[rstest]
fn instructions_and_tool_outputs_are_redacted_too() {
    let body = json!({
        "instructions": "you know AlexExample",
        "input": [
            {"type": "function_call_output", "call_id": "c1", "output": "reached alex@example.com"}
        ]
    });

    let processed = process(
        &body,
        &context(ApiFormat::OpenAiResponses),
        &classifier(),
        &RuleSet::builtin(),
    )
    .expect("处理成功");

    assert_eq!(
        processed.body.pointer("/instructions"),
        Some(&json!("you know <redacted_private_person>"))
    );
    assert_eq!(
        processed.body.pointer("/input/0/output"),
        Some(&json!("reached <redacted_private_email>"))
    );
}

#[rstest]
fn responses_function_arguments_are_untouched() {
    let body = json!({"input": [{
        "type": "function_call", "call_id": "c1", "name": "send_mail",
        "arguments": "{\"email\":\"alex@example.com\",\"count\":1}"
    }]});
    let processed = process(
        &body,
        &context(ApiFormat::OpenAiResponses),
        &classifier(),
        &RuleSet::builtin(),
    )
    .unwrap();
    assert_eq!(
        processed.body["input"][0]["arguments"],
        "{\"email\":\"alex@example.com\",\"count\":1}"
    );
    assert_eq!(processed.body["input"][0]["call_id"], "c1");
    assert_eq!(processed.body["input"][0]["name"], "send_mail");
    assert_eq!(processed.redacted, 0);
}

#[rstest]
fn custom_exec_output_redacts_only_body_and_preserves_envelope() {
    let envelope = json!({"chunk_id":"alex@example.com", "wall_time_seconds":0.2,
        "exit_code":0, "original_token_count":12, "output":"mail alex@example.com"});
    let body = json!({"input":[
        {"type":"custom_tool_call","name":"exec","call_id":"c1","input":"alex@example.com"},
        {"type":"custom_tool_call_output","call_id":"c1","output":[
            {"type":"input_text","text":"Script completed\nWall time 0.2 seconds\nOutput:\n"},
            {"type":"input_text","text":envelope.to_string()},
            {"type":"input_image","image_url":"data:image/png;base64,abc"}
        ]}
    ]});
    let fields = privacy_router::protocol::adapter(ApiFormat::OpenAiResponses)
        .collect(&body)
        .unwrap();
    assert_eq!(
        fields.iter().map(|f| f.text.as_str()).collect::<Vec<_>>(),
        ["mail alex@example.com"]
    );
    let processed = process(
        &body,
        &context(ApiFormat::OpenAiResponses),
        &classifier(),
        &RuleSet::builtin(),
    )
    .unwrap();
    let mut expected = body.clone();
    let mut expected_envelope = envelope;
    expected_envelope["output"] = json!("mail <redacted_private_email>");
    expected["input"][1]["output"][1]["text"] = json!(expected_envelope.to_string());
    assert_eq!(processed.body, expected);
}

#[rstest]
fn repeated_occurrences_inside_one_field_are_all_redacted() {
    let body = json!({"messages": [{"content": "AlexExample wrote to AlexExample"}]});
    let processed = process(
        &body,
        &context(ApiFormat::OpenAiChat),
        &classifier(),
        &RuleSet::builtin(),
    )
    .expect("处理成功");

    assert_eq!(
        processed.body.pointer("/messages/0/content"),
        Some(&json!(
            "<redacted_private_person> wrote to <redacted_private_person>"
        ))
    );
    assert!(!processed.body.to_string().contains("AlexExample"));
    assert_eq!(processed.fragments[0].spans.len(), 2);
}

#[rstest]
fn the_workload_of_each_request_is_reported() {
    let body = json!({"messages": [{"content": leaky_text()}]});
    let processed = process(
        &body,
        &context(ApiFormat::OpenAiChat),
        &classifier(),
        &RuleSet::builtin(),
    )
    .expect("处理成功");

    // 默认实现不统计缓存，但结构必须齐全，供控制台的性能页使用。
    assert_eq!(processed.usage.cached_fragments, 0);
    assert_eq!(processed.usage.inferred_fragments, 0);
}

/// 模型点名的形状从来不是登记的对象：登记的是那段原文本身。
const KEYED_SECRET: &str = "api_key=DEMO_KEY_NOT_A_REAL_SECRET";

/// 登记的禁止覆盖每一个出现处，包括模型完全没有提议的那一处。
#[rstest]
fn a_registered_text_is_redacted_wherever_it_appears() {
    let secret = KEYED_SECRET.strip_prefix("api_key=").expect("带前缀");
    let text = format!("== cut ==\n{KEYED_SECRET}\n== printf loop ==\n{secret}\n== END ==");
    let body = json!({"messages": [{"content": text}]});
    // 模型只把带前缀的那一处挑出来，裸值那一处它没有提议。
    let classifier = FakeClassifier::new(vec![(KEYED_SECRET, EntityGroup::Secret, 0.989)]);
    let rules =
        RuleSet::new(
            RuleSet::builtin()
                .rules()
                .iter()
                .cloned()
                .chain([Rule::operator_redact(
                    RuleId::new("operator.deny.secret").expect("非空"),
                    secret,
                    EntityGroup::Secret,
                )]),
        );

    let processed =
        process(&body, &context(ApiFormat::OpenAiChat), &classifier, &rules).expect("处理成功");

    assert!(
        !processed.body.to_string().contains(secret),
        "登记过的原文不得以任何形式上行：{}",
        processed.body
    );
    let fragment = &processed.fragments[0];
    assert_eq!(fragment.original_text.matches(secret).count(), 2);
    assert!(!fragment.redacted_text.contains(secret));
}

/// 模型把整段判成密钥，但其中那段原文被登记为放行：以登记为准。
#[rstest]
fn a_released_registration_overrides_the_models_redaction() {
    let text = "smtp_password=j575v%9g0z%5AL";
    let body = json!({"messages": [{"content": text}]});
    let classifier = FakeClassifier::new(vec![(text, EntityGroup::Secret, 0.999)]);
    let rules =
        RuleSet::new(
            RuleSet::builtin()
                .rules()
                .iter()
                .cloned()
                .chain([Rule::operator_release(
                    RuleId::new("operator.allow.smtp").expect("非空"),
                    "j575v%9g0z%5AL",
                    EntityGroup::Secret,
                )]),
        );

    let processed =
        process(&body, &context(ApiFormat::OpenAiChat), &classifier, &rules).expect("处理成功");

    assert_eq!(
        processed.body.pointer("/messages/0/content"),
        Some(&json!(text))
    );
    assert_eq!(processed.redacted, 0);
    assert_eq!(processed.released, 1);
}
