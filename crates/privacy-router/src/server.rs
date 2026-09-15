//! 路由装配与代理入口。

use crate::config::ServerArgs;
use crate::console;
use crate::error::ApiError;
use crate::pipeline::{RequestContext, process};
use crate::proxy;
use crate::state::AppState;
use crate::web;
use axum::Router;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, delete, get, patch, post};
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::{AllowHeaders, AllowMethods, CorsLayer};
use tower_http::trace::TraceLayer;

/// 组装完整应用。
pub fn router(state: Arc<AppState>) -> Router {
    // 默认受保护：新增端点若忘记加鉴权，会落在这里而不是暴露出去。
    let protected = Router::new()
        .route("/session", delete(console::logout))
        .route("/stats", get(console::stats))
        .route("/content", get(console::content))
        .route("/spans/{id}/occurrences", get(console::span_occurrences))
        .route("/spans/{id}/fragment", get(console::span_fragment))
        .route("/spans/{id}/release", post(console::release_span))
        .route("/spans/{id}/redact", post(console::redact_span))
        .route("/rules", get(console::rules).post(console::create_rule))
        .route("/rule-policy", patch(console::RulePolicy::update))
        .route(
            "/rules/{id}",
            patch(console::update_rule).delete(console::delete_rule),
        )
        .route("/rules/{id}/enabled", post(console::set_rule_enabled))
        .route(
            "/providers",
            get(console::providers).post(console::create_provider),
        )
        .route(
            "/providers/{id}",
            patch(console::update_provider).delete(console::delete_provider),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_session,
        ));

    // 只有登录、首次初始化与健康检查是公开的，显式列出。
    let public = Router::new()
        .route("/session", post(console::login))
        .route("/setup", post(console::setup))
        .route("/health", get(console::health));

    let mut console_api = Router::new().nest("/api", public.merge(protected));
    if let Some(cors) = dev_cors(state.args.as_ref()) {
        console_api = console_api.layer(cors);
    }

    console_api
        .fallback(web::serve)
        .route("/v1", any(handle_proxy))
        .route("/v1/{*path}", any(handle_proxy))
        .layer(DefaultBodyLimit::max(state.args.max_body_bytes))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// 开发期跨源许可。只挂在控制台 API 上，且必须显式配置来源才存在。
///
/// 生产形态下控制台由本进程同源提供，因此这里默认返回 `None`——不配置 `--dev-origin`
/// 时路由器与生产形态完全一致，`/v1/*` 更不会获得任何跨源许可。
fn dev_cors(args: &ServerArgs) -> Option<CorsLayer> {
    if args.dev_origin.is_empty() {
        return None;
    }
    let origins = args
        .dev_origin
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .collect::<Vec<_>>();
    tracing::warn!(
        origins = ?args.dev_origin,
        "开发来源已启用：控制台 API 接受来自这些来源的跨源请求"
    );
    Some(
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(AllowMethods::mirror_request())
            .allow_headers(AllowHeaders::mirror_request()),
    )
}

/// 控制台鉴权中间件。放在路由层而不是各处理器里，避免新端点漏掉校验。
async fn require_session(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    console::authorize(&state, request.headers()).await?;
    Ok(next.run(request).await)
}

/// 代理入口：定位内容 → 识别 → 判定 → 替换 → 记录 → 转发。
///
/// 任何一个前置步骤失败都会直接返回错误并**不发起上游请求**。
async fn handle_proxy(State(state): State<Arc<AppState>>, request: Request) -> Response {
    let started = Instant::now();
    let request_id = uuid::Uuid::new_v4().to_string();
    match proxy_request(&state, request, &request_id, started).await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(
                request_id = %request_id,
                stage = "reject",
                error_kind = error.kind,
                "request rejected without contacting the upstream"
            );
            error.into_response()
        }
    }
}

async fn proxy_request(
    state: &Arc<AppState>,
    request: Request,
    request_id: &str,
    started: Instant,
) -> Result<Response, ApiError> {
    let path = request.uri().path().to_owned();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    let method = request.method().clone();
    let format = (method == axum::http::Method::POST)
        .then(|| proxy::format_for_path(&path))
        .flatten();
    let provider = console::select_provider(state, format, &headers).await?;
    let Some(format) = format else {
        // 非脱敏接口不缓冲、不解析请求体，模型列表、文件等保持原始字节流。
        return proxy::Forwarded::send(
            state,
            &provider,
            method,
            &uri,
            &headers,
            reqwest::Body::wrap_stream(request.into_body().into_data_stream()),
        )
        .await
        .map(proxy::Forwarded::into_response)
        .map_err(|_| {
            ApiError::new(
                StatusCode::BAD_GATEWAY,
                "upstream_failed",
                "the upstream request could not be completed",
            )
        });
    };
    let body = axum::body::to_bytes(request.into_body(), state.args.max_body_bytes)
        .await
        .map_err(|_| ApiError::bad_request("request body could not be read"))?;

    // 报文必须能解析；解析失败意味着无法定位内容，因此拒绝而不是透传。
    let parsed: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|_| ApiError::bad_request("request body must be a JSON object"))?;

    let context = RequestContext {
        request_id: request_id.to_owned(),
        api_format: format,
        path: path.clone(),
    };

    let classifier = state.classifier.clone();
    let rules = state.rules();
    let parsed_for_task = parsed.clone();
    let context_for_task = context.clone();
    let queue_started = Instant::now();
    let prefix_store = state.store.clone();
    let runtime = tokio::runtime::Handle::current();
    let processed = tokio::task::spawn_blocking(move || {
        let classifier = crate::prefix::PrefixClassifier::new(
            &parsed_for_task,
            context_for_task.api_format,
            classifier,
            prefix_store,
            runtime,
        )?;
        // 结构化类别在识别链的出口处交还给模式，重放与新推理因此得到同一份判定。
        // 确定性过滤器在识别链的出口处接上，重放与新推理因此得到同一份判定。
        let classifier = crate::classifier::FilteredClassifier::new(classifier);
        process(
            &parsed_for_task,
            &context_for_task,
            &classifier,
            rules.as_ref(),
        )
    })
    .await
    .map_err(|_| ApiError::internal("inference task failed"))?
    .map_err(|error| {
        // 识别或判定失败：不转发，并且不把原文放进错误消息。
        tracing::error!(
            request_id = %context.request_id,
            stage = "classify",
            error = %error,
            "processing failed; request will not be forwarded"
        );
        ApiError::unprocessable(
            "processing_failed",
            "request content could not be processed",
        )
    })?;
    let queue_ms = queue_started.elapsed().as_millis() as i64;

    // 先落库再转发：转发失败时审计记录仍然完整。
    let recorded = persist(state, &processed).await;
    if let Err(error) = &recorded {
        tracing::error!(
            request_id = %request_id,
            stage = "persist",
            error = %error,
            "audit write failed; forwarding continues under the decision already made"
        );
    }

    tracing::info!(
        request_id = %request_id,
        provider = %provider.name,
        api_format = ?format,
        stage = "decide",
        released = processed.released,
        redacted = processed.redacted,
        cached_fragments = processed.usage.cached_fragments,
        inferred_fragments = processed.usage.inferred_fragments,
        "content decided"
    );

    // 未发生替换时保留请求原始字节，包括 JSON 排版。
    let forwarded_body = if processed.redacted == 0 {
        body
    } else {
        Bytes::from(
            serde_json::to_vec(&processed.body)
                .map_err(|_| ApiError::internal("processed body could not be serialized"))?,
        )
    };

    let forwarded = proxy::Forwarded::send(
        state,
        &provider,
        method,
        &uri,
        &headers,
        reqwest::Body::from(forwarded_body),
    )
    .await
    .map_err(|error| {
        tracing::error!(
            request_id = %request_id,
            stage = "forward",
            error = %error,
            "upstream request failed"
        );
        ApiError::new(
            StatusCode::BAD_GATEWAY,
            "upstream_failed",
            "the upstream request could not be completed",
        )
    })?;

    let total_ms = started.elapsed().as_millis() as i64;
    record_outcome(
        state,
        &context,
        &provider,
        forwarded.status.as_u16() as i64,
        total_ms,
        queue_ms,
        forwarded.elapsed_ms,
        &processed,
        recorded.err(),
    );

    Ok(forwarded.into_response())
}

/// 写入内容池。片段逐条提交，单条失败不会丢弃其余记录。
async fn persist(
    state: &Arc<AppState>,
    processed: &crate::pipeline::Processed,
) -> Result<(), privacy_store::Error> {
    for fragment in &processed.fragments {
        state.store.pool_records().record(fragment).await?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn record_outcome(
    state: &Arc<AppState>,
    context: &RequestContext,
    provider: &privacy_store::Provider,
    status: i64,
    total_ms: i64,
    queue_ms: i64,
    upstream_ms: i64,
    processed: &crate::pipeline::Processed,
    persist_error: Option<privacy_store::Error>,
) {
    let outcome = privacy_store::RequestOutcome {
        request_id: context.request_id.clone(),
        provider_id: Some(provider.id.clone()),
        api_format: context.api_format,
        status,
        timing: privacy_store::RequestTiming {
            queue_ms,
            inference_ms: processed.usage.inference_ms,
            upstream_ms,
            duration_ms: total_ms,
        },
        cache_usage_json: serde_json::json!({
            "cached_fragments": processed.usage.cached_fragments,
            "deduplicated_fragments": processed.usage.deduplicated_fragments,
            "inferred_fragments": processed.usage.inferred_fragments,
        })
        .to_string(),
        error_kind: persist_error.map(|error| format!("{error:?}")),
    };

    // 统计写入失败只影响可观测性，不影响已经作出的判定；同步等待会拖慢响应路径。
    let store = state.store.clone();
    let performance = processed.usage.performance;
    tokio::spawn(async move {
        if let Err(error) = store
            .metrics()
            .record_with_performance(&outcome, &performance)
            .await
        {
            tracing::error!(stage = "metrics", error = %error, "request metrics were not recorded");
        }
    });
}

/// 供测试与运维使用的就绪检查路径。
pub const HEALTH_PATH: &str = "/api/health";

/// 代理端点的路径集合。协议由路径决定，因此这里与 [`proxy::format_for_path`] 必须一致。
pub const PROXY_PATHS: [&str; 3] = ["/v1/responses", "/v1/chat/completions", "/v1/messages"];
