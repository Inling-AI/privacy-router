//! 上游透明转发。凭据由客户端提供，响应（包括 SSE）保持流式。

use crate::state::AppState;
use axum::body::Body;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use privacy_store::Provider;
use std::time::Instant;

pub struct Forwarded {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Body,
    pub elapsed_ms: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum ForwardError {
    #[error("upstream request failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("upstream url is invalid: {0}")]
    InvalidUrl(String),
}

impl Forwarded {
    /// base_url 是 SDK 风格的 API 根地址（例如 https://host/v1）。
    /// 入站 /v1 是本代理的挂载前缀，去掉后将剩余路径和查询串追加到该根地址。
    pub async fn send(
        state: &AppState,
        provider: &Provider,
        method: Method,
        uri: &Uri,
        incoming_headers: &HeaderMap,
        body: reqwest::Body,
    ) -> Result<Self, ForwardError> {
        let suffix = uri
            .path()
            .strip_prefix("/v1")
            .ok_or_else(|| ForwardError::InvalidUrl("request is outside the API mount".into()))?;
        let target = format!("{}{}", provider.base_url.trim_end_matches('/'), suffix);
        let mut url = reqwest::Url::parse(&target)
            .map_err(|error| ForwardError::InvalidUrl(error.to_string()))?;
        url.set_query(uri.query());
        let mut headers = RelayHeaders(incoming_headers).end_to_end();
        headers.remove("host");
        headers.remove("content-length");
        headers.remove("x-privacy-router-provider");

        let started = Instant::now();
        let response = state
            .http
            .request(method, url)
            .headers(headers)
            .body(body)
            .send()
            .await?;
        Ok(Self {
            status: response.status(),
            headers: RelayHeaders(response.headers()).end_to_end(),
            elapsed_ms: started.elapsed().as_millis() as i64,
            body: Body::from_stream(response.bytes_stream()),
        })
    }

    pub fn into_response(self) -> axum::response::Response {
        let mut response = axum::response::Response::new(self.body);
        *response.status_mut() = self.status;
        *response.headers_mut() = self.headers;
        response
    }
}

/// 只移除 HTTP 逐跳头，包含 Connection 显式指定的扩展头。
struct RelayHeaders<'a>(&'a HeaderMap);

impl RelayHeaders<'_> {
    fn end_to_end(&self) -> HeaderMap {
        let mut headers = self.0.clone();
        for connection in self.0.get_all("connection") {
            if let Ok(connection) = connection.to_str() {
                for name in connection.split(',').map(str::trim) {
                    headers.remove(name);
                }
            }
        }
        for name in [
            "connection",
            "keep-alive",
            "proxy-authenticate",
            "proxy-authorization",
            "te",
            "trailer",
            "transfer-encoding",
            "upgrade",
        ] {
            headers.remove(name);
        }
        headers
    }
}

/// 仅这些接口的 POST 请求需要解析和脱敏，其余 /v1/* 请求直接透传。
pub fn format_for_path(path: &str) -> Option<privacy_store::ApiFormat> {
    match path {
        "/v1/responses" => Some(privacy_store::ApiFormat::OpenAiResponses),
        "/v1/chat/completions" => Some(privacy_store::ApiFormat::OpenAiChat),
        "/v1/messages" => Some(privacy_store::ApiFormat::AnthropicMessages),
        _ => None,
    }
}
