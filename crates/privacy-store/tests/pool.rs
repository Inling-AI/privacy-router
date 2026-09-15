mod common;

use common::TempDatabase;
use privacy_rules::{Action, EntityGroup};
use privacy_store::{ApiFormat, ContentFilter, FragmentRecording, RecordedSpan};
use rstest::rstest;

fn span(byte_start: usize, byte_end: usize, text: &str) -> RecordedSpan {
    RecordedSpan {
        id: String::new(),
        entity_group: EntityGroup::PrivateEmail,
        score: 0.97,
        char_len: text.chars().count(),
        byte_start,
        byte_end,
        original_text: text.to_owned(),
        action: Action::Redact,
        matched_rule_id: None,
    }
}

fn recording(spans: Vec<RecordedSpan>) -> FragmentRecording {
    FragmentRecording {
        request_id: "req-1".to_owned(),
        api_format: ApiFormat::OpenAiChat,
        path: "/v1/chat/completions".to_owned(),
        content_hash: blake3::hash(b"original").as_bytes().to_vec(),
        original_text: "contact alice@corp.com now".to_owned(),
        redacted_text: "contact <redacted_private_email> now".to_owned(),
        spans,
    }
}

/// 一段内容在某一个 turn 里的一次出现。
fn occurrence(group: EntityGroup, score: f64, text: &str, action: Action) -> RecordedSpan {
    RecordedSpan {
        id: String::new(),
        entity_group: group,
        score,
        char_len: text.chars().count(),
        byte_start: 0,
        byte_end: text.len(),
        original_text: text.to_owned(),
        action,
        matched_rule_id: None,
    }
}

/// 一个 turn：同一条消息里可能同时出现多段内容。
fn turn(index: usize, original: &str, spans: Vec<RecordedSpan>) -> FragmentRecording {
    FragmentRecording {
        request_id: format!("req-{index}"),
        api_format: ApiFormat::OpenAiChat,
        path: "/v1/chat/completions".to_owned(),
        content_hash: blake3::hash(original.as_bytes()).as_bytes().to_vec(),
        original_text: original.to_owned(),
        redacted_text: original.to_owned(),
        spans,
    }
}

#[rstest]
#[tokio::test]
async fn a_fragment_and_its_spans_are_written_together() {
    let database = TempDatabase::new();
    let store = database.open().await;

    let id = store
        .pool_records()
        .record(&recording(vec![span(8, 22, "alice@corp.com")]))
        .await
        .expect("可写入");

    let fragment = store
        .pool_records()
        .find(&id)
        .await
        .expect("可查询")
        .expect("片段存在");
    assert_eq!(fragment.original_text, "contact alice@corp.com now");
    assert_eq!(
        fragment.redacted_text,
        "contact <redacted_private_email> now"
    );

    let spans = store.pool_records().spans_of(&id).await.expect("可查询");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].original_text, "alice@corp.com");
    assert_eq!((spans[0].byte_start, spans[0].byte_end), (8, 22));
    assert_eq!(spans[0].action, Action::Redact);
    assert_eq!(store.pool_records().count().await.expect("可计数"), 1);
}

#[rstest]
#[tokio::test]
async fn released_content_is_recorded_alongside_redacted_content() {
    let database = TempDatabase::new();
    let store = database.open().await;

    let mut released = span(0, 17, "user@example.com");
    released.action = Action::Release;
    let id = store
        .pool_records()
        .record(&recording(vec![released]))
        .await
        .expect("可写入");

    let spans = store.pool_records().spans_of(&id).await.expect("可查询");
    // 放行的内容同样入库：控制台要能看到全部判定，而不只是被抹去的部分。
    assert_eq!(spans[0].action, Action::Release);
    assert_eq!(spans[0].original_text, "user@example.com");
}

#[rstest]
#[tokio::test]
async fn identical_content_aggregates_into_one_row_with_its_categories_as_evidence() {
    let database = TempDatabase::new();
    let store = database.open().await;
    let pool = store.pool_records();

    // 同一段文本横跨三个 turn，被认成过两个类别，判定一次放行、两次抹去。
    pool.record(&turn(
        1,
        "ACME-9",
        vec![occurrence(
            EntityGroup::PrivatePerson,
            0.71,
            "ACME-9",
            Action::Release,
        )],
    ))
    .await
    .expect("可写入");
    pool.record(&turn(
        2,
        "ACME-9",
        vec![occurrence(
            EntityGroup::PrivateEmail,
            0.94,
            "ACME-9",
            Action::Redact,
        )],
    ))
    .await
    .expect("可写入");
    pool.record(&turn(
        3,
        "ACME-9",
        vec![occurrence(
            EntityGroup::PrivateEmail,
            0.88,
            "ACME-9",
            Action::Redact,
        )],
    ))
    .await
    .expect("可写入");

    let rows = pool
        .content_page(ContentFilter::All, &[], 10, 0)
        .await
        .expect("可查询");
    assert_eq!(rows.len(), 1, "同一段文本只有一行：{rows:?}");
    let row = &rows[0];
    assert_eq!(row.original_text, "ACME-9");
    assert_eq!(row.occurrences, 3);
    assert_eq!(row.turns, 3);
    assert_eq!((row.released, row.redacted), (1, 2));
    assert_eq!((row.score_min, row.score_max), (0.71, 0.94));
    assert!(row.first_seen <= row.last_seen);
    assert_eq!(row.latest_action, Action::Redact);
    assert_eq!(
        pool.content_count(ContentFilter::All, &[])
            .await
            .expect("可计数"),
        1
    );

    // 类别不是聚合身份，但它是这一行为什么摇摆的证据。
    assert_eq!(row.categories.len(), 2, "被认成过两个类别：{row:?}");
    assert_eq!(row.categories[0].entity_group, EntityGroup::PrivateEmail);
    assert_eq!(row.categories[0].count, 2);
    assert_eq!(row.categories[1].entity_group, EntityGroup::PrivatePerson);
    assert_eq!(row.categories[1].count, 1);
}

/// 复核过滤按「这段内容有没有被登记过」分：登记过的离开待审列表，进入已审列表。
///
/// 登记过的原文由调用方给出——「哪段内容审过」只有一个答案，它属于规则集合。
#[rstest]
#[tokio::test]
async fn the_review_filter_separates_registered_content_from_what_still_needs_a_decision() {
    let database = TempDatabase::new();
    let store = database.open().await;
    let pool = store.pool_records();

    for (index, text, action) in [
        (1, "decided", Action::Redact),
        (2, "waiting", Action::Release),
    ] {
        pool.record(&turn(
            index,
            text,
            vec![occurrence(EntityGroup::PrivateEmail, 0.9, text, action)],
        ))
        .await
        .expect("可写入");
    }

    let registered = vec!["decided".to_owned()];
    let texts = |rows: Vec<privacy_store::ContentSummary>| {
        rows.into_iter()
            .map(|row| row.original_text)
            .collect::<Vec<_>>()
    };

    assert_eq!(
        texts(
            pool.content_page(ContentFilter::Unreviewed, &registered, 10, 0)
                .await
                .expect("可查询")
        ),
        ["waiting"],
        "待审列表只留还没登记过的内容"
    );
    assert_eq!(
        texts(
            pool.content_page(ContentFilter::Reviewed, &registered, 10, 0)
                .await
                .expect("可查询")
        ),
        ["decided"],
        "已审列表只留登记过的内容"
    );
    assert_eq!(
        pool.content_count(ContentFilter::Unreviewed, &registered)
            .await
            .expect("可计数"),
        1
    );
    assert_eq!(
        pool.content_count(ContentFilter::Reviewed, &registered)
            .await
            .expect("可计数"),
        1
    );
    assert_eq!(
        pool.content_count(ContentFilter::All, &registered)
            .await
            .expect("可计数"),
        2
    );

    // 一条登记都没有时：已审是空集，未审是全集。空列表写进 IN 会把整段条件判成空集。
    assert_eq!(
        pool.content_count(ContentFilter::Reviewed, &[])
            .await
            .expect("可计数"),
        0
    );
    assert_eq!(
        pool.content_count(ContentFilter::Unreviewed, &[])
            .await
            .expect("可计数"),
        2
    );
}

#[rstest]
#[tokio::test]
async fn content_is_ordered_by_instability_then_occurrences() {
    let database = TempDatabase::new();
    let store = database.open().await;
    let pool = store.pool_records();

    for (index, text, action) in [
        (1, "flips-twice", Action::Release),
        (2, "flips-twice", Action::Redact),
        (3, "flips-twice", Action::Release),
        (4, "flips-once", Action::Release),
        (5, "flips-once", Action::Redact),
        (6, "seen-often", Action::Redact),
        (7, "seen-often", Action::Redact),
        (8, "seen-often", Action::Redact),
        (9, "seen-often", Action::Redact),
    ] {
        pool.record(&turn(
            index,
            text,
            vec![occurrence(EntityGroup::PrivateEmail, 0.9, text, action)],
        ))
        .await
        .expect("可写入");
    }

    let rows = pool
        .content_page(ContentFilter::All, &[], 10, 0)
        .await
        .expect("可查询");
    assert_eq!(
        rows.iter()
            .map(|row| row.original_text.as_str())
            .collect::<Vec<_>>(),
        ["flips-twice", "flips-once", "seen-often"],
        "判定摇摆的排在前面，其次才是出现得多的"
    );

    // 分页不重不漏：跳过第一行之后拿到剩下的两行。
    let page = pool
        .content_page(ContentFilter::All, &[], 2, 1)
        .await
        .expect("可查询");
    assert_eq!(
        page.iter()
            .map(|row| row.original_text.as_str())
            .collect::<Vec<_>>(),
        ["flips-once", "seen-often"]
    );
}

#[rstest]
#[tokio::test]
async fn occurrences_of_a_text_come_back_with_their_provenance() {
    let database = TempDatabase::new();
    let store = database.open().await;
    let pool = store.pool_records();

    for (index, action) in [(1, Action::Release), (2, Action::Redact)] {
        pool.record(&turn(
            index,
            "ACME-9",
            vec![occurrence(EntityGroup::PrivateEmail, 0.9, "ACME-9", action)],
        ))
        .await
        .expect("可写入");
    }
    // 另一段内容不该混进来。
    pool.record(&turn(
        3,
        "OTHER",
        vec![occurrence(
            EntityGroup::PrivateEmail,
            0.9,
            "OTHER",
            Action::Redact,
        )],
    ))
    .await
    .expect("可写入");

    let occurrences = pool.occurrences_of("ACME-9").await.expect("可查询");
    assert_eq!(occurrences.len(), 2);
    assert!(
        occurrences
            .windows(2)
            .all(|pair| pair[0].created_at >= pair[1].created_at),
        "最近的出现在最前"
    );
    assert_eq!(occurrences[0].api_format, ApiFormat::OpenAiChat);
    assert_eq!(occurrences[0].path, "/v1/chat/completions");
    assert!(
        occurrences
            .iter()
            .all(|item| item.entity_group == EntityGroup::PrivateEmail)
    );
}

#[rstest]
#[tokio::test]
async fn deleting_a_fragment_removes_its_spans() {
    let database = TempDatabase::new();
    let store = database.open().await;
    let id = store
        .pool_records()
        .record(&recording(vec![span(0, 3, "abc")]))
        .await
        .expect("可写入");

    sqlx::query("DELETE FROM fragments WHERE id = ?1")
        .bind(&id)
        .execute(store.pool())
        .await
        .expect("可删除");

    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM spans")
        .fetch_one(store.pool())
        .await
        .expect("可计数");
    assert_eq!(remaining, 0, "片段删除必须级联清除命中记录");
}

#[rstest]
#[tokio::test]
async fn deleting_a_rule_keeps_the_audit_trail_but_clears_the_reference() {
    let database = TempDatabase::new();
    let store = database.open().await;

    let rule = privacy_rules::Rule {
        id: privacy_rules::RuleId::new("console.tmp").expect("非空"),
        name: "临时规则".to_owned(),
        priority: 10,
        condition: privacy_rules::RuleExpression::all([]),
        action: Action::Redact,
        enabled: true,
        source: privacy_rules::RuleSource::Console,
        category: None,
    };
    store.rules().create(&rule).await.expect("可创建");

    let mut recorded = span(0, 3, "abc");
    recorded.matched_rule_id = Some(rule.id.to_string());
    let id = store
        .pool_records()
        .record(&recording(vec![recorded]))
        .await
        .expect("可写入");

    store.rules().delete(&rule.id).await.expect("可删除");

    let spans = store.pool_records().spans_of(&id).await.expect("可查询");
    assert_eq!(spans.len(), 1, "审计记录不随规则删除而消失");
    assert_eq!(spans[0].matched_rule_id, None, "引用被置空");
}

#[rstest]
#[tokio::test]
async fn spans_cannot_be_written_without_a_fragment() {
    let database = TempDatabase::new();
    let store = database.open().await;

    let error = sqlx::query(
        "INSERT INTO spans (id, fragment_id, entity_group, score, char_len, byte_start, byte_end, original_text, action, created_at)
         VALUES ('orphan', 'no-such-fragment', 'secret', 0.9, 3, 0, 3, 'abc', 'redact', 0)",
    )
    .execute(store.pool())
    .await;

    assert!(error.is_err(), "孤立命中必须被外键拒绝");
}

/// 从一条命中读回它所属的 turn：原文、转发出去的内容，以及这次的全部命中。
#[rstest]
#[tokio::test]
async fn a_detection_reads_back_the_turn_it_belongs_to() {
    let database = TempDatabase::new();
    let store = database.open().await;
    let pool = store.pool_records();

    let id = pool
        .record(&recording(vec![span(8, 22, "alice@corp.com")]))
        .await
        .expect("可写入");
    let span_id = pool
        .spans_of(&id)
        .await
        .expect("可查询")
        .first()
        .expect("有命中")
        .id
        .clone();

    let detail = pool
        .fragment_of_span(&span_id)
        .await
        .expect("可查询")
        .expect("命中存在");
    assert_eq!(detail.fragment.id, id);
    assert_eq!(detail.fragment.original_text, "contact alice@corp.com now");
    assert_eq!(
        detail.fragment.redacted_text,
        "contact <redacted_private_email> now"
    );
    assert_eq!(detail.spans.len(), 1);
    assert_eq!(detail.spans[0].original_text, "alice@corp.com");

    assert_eq!(
        pool.fragment_of_span("no-such-span").await.expect("可查询"),
        None,
        "不存在的命中没有 turn 可读"
    );
}
