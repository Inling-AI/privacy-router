mod common;
use privacy_filter::performance::ModelPerformance;
use privacy_store::{ApiFormat, RequestOutcome, RequestTiming};
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn model_summary_weights_throughput_and_excludes_cached_requests() {
    let database = common::TempDatabase::new();
    let store = database.open().await;
    for (index, performance) in [
        ModelPerformance {
            tokens: 100,
            forward_ms: 100.0,
            first_result_ms: Some(25.0),
            ..Default::default()
        },
        ModelPerformance {
            tokens: 300,
            forward_ms: 500.0,
            first_result_ms: Some(75.0),
            ..Default::default()
        },
        ModelPerformance::default(),
    ]
    .iter()
    .enumerate()
    {
        store
            .metrics()
            .record_with_performance(
                &RequestOutcome {
                    request_id: format!("model-{index}"),
                    provider_id: None,
                    api_format: ApiFormat::OpenAiResponses,
                    status: 200,
                    timing: RequestTiming::default(),
                    cache_usage_json: "{}".into(),
                    error_kind: None,
                },
                performance,
            )
            .await
            .unwrap();
    }
    let result = store.metrics().summary(0).await.unwrap().model;
    assert_eq!(result.tokens, 400);
    assert_eq!(result.measured_requests, 2);
    assert_eq!(result.average_first_result_ms, Some(50.0));
    assert_eq!(result.tokens_per_second, Some(400_000.0 / 600.0));
    assert_eq!(result.latest.unwrap().tokens, 0);
    assert_eq!(result.latest_tokens_per_second, None);
}
