//! 枚举取值与数据库 `CHECK` 约束的一致性。
//!
//! 存储层不维护第二份字符串表：写入的取值来自 serde 派生的名称，而约束是一段 SQL 字面量。
//! 两者一旦漂移，写入会在运行时报错。本文件为每个枚举的每个取值写入一行真实数据，把这种
//! 漂移固定在测试阶段。

mod common;

use common::TempDatabase;
use privacy_rules::{Action, EntityGroup, Rule, RuleId, RuleSource};
use privacy_store::{ApiFormat, FragmentRecording, ProviderDraft, RecordedSpan};
use rstest::rstest;

const ALL_API_FORMATS: [ApiFormat; 3] = [
    ApiFormat::OpenAiResponses,
    ApiFormat::OpenAiChat,
    ApiFormat::AnthropicMessages,
];

const ALL_ACTIONS: [Action; 2] = [Action::Release, Action::Redact];

const ALL_RULE_SOURCES: [RuleSource; 3] = [
    RuleSource::Builtin,
    RuleSource::Console,
    RuleSource::Operator,
];

#[rstest]
#[tokio::test]
async fn every_api_format_is_accepted_by_the_schema() {
    let database = TempDatabase::new();
    let store = database.open().await;

    for (index, api_format) in ALL_API_FORMATS.into_iter().enumerate() {
        // 取值覆盖是重点，名称只需唯一。
        let draft = ProviderDraft {
            name: format!("provider-{index}"),
            base_url: "https://upstream.test".to_owned(),
            api_format,
            enabled: true,
        };
        let created = store
            .providers()
            .create(&draft)
            .await
            .unwrap_or_else(|error| panic!("{api_format:?} 被 schema 拒绝：{error}"));

        // 读回来必须与写入时完全相同，否则枚举与列的映射是单向的。
        let reloaded = store
            .providers()
            .find(&created.id)
            .await
            .expect("可查询")
            .expect("存在");
        assert_eq!(reloaded.api_format, api_format);
    }
}

#[rstest]
#[tokio::test]
async fn every_action_and_entity_group_is_accepted_by_the_schema() {
    let database = TempDatabase::new();
    let store = database.open().await;

    for action in ALL_ACTIONS {
        // 类别清单来自类型本身；新增类别时这个测试自动覆盖，不需要第二份名单。
        for entity_group in EntityGroup::all() {
            let recording = FragmentRecording {
                request_id: format!("req-{action:?}-{entity_group}"),
                api_format: ApiFormat::OpenAiChat,
                path: "/v1/chat/completions".to_owned(),
                content_hash: vec![0],
                original_text: "x".to_owned(),
                redacted_text: "y".to_owned(),
                spans: vec![RecordedSpan {
                    id: String::new(),
                    entity_group,
                    score: 0.5,
                    char_len: 1,
                    byte_start: 0,
                    byte_end: 1,
                    original_text: "x".to_owned(),
                    action,
                    matched_rule_id: None,
                }],
            };

            let fragment = store
                .pool_records()
                .record(&recording)
                .await
                .unwrap_or_else(|error| {
                    panic!("{action:?}/{entity_group:?} 被 schema 拒绝：{error}")
                });

            let spans = store
                .pool_records()
                .spans_of(&fragment)
                .await
                .expect("可查询");
            assert_eq!(spans[0].action, action);
            assert_eq!(spans[0].entity_group, entity_group);
        }
    }
}

#[rstest]
#[tokio::test]
async fn every_rule_source_is_accepted_by_the_schema() {
    let database = TempDatabase::new();
    let store = database.open().await;

    for source in ALL_RULE_SOURCES {
        let rule = Rule {
            id: RuleId::new(format!("vocabulary.{source:?}")).expect("非空"),
            name: format!("{source:?}"),
            priority: 1,
            condition: privacy_rules::RuleExpression::all([]),
            action: Action::Redact,
            enabled: true,
            source,
            // 枚举取值的重点是「每个来源都能落库」；类别是 Operator 登记的必填项，
            // 这里给一个取值即可，与内置/控制台规则无关。
            category: (source == RuleSource::Operator).then_some(EntityGroup::Secret),
        };

        store
            .rules()
            .create(&rule)
            .await
            .unwrap_or_else(|error| panic!("{source:?} 被 schema 拒绝：{error}"));

        let reloaded = store
            .rules()
            .find(&rule.id)
            .await
            .expect("可查询")
            .expect("存在");
        assert_eq!(reloaded.source, source);
    }
}
