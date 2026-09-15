//! 端到端：真实 HTTP 请求经过代理，落到假上游。核心断言是「上游收到的报文里没有原文」，
//! 以及失败路径下「上游一个请求都没收到」。

mod common;

use common::{
    FailingClassifier, fake_upstream, harness, leaky, leaky_classifier, post, provider_at,
};
use privacy_store::ApiFormat;
use rstest::rstest;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

#[rstest]
#[tokio::test]
async fn the_upstream_never_receives_the_original_content() {
    let (addr, received) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;

    let (status, _) = post(&harness.app, "/v1/chat/completions", leaky()).await;
    assert!(status.is_success(), "请求应当成功：{status}");

    let bodies = received.bodies();
    assert_eq!(bodies.len(), 1, "上游应当恰好收到一次请求");

    let forwarded = bodies[0].to_string();
    assert!(
        !forwarded.contains("AlexExample"),
        "上游收到了原始姓名：{forwarded}"
    );
    assert!(
        !forwarded.contains("alex@example.com"),
        "上游收到了原始邮箱：{forwarded}"
    );
    assert_eq!(
        bodies[0].pointer("/messages/0/content"),
        Some(&json!(
            "im <redacted_private_person> and my email is <redacted_private_email>"
        ))
    );
    // 与内容无关的字段原样上行。
    assert_eq!(bodies[0].pointer("/model"), Some(&json!("gpt-5")));
}

#[rstest]
#[case(ApiFormat::OpenAiChat, "/v1/chat/completions", json!({"messages": [{"content": "mail alex@example.com"}]}))]
#[case(ApiFormat::OpenAiResponses, "/v1/responses", json!({"input": "mail alex@example.com"}))]
#[case(ApiFormat::AnthropicMessages, "/v1/messages", json!({"messages": [{"content": "mail alex@example.com"}]}))]
#[tokio::test]
async fn every_endpoint_redacts_before_forwarding(
    #[case] format: ApiFormat,
    #[case] path: &str,
    #[case] body: Value,
) {
    let (addr, received) = fake_upstream().await;
    let mut provider = provider_at(addr);
    provider.api_format = format;
    let harness = harness(leaky_classifier(), Some(provider)).await;

    let (status, _) = post(&harness.app, path, body).await;
    assert!(status.is_success(), "{path} 应当成功：{status}");

    let bodies = received.bodies();
    assert_eq!(bodies.len(), 1);
    assert!(
        !bodies[0].to_string().contains("alex@example.com"),
        "{path} 泄漏了原文：{}",
        bodies[0]
    );
}

#[rstest]
#[tokio::test]
async fn inference_failure_sends_nothing_upstream() {
    let (addr, received) = fake_upstream().await;
    let harness = harness(Arc::new(FailingClassifier), Some(provider_at(addr))).await;

    let (status, _) = post(&harness.app, "/v1/chat/completions", leaky()).await;

    assert!(!status.is_success(), "识别失败必须拒绝请求");
    // 关键不变量：失败时上游一个请求都没收到，因此不可能退化成原样透传。
    assert_eq!(received.count(), 0, "失败路径不得联系上游");
}

#[rstest]
#[tokio::test]
async fn a_malformed_body_is_rejected_without_contacting_the_upstream() {
    let (addr, received) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .body(axum::body::Body::from("{ not json"))
        .expect("可构造请求");
    let response = harness.app.clone().oneshot(request).await.expect("有响应");

    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(received.count(), 0, "无法解析的报文不得转发");
}

#[rstest]
#[tokio::test]
async fn a_request_without_a_provider_is_refused_without_forwarding() {
    let (_, received) = fake_upstream().await;
    // 没有配置任何上游。
    let harness = harness(leaky_classifier(), None).await;

    let (status, _) = post(&harness.app, "/v1/chat/completions", leaky()).await;

    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(received.count(), 0);
}

#[rstest]
#[tokio::test]
async fn a_disabled_provider_is_not_used() {
    let (addr, received) = fake_upstream().await;
    let mut provider = provider_at(addr);
    provider.enabled = false;
    let harness = harness(leaky_classifier(), Some(provider)).await;

    let (status, _) = post(&harness.app, "/v1/chat/completions", leaky()).await;

    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(received.count(), 0);
}

#[rstest]
#[tokio::test]
async fn an_explicit_provider_header_selects_the_upstream() {
    let (addr, received) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .header("x-privacy-router-provider", "fake")
        .body(axum::body::Body::from(leaky().to_string()))
        .expect("可构造请求");
    let response = harness.app.clone().oneshot(request).await.expect("有响应");

    assert!(response.status().is_success());
    assert_eq!(received.count(), 1);
    assert!(
        !received.bodies()[0]
            .to_string()
            .contains("alex@example.com")
    );
}

#[rstest]
#[tokio::test]
async fn a_provider_speaking_a_different_protocol_is_refused() {
    let (addr, received) = fake_upstream().await;
    // 上游声明的是 Anthropic 协议，但请求走的是 OpenAI Chat 端点。
    let mut provider = provider_at(addr);
    provider.api_format = ApiFormat::AnthropicMessages;
    let harness = harness(leaky_classifier(), Some(provider)).await;

    // 显式指定该上游，此时协议不一致必须被拒绝，而不是按错误的形状发出去。
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("content-type", "application/json")
        .header("x-privacy-router-provider", "fake")
        .body(axum::body::Body::from(leaky().to_string()))
        .expect("可构造请求");
    let response = harness.app.clone().oneshot(request).await.expect("有响应");

    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(received.count(), 0, "协议不一致不得转发");
}

#[rstest]
#[tokio::test]
async fn the_console_api_requires_a_session() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;

    for path in ["/api/stats", "/api/rules", "/api/content", "/api/providers"] {
        let request = axum::http::Request::builder()
            .method("GET")
            .uri(path)
            .body(axum::body::Body::empty())
            .expect("可构造请求");
        let response = harness.app.clone().oneshot(request).await.expect("有响应");
        assert_eq!(
            response.status(),
            axum::http::StatusCode::UNAUTHORIZED,
            "{path} 必须要求会话"
        );
    }
}

#[rstest]
#[tokio::test]
async fn health_is_reachable_without_a_session() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;

    let request = axum::http::Request::builder()
        .method("GET")
        .uri("/api/health")
        .body(axum::body::Body::empty())
        .expect("可构造请求");
    let response = harness.app.clone().oneshot(request).await.expect("有响应");

    assert!(response.status().is_success());
}

#[rstest]
#[tokio::test]
async fn console_assets_are_served_with_cross_origin_isolation() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;

    let request = axum::http::Request::builder()
        .method("GET")
        .uri("/")
        .body(axum::body::Body::empty())
        .expect("可构造请求");
    let response = harness.app.clone().oneshot(request).await.expect("有响应");

    assert!(response.status().is_success());
    // 缺少这两个头时 skwasm 会静默退化成单线程。
    assert_eq!(
        response
            .headers()
            .get("cross-origin-opener-policy")
            .and_then(|value| value.to_str().ok()),
        Some("same-origin")
    );
    assert_eq!(
        response
            .headers()
            .get("cross-origin-embedder-policy")
            .and_then(|value| value.to_str().ok()),
        Some("credentialless")
    );
}

#[rstest]
#[tokio::test]
async fn a_deep_link_falls_back_to_the_console_entry_with_html() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;

    for path in ["/pool", "/rules", "/pool/fragment-42"] {
        let request = axum::http::Request::builder()
            .method("GET")
            .uri(path)
            .body(axum::body::Body::empty())
            .expect("可构造请求");
        let response = harness.app.clone().oneshot(request).await.expect("有响应");

        assert!(response.status().is_success(), "{path} 应当回落到入口页");
        // 内容类型必须按实际提供的入口页判定，否则浏览器会把深链接当成下载。
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("text/html; charset=utf-8"),
            "{path} 的内容类型不对"
        );
    }
}

#[rstest]
#[tokio::test]
async fn an_unknown_api_path_is_forwarded_without_redaction() {
    let (addr, received) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;

    let (status, _) = post(&harness.app, "/v1/unknown", leaky()).await;

    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(received.bodies(), vec![leaky()]);
}
