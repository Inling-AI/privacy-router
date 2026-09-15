mod common;

use common::entity;
use privacy_filter::EntityGroup;
use privacy_rules::{Action, RuleSet};
use rstest::rstest;
use std::collections::HashSet;

/// 每一类受保护实体在「足够可信 + 足够长」时都必须被抹去，且确实有规则认领这次判定。
#[rstest]
#[case(EntityGroup::AccountNumber, "1234")]
#[case(EntityGroup::BankCard, "6222021234567890")]
#[case(EntityGroup::CreditCode, "91350100M000100Y43")]
#[case(EntityGroup::IdCard, "11010119900307721X")]
#[case(EntityGroup::IpAddress, "10.0.0.1")]
#[case(EntityGroup::MacAddress, "aa:bb:cc:dd:ee:ff")]
#[case(EntityGroup::PrivateAddress, "Main St")]
#[case(EntityGroup::PrivateEmail, "a@b.co")]
#[case(EntityGroup::PrivatePerson, "Alice")]
#[case(EntityGroup::PrivatePhone, "5551234")]
#[case(EntityGroup::PrivateUrl, "a.co/x")]
#[case(EntityGroup::Secret, "sk-live1")]
fn every_protected_group_is_redacted_when_confident_and_plausible(
    entity: impl Fn(EntityGroup, f64, &str) -> privacy_filter::Entity,
    #[case] group: EntityGroup,
    #[case] text: &str,
) {
    let decision = RuleSet::builtin().evaluate(&entity(group, 0.81, text));
    assert_eq!(decision.action, Action::Redact);
    assert!(decision.rule.is_some());
}

/// 概率类别仍然要过置信度门槛：模型的命中是估计，不是事实，门槛是严格大于。
///
/// 手机号属于概率类别：确定性的国内移动号段过滤器只覆盖一部分写法，模型在该类别上继续发言，
/// 因此这里连同长度门槛一起保留。
#[rstest]
#[case(EntityGroup::AccountNumber, "1234")]
#[case(EntityGroup::PrivateAddress, "Main St")]
#[case(EntityGroup::PrivatePerson, "Alice")]
#[case(EntityGroup::PrivatePhone, "5551234")]
#[case(EntityGroup::PrivateUrl, "a.co/x")]
#[case(EntityGroup::Secret, "sk-live1")]
fn probabilistic_categories_still_ask_for_confidence(
    entity: impl Fn(EntityGroup, f64, &str) -> privacy_filter::Entity,
    #[case] group: EntityGroup,
    #[case] text: &str,
) {
    let rules = RuleSet::builtin();
    for score in [0.79, 0.8] {
        let decision = rules.evaluate(&entity(group, score, text));
        assert!(decision.rule.is_none(), "{group} 在 {score} 上不得认领命中");
        assert_eq!(decision.action, Action::Release);
    }
    assert!(
        rules.evaluate(&entity(group, 0.81, text)).rule.is_some(),
        "{group} 必须认领足够可信的命中"
    );
}

/// 日期不设预置档位：没有任何内置规则认领它，是否抹去由兜底策略与管理员规则决定。
#[rstest]
fn no_builtin_rule_claims_a_date(
    entity: impl Fn(EntityGroup, f64, &str) -> privacy_filter::Entity,
) {
    let decision =
        RuleSet::builtin().evaluate(&entity(EntityGroup::PrivateDate, 0.99, "2026-09-11"));
    assert!(decision.rule.is_none());
    assert_eq!(decision.action, Action::Release);
}

/// 长度不够的命中不认领任何规则，直接落到兜底策略；所以不需要「短匹配放行」反向规则。
#[rstest]
fn implausibly_short_entities_fall_through_to_the_fallback(
    entity: impl Fn(EntityGroup, f64, &str) -> privacy_filter::Entity,
) {
    let decision = RuleSet::builtin().evaluate(&entity(EntityGroup::PrivateUrl, 0.99, "a.co"));
    assert!(decision.rule.is_none());
    assert_eq!(decision.action, Action::Release);
}

/// 姓名的门槛是「字符数大于 4」：四个字符的命中不算数，五个字符才拦截。
#[rstest]
fn the_name_floor_starts_above_four_characters(
    entity: impl Fn(EntityGroup, f64, &str) -> privacy_filter::Entity,
) {
    let rules = RuleSet::builtin();
    assert!(
        rules
            .evaluate(&entity(EntityGroup::PrivatePerson, 0.99, "Alex"))
            .rule
            .is_none(),
        "四个字符不足以认领姓名拦截"
    );
    assert!(
        rules
            .evaluate(&entity(EntityGroup::PrivatePerson, 0.99, "Alice"))
            .rule
            .is_some()
    );
}

/// 形状由确定性过滤器定义的类别只问「是不是这个类别」：命中是事实，没有置信度可言，也不看长度。
///
/// 过滤器给这些命中写的得分是常数 1.0，规则再比一次 0.8 只是把同一个常数重算一遍；长度同理，
/// 形状能产出就说明已经完整。所以这里用一个字符、零分的命中把两个门槛都钉掉。
#[rstest]
#[case(EntityGroup::BankCard)]
#[case(EntityGroup::CreditCode)]
#[case(EntityGroup::IdCard)]
#[case(EntityGroup::IpAddress)]
#[case(EntityGroup::MacAddress)]
#[case(EntityGroup::PrivateEmail)]
fn shape_defined_categories_are_claimed_by_category_alone(
    entity: impl Fn(EntityGroup, f64, &str) -> privacy_filter::Entity,
    #[case] group: EntityGroup,
) {
    let decision = RuleSet::builtin().evaluate(&entity(group, 0.0, "1"));
    assert_eq!(decision.action, Action::Redact, "{group} 只由类别认领");
    assert!(decision.rule.is_some());
}

#[rstest]
fn disabling_a_default_rule_releases_its_category(
    entity: impl Fn(EntityGroup, f64, &str) -> privacy_filter::Entity,
) {
    let mut defaults = RuleSet::builtin().rules().to_vec();
    for rule in &mut defaults {
        rule.enabled = false;
    }
    let decision = RuleSet::new(defaults).evaluate(&entity(
        EntityGroup::PrivateEmail,
        0.99,
        "alice@example.com",
    ));
    assert_eq!(decision.action, Action::Release);
}

#[rstest]
fn deleting_a_default_rule_releases_only_its_category(
    entity: impl Fn(EntityGroup, f64, &str) -> privacy_filter::Entity,
) {
    let candidate = entity(EntityGroup::PrivateEmail, 0.99, "alice@example.com");
    let defaults = RuleSet::builtin();
    let matched = defaults.evaluate(&candidate).rule.unwrap();
    let rules = RuleSet::new(
        defaults
            .rules()
            .iter()
            .filter(|rule| rule.id != matched)
            .cloned(),
    );
    assert_eq!(rules.evaluate(&candidate).action, Action::Release);
    assert_eq!(
        rules
            .evaluate(&entity(EntityGroup::Secret, 0.99, "secret1234"))
            .action,
        Action::Redact
    );
}

/// 预置规则集只做拦截：每个受保护类别一条，日期没有档位，因此比类别总数少一条。
#[rstest]
fn builtin_set_holds_one_rule_per_protected_group_and_is_valid() {
    let rules = RuleSet::builtin();
    let ids: HashSet<&str> = rules.rules().iter().map(|rule| rule.id.as_str()).collect();

    assert_eq!(rules.rules().len(), EntityGroup::all().len() - 1);
    assert_eq!(ids.len(), rules.rules().len());
    for rule in rules.rules() {
        rule.validate().expect("内置规则必须有效");
        assert!(rule.enabled);
        assert_eq!(rule.action, Action::Redact);
    }
}
