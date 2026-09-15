//! 端到端：确定性过滤器与模型并行给出命中，谁都不会把对方的漏报当自己的责任。
//!
//! 三个方向的退化都在这里钉住：模型把非邮箱读成邮箱（误伤）、模型看不见真命中（漏报）、
//! 以及模型给出的低置信度命中（本该被门槛放行，但形状已经确认）。

mod common;

use common::{FakeClassifier, fake_upstream, harness, post, provider_at};
use privacy_filter::EntityGroup;
use privacy_rules::Action;
use rstest::rstest;
use serde_json::json;
use std::sync::Arc;

#[rstest]
#[tokio::test]
async fn a_model_false_positive_on_email_is_dropped_entirely() {
    let (upstream, received) = fake_upstream().await;
    let classifier = FakeClassifier::new(vec![(
        "-rwxr-xr-x@",
        EntityGroup::PrivateEmail,
        // 高于内置规则的置信度门槛：模型是「确信地」判错的。
        0.99,
    )]);
    let harness = harness(Arc::new(classifier), Some(provider_at(upstream))).await;
    let listing = "-rwxr-xr-x@ 1 timmyovo wheel 15855616 Sep 12 00:31 /tmp/a.db";

    let (status, _) = post(
        &harness.app,
        "/v1/chat/completions",
        json!({"model": "gpt-5", "messages": [{"role": "user", "content": listing}]}),
    )
    .await;

    assert!(status.is_success(), "请求应当成功：{status}");
    assert_eq!(
        received.bodies()[0].pointer("/messages/0/content"),
        Some(&json!(listing)),
        "权限串不是邮箱，不得被改写"
    );
    assert_eq!(
        harness
            .store
            .pool_records()
            .count()
            .await
            .expect("可读池子"),
        0,
        "被丢弃的误报不得留下审计条目"
    );
}

#[rstest]
#[tokio::test]
async fn a_real_email_is_redacted_when_the_model_reports_nothing() {
    let (upstream, received) = fake_upstream().await;
    let harness = harness(
        Arc::new(FakeClassifier::blind()),
        Some(provider_at(upstream)),
    )
    .await;

    let (status, _) = post(
        &harness.app,
        "/v1/chat/completions",
        json!({"model": "gpt-5", "messages": [{"role": "user", "content": "mail alice@example.com now"}]}),
    )
    .await;

    assert!(status.is_success(), "请求应当成功：{status}");
    assert_eq!(
        received.bodies()[0].pointer("/messages/0/content"),
        Some(&json!("mail <redacted_private_email> now"))
    );

    let rows = harness
        .store
        .pool_records()
        .content_page(privacy_store::ContentFilter::All, &[], 10, 0)
        .await
        .expect("可读池子");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].latest_action, Action::Redact);
    assert!(
        rows[0].original_text.contains('@'),
        "聚合身份是那条邮箱本身：{}",
        rows[0].original_text
    );
    assert_eq!(rows[0].categories.len(), 1);
    assert_eq!(
        rows[0].categories[0].entity_group,
        EntityGroup::PrivateEmail
    );
    // 确定性命中不带概率：得分固定为 1.0，不会被置信度门槛挡下。
    assert_eq!(rows[0].score_max, privacy_filter::PatternFilter::SCORE);
}

#[rstest]
#[tokio::test]
async fn a_low_confidence_model_hit_does_not_weaken_a_real_email() {
    let (upstream, received) = fake_upstream().await;
    let classifier = FakeClassifier::new(vec![(
        "alice@example.com",
        EntityGroup::PrivateEmail,
        // 低于内置规则的置信度门槛；模型「看见」了，但不敢认。
        0.5,
    )]);
    let harness = harness(Arc::new(classifier), Some(provider_at(upstream))).await;

    let (status, _) = post(
        &harness.app,
        "/v1/chat/completions",
        json!({"model": "gpt-5", "messages": [{"role": "user", "content": "mail alice@example.com now"}]}),
    )
    .await;

    assert!(status.is_success(), "请求应当成功：{status}");
    assert_eq!(
        received.bodies()[0].pointer("/messages/0/content"),
        Some(&json!("mail <redacted_private_email> now")),
        "概率低但形状对：仍然必须抹去"
    );
}

/// 身份证、银行卡、统一社会信用代码、MAC、IP 由过滤器直接给出，模型不必看见。
#[rstest]
#[case("身份证 110101199003077213 已登记", "身份证 <redacted_id_card> 已登记")]
#[case("卡号 6222021234567894 已解绑", "卡号 <redacted_bank_card> 已解绑")]
#[case(
    "统一社会信用代码 91350100M000100Y43",
    "统一社会信用代码 <redacted_credit_code>"
)]
#[case("网卡 aa:bb:cc:dd:ee:ff 已上线", "网卡 <redacted_mac_address> 已上线")]
#[case("网关 93.184.216.34 不可达", "网关 <redacted_ip_address> 不可达")]
#[tokio::test]
async fn shaped_categories_are_redacted_without_the_model(
    #[case] content: &str,
    #[case] expected: &str,
) {
    let (upstream, received) = fake_upstream().await;
    let harness = harness(
        Arc::new(FakeClassifier::blind()),
        Some(provider_at(upstream)),
    )
    .await;

    let (status, _) = post(
        &harness.app,
        "/v1/chat/completions",
        json!({"model": "gpt-5", "messages": [{"role": "user", "content": content}]}),
    )
    .await;

    assert!(status.is_success(), "请求应当成功：{status}");
    assert_eq!(
        received.bodies()[0].pointer("/messages/0/content"),
        Some(&json!(expected))
    );
}

/// 手机号按国内移动号段过滤，但模型仍在这个类别上发言：境外号码照样拦。
#[rstest]
#[tokio::test]
async fn the_phone_filter_adds_hits_without_silencing_the_model() {
    let (upstream, received) = fake_upstream().await;
    let classifier =
        FakeClassifier::new(vec![("+1 415 555 0132", EntityGroup::PrivatePhone, 0.99)]);
    let harness = harness(Arc::new(classifier), Some(provider_at(upstream))).await;

    let (status, _) = post(
        &harness.app,
        "/v1/chat/completions",
        json!({"model": "gpt-5", "messages": [{"role": "user", "content": "国内 13812345678，境外 +1 415 555 0132"}]}),
    )
    .await;

    assert!(status.is_success(), "请求应当成功：{status}");
    assert_eq!(
        received.bodies()[0].pointer("/messages/0/content"),
        Some(&json!(
            "国内 <redacted_private_phone>，境外 <redacted_private_phone>"
        )),
        "两条命中属于同一类别，但来源不同"
    );
}
