//! 各测试目标共用的领域对象构造。

#![allow(dead_code)]

use privacy_filter::Entity;
use privacy_rules::EntityGroup;
use privacy_rules::{Action, Pattern, Rule, RuleExpression, RuleId, RuleSet, RuleSource, priority};
use rstest::fixture;

/// 以「类别 + 置信度 + 所在文本 + 起始字节 + 命中文本」构造实体。
///
/// 结束偏移由命中文本的长度导出，与解码器的约定一致：`word == &text[start..end]`。
#[fixture]
pub fn entity() -> impl Fn(EntityGroup, f64, &str, usize, &str) -> Entity {
    |entity_group, score, _text, start, word| Entity {
        entity_group,
        score,
        start,
        end: start + word.len(),
        word: word.to_owned(),
    }
}

/// 一套只做抹去、不依赖模型概率的规则，用于把替换行为与模型输出隔离开验证。
#[fixture]
pub fn redacting_rules() -> RuleSet {
    RuleSet::new([]).with_fallback(Action::Redact)
}

/// 单条放行关键词的规则，用来验证放行分支。
pub fn release_keyword(text: &str) -> Rule {
    Rule {
        id: RuleId::new(format!("test.release.{text}")).expect("非空"),
        name: "测试放行".to_owned(),
        priority: priority::RELEASE_KEYWORD,
        condition: RuleExpression::keyword(Pattern::substring(text)),
        action: Action::Release,
        enabled: true,
        source: RuleSource::Console,
        category: None,
    }
}

/// 强制抹去某个类别的规则，用于验证优先级。
pub fn force_redact(group: EntityGroup) -> Rule {
    Rule {
        id: RuleId::new(format!("test.redact.{group}")).expect("非空"),
        name: "测试抹去".to_owned(),
        priority: priority::FORCE_REDACT,
        condition: RuleExpression::entity(group),
        action: Action::Redact,
        enabled: true,
        source: RuleSource::Console,
        category: None,
    }
}

/// 确定性分类器：按字面量在文本中定位，产出与真实解码器同构的实体。
///
/// 管线测试关心的是「定位、判定、替换、留痕」这条链路，不需要模型；用字面量定位也让断言
/// 可以预期到具体字节。
pub struct FakeClassifier {
    needles: Vec<(&'static str, EntityGroup, f64)>,
}

impl FakeClassifier {
    pub fn new(needles: Vec<(&'static str, EntityGroup, f64)>) -> Self {
        Self { needles }
    }

    /// 一个不识别任何内容的分类器。
    pub fn blind() -> Self {
        Self::new(Vec::new())
    }
}

impl privacy_router::Classifier for FakeClassifier {
    fn classify(&self, texts: &[&str]) -> privacy_filter::Result<Vec<Vec<privacy_filter::Entity>>> {
        Ok(texts
            .iter()
            .map(|text| {
                self.needles
                    .iter()
                    .flat_map(|(needle, group, score)| {
                        // 与真实解码器一致：同一段文本里重复出现的值各自产出一条实体。
                        text.match_indices(needle).map(move |(start, found)| {
                            privacy_filter::Entity {
                                entity_group: *group,
                                score: *score,
                                start,
                                end: start + found.len(),
                                word: found.to_owned(),
                            }
                        })
                    })
                    .collect()
            })
            .collect())
    }
}

/// 识别失败分类器：用于验证失败路径不会放行原文。
pub struct FailingClassifier;

impl privacy_router::Classifier for FailingClassifier {
    fn classify(&self, _: &[&str]) -> privacy_filter::Result<Vec<Vec<privacy_filter::Entity>>> {
        Err(privacy_filter::Error::Inference("model unavailable".into()))
    }
}

/// 返回数量与输入不匹配的分类器：用于验证错位结果被拒绝而不是被用来替换。
pub struct MismatchedClassifier;

impl privacy_router::Classifier for MismatchedClassifier {
    fn classify(&self, _: &[&str]) -> privacy_filter::Result<Vec<Vec<privacy_filter::Entity>>> {
        Ok(Vec::new())
    }
}

// ---------------------------------------------------------------------------
// 临时数据库
// ---------------------------------------------------------------------------

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 每个测试独占一个 SQLite 文件；析构时清掉 WAL 附属文件。
pub struct TempDatabase {
    path: PathBuf,
}

impl TempDatabase {
    pub fn new() -> Self {
        let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "privacy-router-test-{}-{sequence}.db",
            std::process::id()
        ));
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
        Self { path }
    }

    pub async fn open(&self) -> privacy_store::Store {
        privacy_store::Store::open(&self.path)
            .await
            .expect("临时数据库必须可打开并完成迁移")
    }
}

impl Drop for TempDatabase {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.path.display()));
        }
    }
}

// ---------------------------------------------------------------------------
// 端到端脚手架：假上游 + 代理应用
// ---------------------------------------------------------------------------

use clap::Parser;
use privacy_router::classifier::Classifier;
use privacy_router::config::Cli;
use privacy_router::server;
use privacy_router::state::AppState;
use privacy_store::{ApiFormat, ProviderDraft, Store};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

/// 记录假上游收到的每一个请求体。
#[derive(Clone, Default)]
pub struct Received(Arc<Mutex<Vec<Value>>>);

impl Received {
    pub fn bodies(&self) -> Vec<Value> {
        self.0.lock().expect("未被毒化").clone()
    }

    pub fn count(&self) -> usize {
        self.bodies().len()
    }
}

/// 起一个真实的 HTTP 上游，把收到的报文记下来供断言。
pub async fn fake_upstream() -> (SocketAddr, Received) {
    let received = Received::default();
    let recorder = received.clone();

    let app = axum::Router::new().fallback(move |body: axum::body::Bytes| {
        let recorder = recorder.clone();
        async move {
            if let Ok(value) = serde_json::from_slice::<Value>(&body) {
                recorder.0.lock().expect("未被毒化").push(value);
            }
            axum::Json(serde_json::json!({"ok": true}))
        }
    });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("可监听");
    let addr = listener.local_addr().expect("可取得地址");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (addr, received)
}

/// 代理应用及其依赖，供测试驱动。
pub struct Harness {
    pub app: axum::Router,
    pub store: Arc<Store>,
    /// 进程内共享状态。测试直接往库里写规则后要靠它重新装配，否则缓存里没有新规则。
    pub state: Arc<AppState>,
    _database: TempDatabase,
}

/// 用真实默认配置装配应用；`provider` 为 `None` 时表示没有配置任何上游。
pub async fn harness(classifier: Arc<dyn Classifier>, provider: Option<ProviderDraft>) -> Harness {
    // 默认模拟「浏览器就在运行代理的这台机器上」。
    harness_from(
        classifier,
        provider,
        SocketAddr::from(([127, 0, 0, 1], 51234)),
    )
    .await
}

/// 与 [`harness`] 相同，但指定请求的来源地址。首次初始化据此判断请求是否来自本机。
pub async fn harness_from(
    classifier: Arc<dyn Classifier>,
    provider: Option<ProviderDraft>,
    peer: SocketAddr,
) -> Harness {
    let database = TempDatabase::new();
    let store = Arc::new(database.open().await);

    if let Some(draft) = provider {
        store.providers().create(&draft).await.expect("可创建上游");
    }

    // 从真实默认值出发，避免测试里再抄一份默认配置。
    store
        .rules()
        .seed_builtin(RuleSet::builtin().rules())
        .await
        .expect("默认规则安装成功");
    let mut args = Cli::try_parse_from(["privacy-router"])
        .expect("默认配置可解析")
        .server;
    args.max_body_bytes = 4 * 1024 * 1024;

    let rules = store.rules().load_set().await.expect("可装配规则");
    let state = Arc::new(AppState::new(
        store.clone(),
        classifier,
        reqwest::Client::new(),
        Arc::new(args),
        rules,
    ));

    Harness {
        // 生产入口用 `into_make_service_with_connect_info` 提供来源地址；测试在这里注入。
        app: server::router(state.clone())
            .layer(axum::extract::connect_info::MockConnectInfo(peer)),
        store,
        state,
        _database: database,
    }
}

pub fn provider_at(addr: SocketAddr) -> ProviderDraft {
    ProviderDraft {
        name: "fake".to_owned(),
        base_url: format!("http://{addr}/v1"),
        api_format: ApiFormat::OpenAiChat,
        enabled: true,
    }
}

/// 发送一个请求并拆成状态码与 JSON 体。
pub async fn send(
    app: &axum::Router,
    request: axum::http::Request<axum::body::Body>,
) -> (axum::http::StatusCode, Value) {
    let response = app.clone().oneshot(request).await.expect("有响应");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("可读取响应体");
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

pub async fn post(app: &axum::Router, path: &str, body: Value) -> (axum::http::StatusCode, Value) {
    let request = axum::http::Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body.to_string()))
        .expect("可构造请求");
    send(app, request).await
}

/// 无凭据的 GET，用于公开端点。
pub async fn get(app: &axum::Router, path: &str) -> (axum::http::StatusCode, Value) {
    let request = axum::http::Request::builder()
        .method("GET")
        .uri(path)
        .body(axum::body::Body::empty())
        .expect("可构造请求");
    send(app, request).await
}

/// 带会话令牌的请求。
pub async fn authorized(
    app: &axum::Router,
    method: &str,
    path: &str,
    token: &str,
    body: Option<Value>,
) -> (axum::http::StatusCode, Value) {
    let mut builder = axum::http::Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", format!("Bearer {token}"));
    let body = match body {
        Some(value) => {
            builder = builder.header("content-type", "application/json");
            axum::body::Body::from(value.to_string())
        }
        None => axum::body::Body::empty(),
    };
    send(app, builder.body(body).expect("可构造请求")).await
}

/// 测试用的标准隐私文本与其分类器。
pub fn leaky() -> Value {
    serde_json::json!({
        "model": "gpt-5",
        "messages": [{"role": "user", "content": "im AlexExample and my email is alex@example.com"}]
    })
}

pub fn leaky_classifier() -> Arc<dyn Classifier> {
    Arc::new(FakeClassifier::new(vec![
        ("AlexExample", EntityGroup::PrivatePerson, 0.99),
        ("alex@example.com", EntityGroup::PrivateEmail, 0.98),
    ]))
}
