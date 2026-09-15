mod common;

use common::TempDatabase;
use privacy_rules::{Action, EntityGroup};
use privacy_store::{
    ApiFormat, FragmentRecording, ProviderDraft, RecordedSpan, RequestOutcome, RequestTiming,
};
use rstest::rstest;

fn draft(name: &str) -> ProviderDraft {
    ProviderDraft {
        name: name.to_owned(),
        base_url: "https://api.openai.com/".to_owned(),
        api_format: privacy_store::ApiFormat::OpenAiChat,
        enabled: true,
    }
}

#[rstest]
#[tokio::test]
async fn providers_round_trip_through_storage() {
    let database = TempDatabase::new();
    let store = database.open().await;

    let created = store
        .providers()
        .create(&draft("openai"))
        .await
        .expect("可创建");
    assert_eq!(created.name, "openai");
    // 末尾斜杠被归一化，避免与上游路径拼接时出现双斜杠。
    assert_eq!(created.base_url, "https://api.openai.com");

    assert_eq!(
        store.providers().find(&created.id).await.expect("可查询"),
        Some(created.clone())
    );
    assert_eq!(
        store
            .providers()
            .find_by_name("openai")
            .await
            .expect("可查询"),
        Some(created.clone())
    );

    let mut changed = draft("openai-renamed");
    changed.base_url = "https://proxy.internal".to_owned();
    let updated = store
        .providers()
        .update(&created.id, &changed)
        .await
        .expect("可更新");
    assert_eq!(updated.name, "openai-renamed");
    assert_eq!(updated.api_format, privacy_store::ApiFormat::OpenAiChat);

    store.providers().delete(&created.id).await.expect("可删除");
    assert_eq!(
        store.providers().find(&created.id).await.expect("可查询"),
        None
    );
}

#[rstest]
#[tokio::test]
async fn provider_names_are_unique() {
    let database = TempDatabase::new();
    let store = database.open().await;
    store
        .providers()
        .create(&draft("openai"))
        .await
        .expect("可创建");

    assert!(matches!(
        store.providers().create(&draft("openai")).await,
        Err(privacy_store::Error::ProviderNameTaken(_))
    ));
}

#[rstest]
#[case("", "https://api.openai.com/v1")]
#[case("openai", "api.openai.com/v1")]
#[tokio::test]
async fn invalid_provider_drafts_are_rejected_before_touching_the_database(
    #[case] name: &str,
    #[case] base_url: &str,
) {
    let database = TempDatabase::new();
    let store = database.open().await;

    let invalid = ProviderDraft {
        name: name.to_owned(),
        base_url: base_url.to_owned(),
        api_format: privacy_store::ApiFormat::OpenAiChat,
        enabled: true,
    };

    assert!(matches!(
        store.providers().create(&invalid).await,
        Err(privacy_store::Error::InvalidProvider(_))
    ));
    assert!(store.providers().list().await.expect("可查询").is_empty());
}

#[rstest]
#[tokio::test]
async fn updating_a_missing_provider_reports_not_found() {
    let database = TempDatabase::new();
    let store = database.open().await;

    assert!(matches!(
        store.providers().update("absent", &draft("x")).await,
        Err(privacy_store::Error::NotFound(_))
    ));
    assert!(matches!(
        store.providers().delete("absent").await,
        Err(privacy_store::Error::NotFound(_))
    ));
}

#[rstest]
#[tokio::test]
async fn request_timings_are_aggregated_into_statistics() {
    let database = TempDatabase::new();
    let store = database.open().await;

    let mut released = span();
    released.action = Action::Release;
    released.entity_group = EntityGroup::PrivateEmail;
    let mut redacted = span();
    redacted.action = Action::Redact;
    redacted.entity_group = EntityGroup::Secret;

    store
        .pool_records()
        .record(&FragmentRecording {
            request_id: "req-1".to_owned(),
            api_format: ApiFormat::OpenAiChat,
            path: "/v1/chat/completions".to_owned(),
            content_hash: vec![1, 2, 3],
            original_text: "a".to_owned(),
            redacted_text: "b".to_owned(),
            spans: vec![released, redacted],
        })
        .await
        .expect("可写入");

    for (index, duration) in [100, 200, 300, 400].into_iter().enumerate() {
        store
            .metrics()
            .record(&RequestOutcome {
                request_id: format!("req-{index}"),
                provider_id: None,
                api_format: ApiFormat::OpenAiChat,
                status: 200,
                timing: RequestTiming {
                    queue_ms: 1,
                    inference_ms: 10,
                    upstream_ms: 20,
                    duration_ms: duration,
                },
                cache_usage_json: r#"{"cached_fragments":2,"inferred_fragments":1}"#.to_owned(),
                error_kind: None,
            })
            .await
            .expect("可写入");
    }

    let stats = store.metrics().summary(0).await.expect("可汇总");

    assert_eq!(stats.requests, 4);
    assert_eq!(stats.fragments, 1);
    assert_eq!(stats.spans, 2);
    assert_eq!(stats.released_spans, 1);
    assert_eq!(stats.redacted_spans, 1);
    assert_eq!(stats.latency.average_ms, 250);
    assert_eq!(stats.latency.max_ms, 400);
    assert_eq!(stats.latency.p50_ms, 200);
    assert_eq!(stats.latency.p95_ms, 400);
    assert_eq!(stats.latency.average_inference_ms, 10);
    assert_eq!(stats.latency.average_upstream_ms, 20);
    // 缓存用量按请求累加：每条记录 2 命中、1 推理。
    assert_eq!(stats.cached_fragments, 8);
    assert_eq!(stats.inferred_fragments, 4);
    assert_eq!(stats.by_entity_group.len(), 2);
}

#[rstest]
#[tokio::test]
async fn statistics_respect_the_time_window() {
    let database = TempDatabase::new();
    let store = database.open().await;

    store
        .metrics()
        .record(&RequestOutcome {
            request_id: "req-old".to_owned(),
            provider_id: None,
            api_format: ApiFormat::OpenAiChat,
            status: 200,
            timing: RequestTiming {
                duration_ms: 5,
                ..RequestTiming::default()
            },
            cache_usage_json: "{}".to_owned(),
            error_kind: None,
        })
        .await
        .expect("可写入");

    // 窗口起点晚于该请求的开始时间，统计必须为空。
    let future = store.now_ms() + 1_000;
    let stats = store.metrics().summary(future).await.expect("可汇总");
    assert_eq!(stats.requests, 0);
    assert_eq!(stats.latency.average_ms, 0, "没有数据时不得显示成有延迟");
    assert_eq!(stats.latency.p95_ms, 0);
}

#[rstest]
#[tokio::test]
async fn recording_the_same_request_twice_does_not_double_count() {
    let database = TempDatabase::new();
    let store = database.open().await;

    let outcome = RequestOutcome {
        request_id: "req-once".to_owned(),
        provider_id: None,
        api_format: ApiFormat::OpenAiChat,
        status: 200,
        timing: RequestTiming::default(),
        cache_usage_json: "{}".to_owned(),
        error_kind: None,
    };

    store.metrics().record(&outcome).await.expect("可写入");
    store.metrics().record(&outcome).await.expect("可重放");

    assert_eq!(
        store.metrics().summary(0).await.expect("可汇总").requests,
        1
    );
}

#[rstest]
#[tokio::test]
async fn deleting_a_provider_keeps_the_requests_that_used_it() {
    let database = TempDatabase::new();
    let store = database.open().await;
    let provider = store
        .providers()
        .create(&draft("openai"))
        .await
        .expect("可创建");

    store
        .metrics()
        .record(&RequestOutcome {
            request_id: "req-1".to_owned(),
            provider_id: Some(provider.id.clone()),
            api_format: ApiFormat::OpenAiChat,
            status: 200,
            timing: RequestTiming::default(),
            cache_usage_json: "{}".to_owned(),
            error_kind: None,
        })
        .await
        .expect("可写入");

    store
        .providers()
        .delete(&provider.id)
        .await
        .expect("可删除");

    let stats = store.metrics().summary(0).await.expect("可汇总");
    assert_eq!(stats.requests, 1, "历史请求不随 provider 删除而消失");
}

fn span() -> RecordedSpan {
    RecordedSpan {
        id: String::new(),
        entity_group: EntityGroup::PrivateEmail,
        score: 0.9,
        char_len: 13,
        byte_start: 0,
        byte_end: 13,
        original_text: "a@example.com".to_owned(),
        action: Action::Redact,
        matched_rule_id: None,
    }
}
