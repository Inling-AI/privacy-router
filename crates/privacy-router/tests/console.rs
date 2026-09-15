//! 控制台流程：登录、查看内容池、逐条登记放行或禁止，并确认登记对后续请求立即生效。
//!
//! 内容池按**内容**聚合，不按 turn：同一段文本出现多次就是一行，因此这里的入口一律是
//! `/api/content`（一行）、`/api/spans/{id}/occurrences`（一行里的全部出现）与
//! `/api/spans/{id}/fragment`（一次出现所属的 turn）。

mod common;

use common::{
    authorized, fake_upstream, get, harness, harness_from, leaky, leaky_classifier, post,
    provider_at,
};
use privacy_filter::EntityGroup;
use privacy_store::Credentials;
use rstest::rstest;
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// 测试关心的是控制台流程，不是凭据校验，因此在这里一次性构造。
fn credentials(username: &str, password: &str) -> Credentials {
    Credentials::new(username, password).expect("测试凭据非空")
}

async fn logged_in(app: &axum::Router, store: &privacy_store::Store) -> String {
    store
        .admin()
        .create(&credentials("admin", "correct horse battery staple"))
        .await
        .expect("可创建管理员");

    let (status, body) = post(
        app,
        "/api/session",
        json!({"username": "admin", "password": "correct horse battery staple"}),
    )
    .await;
    assert!(status.is_success(), "登录应当成功：{body}");
    body["token"].as_str().expect("返回令牌").to_owned()
}

/// 判定随上下文摇摆的分类器：同一段文本第一次被认作姓名（命中预置抹去），之后被认作日期
/// （不设档位，交给兜底放行）。
///
/// 这正是要复核的局面——判定的对象不是某一次请求，而是这段内容在放行与抹去之间反复横跳。
struct FlippingClassifier {
    calls: AtomicUsize,
}

impl privacy_router::classifier::Classifier for FlippingClassifier {
    fn classify(&self, texts: &[&str]) -> privacy_filter::Result<Vec<Vec<privacy_filter::Entity>>> {
        let name = self.calls.fetch_add(1, Ordering::Relaxed) == 0;
        Ok(texts
            .iter()
            .map(|text| {
                text.match_indices("ACME-secret")
                    .map(|(start, found)| privacy_filter::Entity {
                        entity_group: if name {
                            EntityGroup::PrivatePerson
                        } else {
                            EntityGroup::PrivateDate
                        },
                        score: 0.95,
                        start,
                        end: start + found.len(),
                        word: found.to_owned(),
                    })
                    .collect()
            })
            .collect())
    }
}

/// 发送一条只包含待复核文本的请求，让同一段内容再出现一次。
fn flipping_message() -> Value {
    json!({
        "model": "gpt-5",
        "messages": [{"role": "user", "content": "ACME-secret"}]
    })
}

/// 内容池的一页。`filter` 取 `unreviewed`（默认视图）、`reviewed` 或 `all`。
async fn content_rows(app: &axum::Router, token: &str, filter: &str) -> Vec<Value> {
    let (status, page) = authorized(
        app,
        "GET",
        &format!("/api/content?filter={filter}"),
        token,
        None,
    )
    .await;
    assert!(status.is_success(), "内容池应当可读：{page}");
    page["items"].as_array().cloned().expect("有内容行")
}

/// 原文为 `text` 的那一行内容。
async fn content_row(app: &axum::Router, token: &str, text: &str) -> Value {
    content_rows(app, token, "all")
        .await
        .into_iter()
        .find(|row| row["original_text"] == text)
        .unwrap_or_else(|| panic!("内容池应当有 {text} 这一行"))
}

/// 一行内容里某个类别的那次出现。登记用它的命中标识做锚点。
async fn occurrence_of(app: &axum::Router, token: &str, row: &Value, group: &str) -> String {
    let anchor = row["anchor_span_id"].as_str().expect("有锚点");
    let (status, page) = authorized(
        app,
        "GET",
        &format!("/api/spans/{anchor}/occurrences"),
        token,
        None,
    )
    .await;
    assert!(status.is_success(), "出现列表应当可读：{page}");
    page["items"]
        .as_array()
        .expect("有出现")
        .iter()
        .find(|item| item["entity_group"] == group)
        .unwrap_or_else(|| panic!("应当出现 {group}：{page}"))["span_id"]
        .as_str()
        .expect("有命中标识")
        .to_owned()
}

#[rstest]
#[tokio::test]
async fn login_rejects_a_wrong_password() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    harness
        .store
        .admin()
        .create(&credentials("admin", "right-password"))
        .await
        .expect("可创建管理员");

    let (status, _) = post(
        &harness.app,
        "/api/session",
        json!({"username": "admin", "password": "wrong-password"}),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[rstest]
#[tokio::test]
async fn an_unknown_username_is_indistinguishable_from_a_wrong_password() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    harness
        .store
        .admin()
        .create(&credentials("admin", "right-password"))
        .await
        .expect("可创建管理员");

    let (unknown_user, _) = post(
        &harness.app,
        "/api/session",
        json!({"username": "someone", "password": "right-password"}),
    )
    .await;
    let (wrong_password, _) = post(
        &harness.app,
        "/api/session",
        json!({"username": "admin", "password": "nope"}),
    )
    .await;

    // 两者必须给出同样的响应，否则账号是否存在可以被枚举出来。
    assert_eq!(unknown_user, wrong_password);
    assert_eq!(unknown_user, axum::http::StatusCode::UNAUTHORIZED);
}

#[rstest]
#[tokio::test]
async fn logging_out_invalidates_the_token() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    let token = logged_in(&harness.app, &harness.store).await;

    let (status, _) = authorized(&harness.app, "DELETE", "/api/session", &token, None).await;
    assert!(status.is_success());

    let (status, _) = authorized(&harness.app, "GET", "/api/stats", &token, None).await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[rstest]
#[tokio::test]
async fn proxied_traffic_appears_in_the_pool_and_in_the_statistics() {
    let (addr, received) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    let token = logged_in(&harness.app, &harness.store).await;

    let (status, _) = post(&harness.app, "/v1/chat/completions", leaky()).await;
    assert!(status.is_success());
    assert_eq!(received.count(), 1);

    // 内容池按内容聚合：这条消息里的两段命中各自成为一行。
    let rows = content_rows(&harness.app, &token, "all").await;
    assert_eq!(rows.len(), 2, "内容池按内容聚合：{rows:?}");
    for row in &rows {
        assert_eq!(row["occurrences"], 1, "各自只出现过一次：{row}");
        assert_eq!(row["turns"], 1, "各自只在一个 turn 里：{row}");
    }

    // 统计异步落库：通过公开接口等待最终结果，而不是依赖后台任务的调度顺序。
    let stats = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let (status, stats) = authorized(&harness.app, "GET", "/api/stats", &token, None).await;
            assert!(status.is_success(), "统计接口应当成功：{stats}");
            if stats["requests"] != 0 {
                break stats;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("代理请求的统计应在两秒内可见");
    assert_eq!(stats["requests"], 1);
    assert_eq!(stats["fragments"], 1);
    assert_eq!(stats["spans"], 2);
    assert_eq!(stats["redacted_spans"], 2);
    assert_eq!(stats["released_spans"], 0);
}

/// 这条测试覆盖产品要求的核心闭环：管理员放行一条内容后，同一内容不再被抹去。
#[rstest]
#[tokio::test]
async fn releasing_a_finding_lets_the_same_content_through_on_the_next_request() {
    let (addr, received) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    let token = logged_in(&harness.app, &harness.store).await;

    post(&harness.app, "/v1/chat/completions", leaky()).await;
    let row = content_row(&harness.app, &token, "AlexExample").await;
    let span_id = occurrence_of(&harness.app, &token, &row, "private_person").await;

    let (status, outcome) = authorized(
        &harness.app,
        "POST",
        &format!("/api/spans/{span_id}/release"),
        &token,
        None,
    )
    .await;
    assert!(status.is_success(), "放行应当成功：{outcome}");
    assert_eq!(outcome["changed"], true, "首次放行应当登记");

    // 再次发送同样的内容：被放行的姓名按原文上行，邮箱仍然被抹去。
    post(&harness.app, "/v1/chat/completions", leaky()).await;

    let bodies = received.bodies();
    assert_eq!(bodies.len(), 2);
    let second = bodies[1].to_string();
    assert!(
        second.contains("AlexExample"),
        "放行后姓名应当原样上行：{second}"
    );
    assert!(
        !second.contains("alex@example.com"),
        "未放行的邮箱仍然必须被抹去：{second}"
    );
    assert_eq!(
        bodies[1].pointer("/messages/0/content"),
        Some(&json!(
            "im AlexExample and my email is <redacted_private_email>"
        ))
    );
}

#[rstest]
#[tokio::test]
async fn releasing_the_same_content_twice_reuses_one_rule() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    let token = logged_in(&harness.app, &harness.store).await;

    post(&harness.app, "/v1/chat/completions", leaky()).await;
    let row = content_row(&harness.app, &token, "AlexExample").await;
    let span_id = occurrence_of(&harness.app, &token, &row, "private_person").await;

    let (_, first) = authorized(
        &harness.app,
        "POST",
        &format!("/api/spans/{span_id}/release"),
        &token,
        None,
    )
    .await;
    let (_, second) = authorized(
        &harness.app,
        "POST",
        &format!("/api/spans/{span_id}/release"),
        &token,
        None,
    )
    .await;

    assert_eq!(second["changed"], false, "重复放行不改动登记");
    assert_eq!(first["rule_id"], second["rule_id"]);
}

/// 管理员改主意时改写的是同一条登记：同一段文本不会同时躺着放行与禁止两个方向。
#[rstest]
#[tokio::test]
async fn denying_a_released_finding_redacts_it_again() {
    let (addr, received) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    let token = logged_in(&harness.app, &harness.store).await;

    post(&harness.app, "/v1/chat/completions", leaky()).await;
    let row = content_row(&harness.app, &token, "AlexExample").await;
    let span_id = occurrence_of(&harness.app, &token, &row, "private_person").await;

    // 先放行：下一次请求里姓名原样上行。
    let (status, released) = authorized(
        &harness.app,
        "POST",
        &format!("/api/spans/{span_id}/release"),
        &token,
        None,
    )
    .await;
    assert!(status.is_success(), "放行应当成功：{released}");
    post(&harness.app, "/v1/chat/completions", leaky()).await;
    assert!(
        received.bodies()[1].to_string().contains("AlexExample"),
        "放行后姓名应当原样上行"
    );

    // 再禁止同一段文本：改写的是同一条登记，而不是另立一条反方向的规则。
    let (status, denied) = authorized(
        &harness.app,
        "POST",
        &format!("/api/spans/{span_id}/redact"),
        &token,
        None,
    )
    .await;
    assert!(status.is_success(), "禁止应当成功：{denied}");
    assert_eq!(denied["changed"], true, "改判改动了登记");
    assert_eq!(
        denied["rule_id"], released["rule_id"],
        "同一段文本只有一条登记，改判改的就是它"
    );

    post(&harness.app, "/v1/chat/completions", leaky()).await;
    let bodies = received.bodies();
    assert_eq!(bodies.len(), 3);
    let third = bodies[2].to_string();
    assert!(
        !third.contains("AlexExample"),
        "禁止之后姓名不得再上行：{third}"
    );

    // 规则表里只有这一条登记：改判是改写，不是往同一个方向再叠一条。
    let (_, rules) = authorized(&harness.app, "GET", "/api/rules", &token, None).await;
    let rule_id = denied["rule_id"].as_str().expect("有规则标识");
    let registered = rules["items"]
        .as_array()
        .expect("有规则")
        .iter()
        .filter(|rule| rule["id"] == rule_id)
        .count();
    assert_eq!(registered, 1, "同一段内容只登记一条规则：{rules}");

    // 内容池里的这一行报出当前登记的方向。
    let row = content_row(&harness.app, &token, "AlexExample").await;
    assert_eq!(row["registered_action"], "redact");
}

/// 历史上「改主意」是往同一段文本上再加一条反方向的规则；一次登记要把它们收敛成一条。
///
/// 不收敛的话，列表报出的「已登记」可能是那条被优先级压住的规则，界面与实际判定对不上。
#[rstest]
#[tokio::test]
async fn registering_again_collapses_leftover_registrations_on_the_same_text() {
    let (addr, received) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    let token = logged_in(&harness.app, &harness.store).await;

    post(&harness.app, "/v1/chat/completions", leaky()).await;
    let row = content_row(&harness.app, &token, "AlexExample").await;
    let span_id = occurrence_of(&harness.app, &token, &row, "private_person").await;

    // 制造遗留状态：同一段文本上先放行、再禁止，两条各自成规则。
    let released = privacy_rules::Rule::operator_release(
        privacy_rules::RuleId::new("operator.legacy.release").expect("非空"),
        "AlexExample",
        privacy_rules::EntityGroup::PrivatePerson,
    );
    let denied = privacy_rules::Rule::operator_redact(
        privacy_rules::RuleId::new("operator.legacy.redact").expect("非空"),
        "AlexExample",
        privacy_rules::EntityGroup::PrivatePerson,
    );
    harness
        .store
        .rules()
        .create(&released)
        .await
        .expect("可写入");
    harness.store.rules().create(&denied).await.expect("可写入");
    harness.state.reload_rules().await.expect("可装配");

    // 算数的是优先级最高的那条：列表据此报出「已登记：禁止」。
    let row = content_row(&harness.app, &token, "AlexExample").await;
    assert_eq!(row["registered_action"], "redact");

    let (status, outcome) = authorized(
        &harness.app,
        "POST",
        &format!("/api/spans/{span_id}/release"),
        &token,
        None,
    )
    .await;
    assert!(status.is_success(), "放行应当成功：{outcome}");
    assert_eq!(outcome["changed"], true, "这次点击改动了登记");

    let (_, rules) = authorized(&harness.app, "GET", "/api/rules", &token, None).await;
    let registrations: Vec<&Value> = rules["items"]
        .as_array()
        .expect("有规则")
        .iter()
        .filter(|rule| {
            rule["source"] == "approval"
                && rule["condition"]["filter"]["pattern"]["text"] == "AlexExample"
        })
        .collect();
    assert_eq!(registrations.len(), 1, "同一段文本只留一条登记：{rules}");
    assert_eq!(registrations[0]["action"], "release");

    // 收敛之后立刻生效：姓名原样上行。
    post(&harness.app, "/v1/chat/completions", leaky()).await;
    assert!(
        received.bodies()[1].to_string().contains("AlexExample"),
        "收敛后的登记必须马上生效"
    );

    let row = content_row(&harness.app, &token, "AlexExample").await;
    assert_eq!(row["registered_action"], "release");
}

/// 默认视图只给判定摇摆的内容，并且按摇摆程度排序。
#[rstest]
#[tokio::test]
async fn the_pool_lists_repeated_content_that_keeps_flipping() {
    let (addr, _) = fake_upstream().await;
    let classifier: Arc<dyn privacy_router::classifier::Classifier> =
        Arc::new(FlippingClassifier {
            calls: AtomicUsize::new(0),
        });
    let harness = harness(classifier, Some(provider_at(addr))).await;
    let token = logged_in(&harness.app, &harness.store).await;

    // 同一段文本出现三次：抹去、放行、放行。
    for _ in 0..3 {
        post(&harness.app, "/v1/chat/completions", flipping_message()).await;
    }

    let rows = content_rows(&harness.app, &token, "all").await;
    assert_eq!(rows.len(), 1, "摇摆的这段内容应当只有一行：{rows:?}");
    let row = &rows[0];
    assert_eq!(row["original_text"], "ACME-secret");
    assert_eq!(row["occurrences"], 3, "三次出现合成一行：{row}");
    assert_eq!(row["turns"], 3, "来自三个 turn：{row}");
    assert_eq!(row["redacted"], 1);
    assert_eq!(row["released"], 2);
    assert_eq!(row["latest_action"], "release", "最近一次是放行：{row}");
    assert_eq!(row["registered_action"], Value::Null, "还没人登记过");

    let categories = row["categories"].as_array().expect("带类别证据");
    assert_eq!(categories.len(), 2, "它被认成过两个类别：{row}");

    let (status, _) = authorized(
        &harness.app,
        "GET",
        "/api/spans/does-not-exist/occurrences",
        &token,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[rstest]
#[tokio::test]
async fn rules_can_be_created_edited_and_deleted_from_the_console() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    let token = logged_in(&harness.app, &harness.store).await;

    let (status, created) = authorized(
        &harness.app,
        "POST",
        "/api/rules",
        &token,
        Some(json!({
            "name": "放行测试地址",
            "priority": 600,
            "condition": {
                "kind": "filter",
                "filter": {
                    "kind": "keyword",
                    "pattern": {
                        "text": "corp.com",
                        "kind": "substring",
                        "case_sensitive": false
                    }
                }
            },
            "action": "release",
            "enabled": true
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED, "{created}");
    let id = created["id"].as_str().expect("返回标识");
    assert_eq!(created["source"], "console");

    let (status, edited) = authorized(
        &harness.app,
        "PATCH",
        &format!("/api/rules/{id}"),
        &token,
        Some(json!({
            "name": "改名",
            "priority": 700,
            "condition": {
                "kind": "filter",
                "filter": {
                    "kind": "keyword",
                    "pattern": {
                        "text": "corp.com",
                        "kind": "substring",
                        "case_sensitive": false
                    }
                }
            },
            "action": "release",
            "enabled": false
        })),
    )
    .await;
    assert!(status.is_success());
    assert_eq!(edited["enabled"], false);

    // 规则列表反映当前判定集合，并带上兜底动作。
    let (_, list) = authorized(&harness.app, "GET", "/api/rules", &token, None).await;
    assert_eq!(list["fallback_action"], "release");
    assert!(
        list["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let (status, _) = authorized(
        &harness.app,
        "DELETE",
        &format!("/api/rules/{id}"),
        &token,
        None,
    )
    .await;
    assert!(status.is_success());
}

#[rstest]
#[tokio::test]
async fn default_rules_can_be_deleted_through_the_console() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    let token = logged_in(&harness.app, &harness.store).await;

    // 内置规则在启动时补齐；这里手动补一次，模拟已启动的进程。
    harness
        .store
        .rules()
        .seed_builtin(privacy_rules::RuleSet::builtin().rules())
        .await
        .expect("可写入内置规则");

    let (status, _) = authorized(
        &harness.app,
        "DELETE",
        "/api/rules/builtin.redact.secret",
        &token,
        None,
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);
    assert_eq!(
        harness
            .store
            .rules()
            .find(&privacy_rules::RuleId::new("builtin.redact.secret").expect("有效标识"))
            .await
            .expect("可查询"),
        None
    );
}

#[rstest]
#[tokio::test]
async fn providers_can_be_managed_from_the_console() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), None).await;
    let token = logged_in(&harness.app, &harness.store).await;

    let draft = json!({
        "name": "openai",
        "base_url": format!("http://{addr}/"),
        "api_format": "openai_chat",
        "enabled": true
    });

    let (status, created) = authorized(
        &harness.app,
        "POST",
        "/api/providers",
        &token,
        Some(draft.clone()),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED, "{created}");
    assert_eq!(
        created["base_url"],
        format!("http://{addr}"),
        "末尾斜杠被归一化"
    );
    let id = created["id"].as_str().expect("返回标识");

    let (status, list) = authorized(&harness.app, "GET", "/api/providers", &token, None).await;
    assert!(status.is_success());
    assert_eq!(list["items"].as_array().expect("有列表").len(), 1);

    // 重名必须被拒绝。
    let (status, _) = authorized(
        &harness.app,
        "POST",
        "/api/providers",
        &token,
        Some(draft.clone()),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CONFLICT);

    let mut renamed = draft.clone();
    renamed["name"] = Value::String("renamed".to_owned());
    let (status, updated) = authorized(
        &harness.app,
        "PATCH",
        &format!("/api/providers/{id}"),
        &token,
        Some(renamed),
    )
    .await;
    assert!(status.is_success());
    assert_eq!(updated["name"], "renamed");

    let (status, _) = authorized(
        &harness.app,
        "DELETE",
        &format!("/api/providers/{id}"),
        &token,
        None,
    )
    .await;
    assert!(status.is_success());
}

#[rstest]
#[tokio::test]
async fn an_invalid_provider_draft_is_rejected_with_a_reason() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    let token = logged_in(&harness.app, &harness.store).await;

    let (status, body) = authorized(
        &harness.app,
        "POST",
        "/api/providers",
        &token,
        Some(json!({
            "name": "bad",
            "base_url": "not-a-url",
            "api_format": "openai_chat",
            "enabled": true
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["kind"], "bad_request");
}

#[rstest]
#[tokio::test]
async fn health_reports_what_the_console_needs_to_show_setup_hints() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;

    let (status, body) = get(&harness.app, "/api/health").await;

    assert!(status.is_success());
    assert_eq!(body["status"], "ok");
    assert_eq!(body["providers"], 1);
    assert_eq!(body["admin_configured"], false);
}

#[rstest]
#[tokio::test]
async fn first_run_setup_creates_the_administrator_and_signs_it_in() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    assert!(!harness.store.admin().is_configured().await.expect("可查询"));

    let (status, body) = post(
        &harness.app,
        "/api/setup",
        json!({"username": "owner", "password": "correct horse battery staple"}),
    )
    .await;

    assert!(status.is_success(), "首次初始化应当成功：{body}");
    assert!(harness.store.admin().is_configured().await.expect("可查询"));

    // 返回的令牌立刻可用，且初始化之后健康检查不再提示需要设置。
    let token = body["token"].as_str().expect("返回会话令牌");
    let (status, _) = authorized(&harness.app, "GET", "/api/stats", token, None).await;
    assert!(status.is_success());
    let (_, health) = get(&harness.app, "/api/health").await;
    assert_eq!(health["admin_configured"], true);

    // 用户记下的凭据就是登录凭据。
    let (status, _) = post(
        &harness.app,
        "/api/session",
        json!({"username": "owner", "password": "correct horse battery staple"}),
    )
    .await;
    assert!(status.is_success());
}

#[rstest]
#[tokio::test]
async fn setup_refuses_to_overwrite_an_existing_administrator() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    harness
        .store
        .admin()
        .create(&credentials("admin", "original-password"))
        .await
        .expect("可创建管理员");

    let (status, body) = post(
        &harness.app,
        "/api/setup",
        json!({"username": "intruder", "password": "another password"}),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CONFLICT);
    assert_eq!(body["error"]["kind"], "conflict");
    assert!(
        harness
            .store
            .admin()
            .authenticate("admin", "original-password")
            .await
            .expect("可校验"),
        "既有凭据不得被初始化端点覆盖"
    );
}

/// 首次初始化没有凭据可校验，因此只对本机开放：绑定到公网时也不能被别人抢先占用。
#[rstest]
#[tokio::test]
async fn setup_from_another_machine_is_refused() {
    let harness = harness_from(
        leaky_classifier(),
        None,
        SocketAddr::from(([203, 0, 113, 7], 51234)),
    )
    .await;

    let (status, body) = post(
        &harness.app,
        "/api/setup",
        json!({"username": "intruder", "password": "another password"}),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["kind"], "forbidden");
    assert!(!harness.store.admin().is_configured().await.expect("可查询"));
}

#[rstest]
#[case(json!({"username": "  ", "password": "correct horse battery staple"}))]
#[case(json!({"username": "admin", "password": ""}))]
#[tokio::test]
async fn setup_rejects_blank_credentials(#[case] request: Value) {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;

    let (status, body) = post(&harness.app, "/api/setup", request).await;

    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["kind"], "bad_request");
    assert!(!harness.store.admin().is_configured().await.expect("可查询"));
}

/// 复核过滤按「审没审过」分：登记过决定的内容从待审列表里消失，出现在已审列表里。
#[rstest]
#[tokio::test]
async fn the_pool_separates_reviewed_content_from_what_still_needs_a_decision() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    let token = logged_in(&harness.app, &harness.store).await;

    post(&harness.app, "/v1/chat/completions", leaky()).await;

    // 还没登记过任何决定：全部内容都在待审列表里，已审列表为空。
    assert_eq!(
        content_rows(&harness.app, &token, "unreviewed").await.len(),
        2
    );
    assert!(
        content_rows(&harness.app, &token, "reviewed")
            .await
            .is_empty()
    );

    let row = content_row(&harness.app, &token, "AlexExample").await;
    let span_id = occurrence_of(&harness.app, &token, &row, "private_person").await;
    let (status, _) = authorized(
        &harness.app,
        "POST",
        &format!("/api/spans/{span_id}/release"),
        &token,
        None,
    )
    .await;
    assert!(status.is_success(), "登记应当成功");

    // 登记之后：这段内容离开待审列表，进入已审列表，另一段照旧待审。
    let unreviewed: Vec<String> = content_rows(&harness.app, &token, "unreviewed")
        .await
        .iter()
        .map(|row| row["original_text"].as_str().expect("文本").to_owned())
        .collect();
    assert_eq!(unreviewed, ["alex@example.com"], "只剩没登记过的那段");
    let reviewed = content_rows(&harness.app, &token, "reviewed").await;
    assert_eq!(reviewed.len(), 1, "登记过的只有一段：{reviewed:?}");
    assert_eq!(reviewed[0]["original_text"], "AlexExample");
    assert_eq!(reviewed[0]["registered_action"], "release");
    assert_eq!(content_rows(&harness.app, &token, "all").await.len(), 2);
}

/// 一次出现读回它所属的 turn：原文、转发出去的内容，以及这次的全部命中。
#[rstest]
#[tokio::test]
async fn an_occurrence_reads_back_the_turn_it_belongs_to() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    let token = logged_in(&harness.app, &harness.store).await;

    post(&harness.app, "/v1/chat/completions", leaky()).await;

    let row = content_row(&harness.app, &token, "AlexExample").await;
    let span_id = occurrence_of(&harness.app, &token, &row, "private_person").await;
    let (status, detail) = authorized(
        &harness.app,
        "GET",
        &format!("/api/spans/{span_id}/fragment"),
        &token,
        None,
    )
    .await;

    assert!(status.is_success(), "命中所属的 turn 应当可读：{detail}");
    let original = detail["fragment"]["original_text"]
        .as_str()
        .expect("有原文");
    let forwarded = detail["fragment"]["redacted_text"]
        .as_str()
        .expect("有转发内容");
    assert!(
        original.contains("AlexExample"),
        "原文照原样读出：{original}"
    );
    assert!(
        !forwarded.contains("AlexExample"),
        "转发内容里不该还有被抹去的名字：{forwarded}"
    );
    assert_eq!(detail["fragment"]["api_format"], "openai_chat");
    assert_eq!(detail["fragment"]["path"], "/v1/chat/completions");
    assert!(
        detail["spans"]
            .as_array()
            .expect("有命中列表")
            .iter()
            .any(|span| span["original_text"] == "AlexExample"),
        "命中列表要给出具体识别到的内容：{detail}"
    );
}

/// 不存在的命中没有 turn 可读，报未找到而不是空详情。
#[rstest]
#[tokio::test]
async fn an_unknown_occurrence_has_no_turn_to_read() {
    let (addr, _) = fake_upstream().await;
    let harness = harness(leaky_classifier(), Some(provider_at(addr))).await;
    let token = logged_in(&harness.app, &harness.store).await;

    let (status, _) = authorized(
        &harness.app,
        "GET",
        "/api/spans/no-such-span/fragment",
        &token,
        None,
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}
