//! 内嵌的控制台静态资源。
//!
//! 前端产物在编译期嵌入二进制，因此运行时不需要任何静态目录。wasm 需要跨源隔离才能启用
//! 多线程：缺少下面两个响应头时 skwasm 会静默退化成单线程并打印警告。

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../apps/console/build/web"]
struct Console;

/// 未命中文件时回落到单页应用入口。
const ENTRY: &str = "index.html";

/// 跨源隔离所需的响应头。Flutter 的 skwasm 加载器会探测 `crossOriginIsolated`。
const ISOLATION_HEADERS: [(&str, &str); 2] = [
    ("cross-origin-opener-policy", "same-origin"),
    ("cross-origin-embedder-policy", "credentialless"),
];

/// 静态资源处理：命中即返回文件，未命中回落到入口页。
///
/// 只对读取类方法回落。未知的写入路径是调用方搞错了端点，回一份 HTML 会掩盖问题——
/// 那看起来像成功，实际什么都没发生。
pub async fn serve(method: axum::http::Method, uri: axum::http::Uri) -> Response {
    if !matches!(method, axum::http::Method::GET | axum::http::Method::HEAD) {
        return (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": {
                    "kind": "not_found",
                    "message": format!("no such endpoint: {}", uri.path()),
                }
            })),
        )
            .into_response();
    }

    let requested = uri.path().trim_start_matches('/');
    let requested = if requested.is_empty() {
        ENTRY
    } else {
        requested
    };

    // 未命中时回落到入口页。响应的内容类型必须按**实际提供的那个文件**判定：
    // 按请求路径判定会让深链接（例如 /pool）拿到 octet-stream，浏览器会把它当成下载。
    let (name, asset) = match Console::get(requested) {
        Some(asset) => (requested, asset),
        None => match Console::get(ENTRY) {
            Some(asset) => (ENTRY, asset),
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    "console assets are not embedded in this build",
                )
                    .into_response();
            }
        },
    };

    asset_response(name, asset.data.into_owned())
}

fn asset_response(path: &str, bytes: Vec<u8>) -> Response {
    let mut response = bytes.into_response();
    if let Some(content_type) = content_type(path) {
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static(content_type),
        );
    }
    for (name, value) in ISOLATION_HEADERS {
        response.headers_mut().insert(
            header::HeaderName::from_static(name),
            header::HeaderValue::from_static(value),
        );
    }
    // 入口页不缓存，避免升级后浏览器继续加载旧版本的前端。
    if path == ENTRY {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-cache"),
        );
    }
    response
}

/// 显式的内容类型表。
///
/// 不按扩展名猜测：`mime_guess` 对 `.wasm` 之类的类型在不同版本上并不一致，而错误的
/// `Content-Type` 会让 `WebAssembly.instantiateStreaming` 直接拒绝加载。
fn content_type(path: &str) -> Option<&'static str> {
    let extension = path.rsplit('.').next()?;
    Some(match extension {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "wasm" => "application/wasm",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "txt" => "text/plain; charset=utf-8",
        _ => return None,
    })
}
