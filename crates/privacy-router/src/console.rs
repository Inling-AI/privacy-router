//! 控制台 API。
//!
//! 全部端点都在会话保护之下，只有 `POST /api/session`（登录）、`POST /api/setup`
//! （首次自助初始化）与 `GET /api/health` 例外。
//! 写操作完成后立即重新装配规则集合，因此「在网页上放行一条内容」对后续请求立刻生效。

use crate::error::ApiError;
use crate::state::AppState;
use axum::Json;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use privacy_rules::{Action, Rule, RuleExpression, RuleId, RuleSource};
use privacy_store::{Credentials, Provider, ProviderDraft, Statistics};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// 会话
// ---------------------------------------------------------------------------

/// 登录与首次初始化提交的是同一组凭据，因此只有一个请求体类型。
#[derive(Debug, Deserialize)]
pub struct CredentialsRequest {
    pub username: String,
    pub password: String,
}

/// 刚签发的会话。明文令牌只在这一刻返回。
#[derive(Debug, Serialize)]
pub struct IssuedSession {
    pub token: String,
    pub expires_at: i64,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CredentialsRequest>,
) -> Result<Json<IssuedSession>, ApiError> {
    let authenticated = state
        .store
        .admin()
        .authenticate(&request.username, &request.password)
        .await?;

    if !authenticated {
        // 不区分「用户不存在」与「口令错误」，避免把账号存在性变成可枚举的信息。
        tracing::warn!(stage = "console", "console login rejected");
        return Err(ApiError::unauthorized());
    }

    let session = state
        .store
        .admin()
        .create_session(state.args.session_ttl_ms())
        .await?;
    tracing::info!(stage = "console", "console session created");
    Ok(Json(IssuedSession {
        token: session.token,
        expires_at: session.expires_at,
    }))
}

/// 首次安装的自助初始化：还没有管理员时创建唯一账号，并直接签发会话。
///
/// 这是唯一一个无认证的**写**入口，因此准入条件由 [`LocalPeer`] 在类型上给出。已经
/// 初始化过之后再调用返回冲突，凭据不会被静默覆盖——重置仍走 `init-admin --force`。
pub async fn setup(
    State(state): State<Arc<AppState>>,
    _: LocalPeer,
    Json(request): Json<CredentialsRequest>,
) -> Result<Json<IssuedSession>, ApiError> {
    let credentials = Credentials::new(request.username, request.password)?;
    state.store.admin().create(&credentials).await?;
    let session = state
        .store
        .admin()
        .create_session(state.args.session_ttl_ms())
        .await?;
    tracing::info!(stage = "console", "console administrator created");
    Ok(Json(IssuedSession {
        token: session.token,
        expires_at: session.expires_at,
    }))
}

/// 只接受来自本机的请求。
///
/// 首次初始化没有任何凭据可以校验，「从哪里来」就是唯一的准入条件；把它做成提取器而
/// 不是在处理器里写判断，签名里出现它就不可能被忘记。
pub struct LocalPeer;

impl<S> FromRequestParts<S> for LocalPeer
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ConnectInfo(peer) = ConnectInfo::<SocketAddr>::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::forbidden(SELF_ONLY))?;
        if !peer.ip().is_loopback() {
            tracing::warn!(stage = "console", peer = %peer, "initial setup rejected");
            return Err(ApiError::forbidden(SELF_ONLY));
        }
        Ok(LocalPeer)
    }
}

/// 拒绝理由对远程与本地一致：不区分「地址不对」与「拿不到地址」。
const SELF_ONLY: &str =
    "the first administrator can only be created on the machine running the proxy";

/// 注销当前会话。会话有效性已由中间件校验，这里只需要取出令牌并删除它。
pub async fn logout(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let token = SessionToken::from_headers(&headers)?;
    state.store.admin().revoke_session(&token.0).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// 从 `Authorization: Bearer <token>` 中取出的会话令牌。
pub struct SessionToken(pub String);

impl SessionToken {
    pub fn from_headers(headers: &HeaderMap) -> Result<Self, ApiError> {
        let value = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or_else(ApiError::unauthorized)?;
        Ok(Self(value.trim().to_owned()))
    }
}

/// 校验会话。除登录与健康检查外的所有控制台端点都要经过它。
pub async fn authorize(state: &AppState, headers: &HeaderMap) -> Result<SessionToken, ApiError> {
    let token = SessionToken::from_headers(headers)?;
    if !state.store.admin().validate_session(&token.0).await? {
        return Err(ApiError::unauthorized());
    }
    Ok(token)
}

// ---------------------------------------------------------------------------
// 健康检查
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct Health {
    pub status: &'static str,
    pub rules: usize,
    pub providers: usize,
    pub admin_configured: bool,
}

pub async fn health(State(state): State<Arc<AppState>>) -> Result<Json<Health>, ApiError> {
    let providers = state.store.providers().list().await?.len();
    let admin_configured = state.store.admin().is_configured().await?;
    Ok(Json(Health {
        status: "ok",
        rules: state.rules().rules().len(),
        providers,
        admin_configured,
    }))
}

// ---------------------------------------------------------------------------
// 统计
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    /// 统计窗口，小时；默认 24。
    #[serde(default)]
    pub hours: Option<i64>,
}

pub async fn stats(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StatsQuery>,
) -> Result<Json<Statistics>, ApiError> {
    let hours = query.hours.unwrap_or(24).clamp(0, 24 * 365);
    let since = state.store.now_ms() - hours.saturating_mul(60 * 60 * 1000);
    Ok(Json(state.store.metrics().summary(since).await?))
}

// ---------------------------------------------------------------------------
// 内容池
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ContentPage {
    pub total: i64,
    pub items: Vec<ContentRow>,
}

/// 内容池的一行：同一段内容的判定统计，加上它当前登记的方向。
///
/// 登记方向由规则集合给出而不是另存一份：登记的唯一来源就是那条规则。
#[derive(Debug, Serialize)]
pub struct ContentRow {
    #[serde(flatten)]
    pub summary: privacy_store::ContentSummary,
    /// 管理员已经为这段内容登记的方向；`None` 表示尚未登记。
    pub registered_action: Option<Action>,
}

#[derive(Debug, Deserialize)]
pub struct ContentQuery {
    #[serde(default)]
    pub filter: privacy_store::ContentFilter,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

/// 按内容聚合的一页。默认只给还没登记过决定的内容；判定是否摇摆只影响排序。
pub async fn content(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ContentQuery>,
) -> Result<Json<ContentPage>, ApiError> {
    let limit = query.limit.unwrap_or(50).clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);
    let records = state.store.pool_records();

    // 「审过没有」问的是规则集合，不问聚合结果：登记的唯一来源就是那些规则。
    let rules = state.rules();
    let registered: Vec<String> = rules.registered_texts().map(str::to_owned).collect();
    let items = records
        .content_page(query.filter, &registered, limit, offset)
        .await?
        .into_iter()
        .map(|summary| {
            let registered_action = rules
                .operator_decision(&summary.original_text)
                .map(|rule| rule.action);
            ContentRow {
                summary,
                registered_action,
            }
        })
        .collect();

    Ok(Json(ContentPage {
        total: records.content_count(query.filter, &registered).await?,
        items,
    }))
}

#[derive(Debug, Serialize)]
pub struct OccurrencePage {
    pub items: Vec<privacy_store::ContentOccurrence>,
}

/// 一条命中所属的 turn：原文、转发出去的内容，以及这次的全部命中。
///
/// 锚点同样是命中标识：片段标识与被审查的文本都不进 URL。
pub async fn span_fragment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<privacy_store::FragmentDetail>, ApiError> {
    state
        .store
        .pool_records()
        .fragment_of_span(&id)
        .await?
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("span {id}")))
}

/// 一段内容的全部出现，最近的在前。
///
/// 锚点用命中标识而不是文本本身，被审查的内容就不会出现在 URL、日志与浏览器历史里。
pub async fn span_occurrences(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<OccurrencePage>, ApiError> {
    let span = state
        .store
        .pool_records()
        .find_span(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("span {id}")))?;
    Ok(Json(OccurrencePage {
        items: state
            .store
            .pool_records()
            .occurrences_of(&span.original_text)
            .await?,
    }))
}

/// 一次登记的结果：登记到哪条规则、这次调用有没有改动它。
#[derive(Debug, Serialize)]
pub struct SpanDecisionOutcome {
    pub rule_id: String,
    /// 为 false 表示这段内容此前已经登记过同一方向，这一次没有改动任何东西。
    pub changed: bool,
}

/// 放行一段内容：登记一条整段匹配的放行规则，后续相同内容直接走放行分支。
pub async fn release_span(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SpanDecisionOutcome>, ApiError> {
    let (rule_id, changed) = state
        .record_span_decision(&id, Action::Release)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("span {id}")))?;
    Ok(Json(SpanDecisionOutcome {
        rule_id: rule_id.to_string(),
        changed,
    }))
}

/// 禁止一段内容：登记一条整段匹配的强制抹去规则，后续相同内容不再上行。
///
/// 改主意时改写的是同一条登记，因此不存在「两个方向同时躺着、靠优先级决出胜负」的局面。
pub async fn redact_span(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SpanDecisionOutcome>, ApiError> {
    let (rule_id, changed) = state
        .record_span_decision(&id, Action::Redact)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("span {id}")))?;
    Ok(Json(SpanDecisionOutcome {
        rule_id: rule_id.to_string(),
        changed,
    }))
}

// ---------------------------------------------------------------------------
// 规则
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct RuleList {
    pub items: Vec<Rule>,
    pub fallback_action: privacy_rules::Action,
}

#[derive(Deserialize)]
pub struct RulePolicy {
    pub fallback_action: privacy_rules::Action,
}

impl RulePolicy {
    pub async fn update(
        State(state): State<Arc<AppState>>,
        Json(policy): Json<Self>,
    ) -> Result<StatusCode, ApiError> {
        state
            .store
            .rules()
            .set_fallback(policy.fallback_action)
            .await?;
        state.reload_rules().await?;
        Ok(StatusCode::NO_CONTENT)
    }
}

pub async fn rules(State(state): State<Arc<AppState>>) -> Result<Json<RuleList>, ApiError> {
    let set = state.rules();
    Ok(Json(RuleList {
        items: set.rules().to_vec(),
        fallback_action: set.fallback(),
    }))
}

/// 新建或修改规则的内容。标识由服务端生成，允许范围由来源决定。
#[derive(Debug, Deserialize)]
pub struct RuleDraft {
    pub name: String,
    pub priority: i32,
    pub condition: RuleExpression,
    pub action: privacy_rules::Action,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

pub async fn create_rule(
    State(state): State<Arc<AppState>>,
    Json(draft): Json<RuleDraft>,
) -> Result<(StatusCode, Json<Rule>), ApiError> {
    let rule = Rule {
        id: RuleId::new(format!("console.{}", uuid::Uuid::new_v4()))?,
        name: draft.name,
        priority: draft.priority,
        condition: draft.condition,
        action: draft.action,
        enabled: draft.enabled,
        source: RuleSource::Console,
        // 类别只由内容池的登记产生：控制台草稿认的是条件，没有「这段原文」可归类。
        category: None,
    };
    state.store.rules().create(&rule).await?;
    state.reload_rules().await?;
    Ok((StatusCode::CREATED, Json(rule)))
}

pub async fn update_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(draft): Json<RuleDraft>,
) -> Result<Json<Rule>, ApiError> {
    let id = RuleId::new(id)?;
    let existing = state
        .store
        .rules()
        .find(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("rule {id}")))?;

    let rule = Rule {
        id,
        name: draft.name,
        priority: draft.priority,
        condition: draft.condition,
        action: draft.action,
        enabled: draft.enabled,
        // 来源描述规则最初如何产生，不由客户端改写。
        source: existing.source,
        // 类别是登记的一部分，而规则页改的是条件与动作，因此原样保留：编辑规则不该把登记改残。
        category: existing.category,
    };
    state.store.rules().update(&rule).await?;
    state.reload_rules().await?;
    Ok(Json(rule))
}

#[derive(Debug, Deserialize)]
pub struct EnabledPatch {
    pub enabled: bool,
}

pub async fn set_rule_enabled(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(patch): Json<EnabledPatch>,
) -> Result<StatusCode, ApiError> {
    state
        .store
        .rules()
        .set_enabled(&RuleId::new(id)?, patch.enabled)
        .await?;
    state.reload_rules().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_rule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state.store.rules().delete(&RuleId::new(id)?).await?;
    state.reload_rules().await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// 上游 provider
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ProviderList {
    pub items: Vec<Provider>,
}

pub async fn providers(State(state): State<Arc<AppState>>) -> Result<Json<ProviderList>, ApiError> {
    Ok(Json(ProviderList {
        items: state.store.providers().list().await?,
    }))
}

pub async fn create_provider(
    State(state): State<Arc<AppState>>,
    Json(draft): Json<ProviderDraft>,
) -> Result<(StatusCode, Json<Provider>), ApiError> {
    let provider = state.store.providers().create(&draft).await?;
    Ok((StatusCode::CREATED, Json(provider)))
}

pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(draft): Json<ProviderDraft>,
) -> Result<Json<Provider>, ApiError> {
    Ok(Json(state.store.providers().update(&id, &draft).await?))
}

pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state.store.providers().delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// 供代理复用：按请求选择上游
// ---------------------------------------------------------------------------

/// 控制台之外唯一的读取路径：代理为一次入站请求挑选上游。
pub async fn select_provider(
    state: &AppState,
    format: Option<privacy_store::ApiFormat>,
    headers: &HeaderMap,
) -> Result<Provider, ApiError> {
    if let Some(name) = headers
        .get("x-privacy-router-provider")
        .and_then(|value| value.to_str().ok())
    {
        let provider = state
            .store
            .providers()
            .find_by_name(name)
            .await?
            .ok_or_else(|| ApiError::not_found(format!("provider '{name}'")))?;
        if !provider.enabled {
            return Err(ApiError::bad_request(format!(
                "provider '{name}' is disabled"
            )));
        }
        // 协议必须与入站端点一致：本代理只脱敏、不翻译协议。
        if format.is_some_and(|format| provider.api_format != format) {
            return Err(ApiError::bad_request(format!(
                "provider '{name}' speaks {:?} but the request is {format:?}",
                provider.api_format
            )));
        }
        return Ok(provider);
    }

    state
        .store
        .providers()
        .list()
        .await?
        .into_iter()
        .find(|provider| {
            provider.enabled && format.is_none_or(|format| provider.api_format == format)
        })
        .ok_or_else(|| {
            ApiError::not_found(format!("no enabled provider configured for {format:?}"))
        })
}
