mod common;

use privacy_filter::{Entity, Result};
use privacy_router::prefix::PrefixClassifier;
use privacy_router::{Classifier, Processed, RequestContext, process};
use privacy_rules::{Action, EntityGroup, RuleSet};
use privacy_store::{ApiFormat, Store};
use rstest::{fixture, rstest};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

struct Detector {
    identity: &'static str,
    received: Mutex<Vec<String>>,
}

impl Detector {
    fn new(identity: &'static str) -> Arc<Self> {
        Arc::new(Self {
            identity,
            received: Mutex::new(Vec::new()),
        })
    }
}

impl Classifier for Detector {
    fn cache_identity(&self) -> Option<&str> {
        Some(self.identity)
    }

    fn classify(&self, texts: &[&str]) -> Result<Vec<Vec<Entity>>> {
        self.received
            .lock()
            .unwrap()
            .extend(texts.iter().map(|s| s.to_string()));
        Ok(texts
            .iter()
            .map(|text| {
                text.match_indices("alex@example.com")
                    .map(|(start, word)| Entity {
                        entity_group: EntityGroup::PrivateEmail,
                        score: 0.99,
                        start,
                        end: start + word.len(),
                        word: word.into(),
                    })
                    .collect()
            })
            .collect())
    }
}

struct Request {
    body: Value,
    format: ApiFormat,
}

impl Request {
    fn new(format: ApiFormat, texts: &[&str]) -> Self {
        let messages: Vec<_> = texts
            .iter()
            .map(|text| json!({"role":"user", "content":text}))
            .collect();
        let body = match format {
            ApiFormat::OpenAiResponses => json!({"input":messages}),
            _ => json!({"messages":messages}),
        };
        Self { body, format }
    }

    async fn run(self, store: Arc<Store>, detector: Arc<Detector>, action: Action) -> Processed {
        let runtime = tokio::runtime::Handle::current();
        tokio::task::spawn_blocking(move || {
            let classifier =
                PrefixClassifier::new(&self.body, self.format, detector, store, runtime).unwrap();
            process(
                &self.body,
                &RequestContext {
                    request_id: "test".into(),
                    api_format: self.format,
                    path: "/test".into(),
                },
                &classifier,
                &RuleSet::new([]).with_fallback(action),
            )
            .unwrap()
        })
        .await
        .unwrap()
    }
}

#[fixture]
fn database() -> common::TempDatabase {
    common::TempDatabase::new()
}

/// 审计池只收增量 turn。请求每次都会把整段历史重发一遍，命中持久化链的内容此前已经审过，
/// 再产生条目只会把池子淹掉；真正新追加的那一段才是这次要看的东西。
#[rstest]
#[tokio::test]
async fn replaying_a_conversation_does_not_grow_the_audit_pool() {
    let (upstream, _received) = common::fake_upstream().await;
    let harness = common::harness(
        Detector::new("model-a"),
        Some(common::provider_at(upstream)),
    )
    .await;
    let conversation = |texts: &[&str]| {
        json!({
            "model": "gpt-5",
            "messages": texts
                .iter()
                .map(|text| json!({"role": "user", "content": text}))
                .collect::<Vec<_>>(),
        })
    };
    let history = ["alex@example.com", "ordinary text"];

    let (status, _) =
        common::post(&harness.app, "/v1/chat/completions", conversation(&history)).await;
    assert!(status.is_success(), "请求应当成功：{status}");
    assert_eq!(
        harness.store.pool_records().count().await.expect("可统计"),
        1,
        "只有真的含命中的那一段入池"
    );

    let (status, _) =
        common::post(&harness.app, "/v1/chat/completions", conversation(&history)).await;
    assert!(status.is_success(), "请求应当成功：{status}");
    assert_eq!(
        harness.store.pool_records().count().await.expect("可统计"),
        1,
        "重放的历史此前已经审过，不再产生条目"
    );

    let (status, _) = common::post(
        &harness.app,
        "/v1/chat/completions",
        conversation(&[
            "alex@example.com",
            "ordinary text",
            "again alex@example.com",
        ]),
    )
    .await;
    assert!(status.is_success(), "请求应当成功：{status}");
    assert_eq!(
        harness.store.pool_records().count().await.expect("可统计"),
        2,
        "只有新追加的那一段入池"
    );
}

#[rstest]
#[case(ApiFormat::OpenAiChat)]
#[case(ApiFormat::OpenAiResponses)]
#[case(ApiFormat::AnthropicMessages)]
#[tokio::test]
async fn reopened_chain_reuses_history_and_only_infers_appended_turns(
    database: common::TempDatabase,
    #[case] format: ApiFormat,
) {
    let store = Arc::new(database.open().await);
    let detector = Detector::new("model-a");
    let original = Request::new(format, &["alex@example.com", "ordinary text"])
        .run(store.clone(), detector, Action::Redact)
        .await;
    store.pool().close().await;
    drop(store);

    let reopened = Arc::new(database.open().await);
    let detector = Detector::new("model-a");
    let replay = Request::new(format, &["alex@example.com", "ordinary text"])
        .run(reopened.clone(), detector.clone(), Action::Redact)
        .await;
    assert_eq!(replay.body, original.body);
    assert!(detector.received.lock().unwrap().is_empty());
    assert_eq!(replay.usage.cached_fragments, 2);

    Request::new(format, &["alex@example.com", "ordinary text", "new turn"])
        .run(reopened.clone(), detector.clone(), Action::Redact)
        .await;
    assert_eq!(*detector.received.lock().unwrap(), ["new turn"]);

    let released = Request::new(format, &["alex@example.com", "ordinary text"])
        .run(reopened, detector.clone(), Action::Release)
        .await;
    assert!(released.body.to_string().contains("alex@example.com"));
    assert_eq!(*detector.received.lock().unwrap(), ["new turn"]);
}

#[rstest]
#[tokio::test]
async fn editing_history_creates_a_branch_and_preserves_the_old_branch(
    database: common::TempDatabase,
) {
    let store = Arc::new(database.open().await);
    Request::new(ApiFormat::OpenAiChat, &["a", "b", "c"])
        .run(store.clone(), Detector::new("model-a"), Action::Redact)
        .await;
    let detector = Detector::new("model-a");
    Request::new(ApiFormat::OpenAiChat, &["a", "edited", "c"])
        .run(store.clone(), detector.clone(), Action::Redact)
        .await;
    assert_eq!(*detector.received.lock().unwrap(), ["edited", "c"]);
    let replay = Detector::new("model-a");
    Request::new(ApiFormat::OpenAiChat, &["a", "b", "c"])
        .run(store, replay.clone(), Action::Redact)
        .await;
    assert!(replay.received.lock().unwrap().is_empty());
}

#[rstest]
#[tokio::test]
async fn a_new_model_identity_does_not_reuse_old_predictions(database: common::TempDatabase) {
    let store = Arc::new(database.open().await);
    Request::new(ApiFormat::OpenAiChat, &["a"])
        .run(store.clone(), Detector::new("model-a"), Action::Redact)
        .await;
    let detector = Detector::new("model-b");
    Request::new(ApiFormat::OpenAiChat, &["a"])
        .run(store, detector.clone(), Action::Redact)
        .await;
    assert_eq!(*detector.received.lock().unwrap(), ["a"]);
}

#[rstest]
#[tokio::test]
async fn custom_output_replay_preserves_wrapper_and_excludes_tool_arguments(
    database: common::TempDatabase,
) {
    let body = json!({"input":[
        {"type":"custom_tool_call", "name":"exec", "call_id":"c1", "input":"do not scan"},
        {"type":"custom_tool_call_output", "call_id":"c1", "output":[
            {"type":"input_text", "text":json!({"chunk_id":"c", "wall_time_seconds":1, "output":"alex@example.com"}).to_string()}
        ]}
    ]});
    let store = Arc::new(database.open().await);
    let first = Detector::new("model-a");
    let original = Request {
        body: body.clone(),
        format: ApiFormat::OpenAiResponses,
    }
    .run(store.clone(), first.clone(), Action::Redact)
    .await;
    assert_eq!(*first.received.lock().unwrap(), ["alex@example.com"]);
    let second = Detector::new("model-a");
    let replay = Request {
        body,
        format: ApiFormat::OpenAiResponses,
    }
    .run(store, second.clone(), Action::Redact)
    .await;
    assert_eq!(replay.body, original.body);
    assert!(second.received.lock().unwrap().is_empty());
}
