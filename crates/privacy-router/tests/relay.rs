mod common;

use axum::{
    body::Body,
    http::{HeaderMap, Method, Request, StatusCode, Uri},
};
use common::{FailingClassifier, harness, leaky_classifier, provider_at};
use http_body_util::BodyExt;
use rstest::rstest;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

struct InspectingUpstream;

impl InspectingUpstream {
    async fn start() -> std::net::SocketAddr {
        let app = axum::Router::new().fallback(
            |method: Method, uri: Uri, headers: HeaderMap, body: axum::body::Bytes| async move {
                let response_headers = [
                    ("x-request-id", "upstream-request"),
                    ("retry-after", "12"),
                    ("connection", "x-upstream-hop"),
                    ("x-upstream-hop", "remove-me"),
                ];
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    response_headers,
                    axum::Json(json!({
                        "method": method.as_str(), "uri": uri.to_string(), "body": body.to_vec(),
                        "authorization": headers.get("authorization").and_then(|h| h.to_str().ok()),
                        "api_key": headers.get("x-api-key").and_then(|h| h.to_str().ok()),
                        "beta": headers.get("anthropic-beta").and_then(|h| h.to_str().ok()),
                        "hop": headers.contains_key("x-client-hop"),
                        "selector": headers.contains_key("x-privacy-router-provider"),
                    })),
                )
            },
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        address
    }
}

#[rstest]
#[case("GET", "/v1/models?after=a%2Fb&limit=2", b"".as_slice())]
#[case("POST", "/v1/files?purpose=test", b"\x00\xffopaque upload".as_slice())]
#[case("DELETE", "/v1/responses/resp_123", b"".as_slice())]
#[case("POST", "/v1/unknown?x=1&x=2", b"{ invalid JSON is not ours to parse".as_slice())]
#[tokio::test]
async fn non_redaction_endpoints_preserve_requests_credentials_and_upstream_errors(
    #[case] method: &str,
    #[case] path: &str,
    #[case] body: &[u8],
) {
    let address = InspectingUpstream::start().await;
    let app = harness(Arc::new(FailingClassifier), Some(provider_at(address))).await;
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", "Bearer client-test-token")
        .header("x-api-key", "client-test-key")
        .header("anthropic-beta", "client-feature")
        .header("x-privacy-router-provider", "fake")
        .header("connection", "x-client-hop")
        .header("x-client-hop", "remove-me")
        .body(Body::from(body.to_vec()))
        .unwrap();
    let response = app.app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response.headers()["retry-after"], "12");
    assert_eq!(response.headers()["x-request-id"], "upstream-request");
    assert!(!response.headers().contains_key("x-upstream-hop"));
    let received: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(received["method"], method);
    assert_eq!(received["uri"], path);
    assert_eq!(received["body"], json!(body));
    assert_eq!(received["authorization"], "Bearer client-test-token");
    assert_eq!(received["api_key"], "client-test-key");
    assert_eq!(received["beta"], "client-feature");
    assert_eq!(received["hop"], false);
    assert_eq!(received["selector"], false);
}

#[rstest]
#[case("/v1/chat/completions?trace=a%2Fb", json!({"messages": [{"content": "mail alex@example.com"}]}), privacy_store::ApiFormat::OpenAiChat)]
#[case("/v1/responses?trace=a%2Fb", json!({"input": [{"type":"function_call_output", "call_id":"c1", "output":"alex@example.com"}]}), privacy_store::ApiFormat::OpenAiResponses)]
#[tokio::test]
async fn redaction_preserves_credentials_query_and_function_arguments_structure(
    #[case] path: &str,
    #[case] body: Value,
    #[case] format: privacy_store::ApiFormat,
) {
    let address = InspectingUpstream::start().await;
    let mut provider = provider_at(address);
    provider.api_format = format;
    let app = harness(leaky_classifier(), Some(provider)).await;
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header("authorization", "Bearer client-test-token")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = app.app.oneshot(request).await.unwrap();
    let received: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(received["uri"], path);
    assert_eq!(received["authorization"], "Bearer client-test-token");
    let bytes: Vec<u8> = serde_json::from_value(received["body"].clone()).unwrap();
    let forwarded = String::from_utf8(bytes).unwrap();
    assert!(!forwarded.contains("alex@example.com"));
    assert!(forwarded.contains("<redacted_private_email>"));
}

#[rstest]
#[tokio::test]
async fn a_request_without_redactions_retains_its_exact_json_bytes() {
    let address = InspectingUpstream::start().await;
    let app = harness(leaky_classifier(), Some(provider_at(address))).await;
    let body = "{ \"messages\" : [ { \"content\" : \"hello\" } ] }\n";
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .body(Body::from(body))
        .unwrap();
    let response = app.app.oneshot(request).await.unwrap();
    let received: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(received["body"], json!(body.as_bytes()));
}

#[rstest]
#[tokio::test]
async fn sse_reaches_the_client_before_the_upstream_finishes() {
    let (sender, receiver) = tokio::sync::mpsc::channel::<Result<bytes::Bytes, std::io::Error>>(2);
    sender
        .send(Ok(bytes::Bytes::from_static(b"data: first\n\n")))
        .await
        .unwrap();
    let receiver = Arc::new(tokio::sync::Mutex::new(Some(receiver)));
    let upstream = axum::Router::new().fallback(move || {
        let receiver = receiver.clone();
        async move {
            let stream =
                tokio_stream::wrappers::ReceiverStream::new(receiver.lock().await.take().unwrap());
            (
                [("content-type", "text/event-stream")],
                Body::from_stream(stream),
            )
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, upstream).await.unwrap();
    });
    let app = harness(leaky_classifier(), Some(provider_at(address))).await;
    let request = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .body(Body::from(
            r#"{"messages":[{"content":"hello"}],"stream":true}"#,
        ))
        .unwrap();
    let response = app.app.oneshot(request).await.unwrap();
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    let mut body = response.into_body();
    let first = tokio::time::timeout(std::time::Duration::from_secs(1), body.frame())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .into_data()
        .unwrap();
    assert_eq!(first.as_ref(), b"data: first\n\n");
    sender
        .send(Ok(bytes::Bytes::from_static(b"data: [DONE]\n\n")))
        .await
        .unwrap();
    drop(sender);
    assert_eq!(
        body.collect().await.unwrap().to_bytes().as_ref(),
        b"data: [DONE]\n\n"
    );
}
