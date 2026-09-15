mod common;

use common::{entity, rule};
use privacy_filter::{Entity, EntityGroup};
use privacy_rules::{Action, Rule, RuleExpression, RuleSet};
use rstest::rstest;

#[rstest]
fn no_matching_rule_releases_the_entity(entity: impl Fn(EntityGroup, f64, &str) -> Entity) {
    let rules = RuleSet::new([]);
    let decision = rules.evaluate(&entity(EntityGroup::PrivateEmail, 0.99, "a@real.com"));

    assert_eq!(decision.action, Action::Release);
    assert!(decision.rule.is_none());
    assert_eq!(rules.fallback(), Action::Release);
}

#[rstest]
fn fallback_can_be_overridden_explicitly(entity: impl Fn(EntityGroup, f64, &str) -> Entity) {
    let rules = RuleSet::new([]).with_fallback(Action::Redact);
    let decision = rules.evaluate(&entity(EntityGroup::PrivateEmail, 0.99, "a@real.com"));

    assert_eq!(decision.action, Action::Redact);
    assert!(decision.rule.is_none());
}

#[rstest]
fn higher_priority_wins_regardless_of_construction_order(
    rule: impl Fn(&str, i32, RuleExpression, Action) -> Rule,
    entity: impl Fn(EntityGroup, f64, &str) -> Entity,
) {
    let redact = rule("redact", 1000, RuleExpression::all([]), Action::Redact);
    let release = rule("release", 10, RuleExpression::all([]), Action::Release);
    let candidate = entity(EntityGroup::PrivateEmail, 0.99, "a@real.com");

    let forward = RuleSet::new([release.clone(), redact.clone()]);
    let backward = RuleSet::new([redact, release]);

    assert_eq!(forward.evaluate(&candidate).action, Action::Redact);
    assert_eq!(backward.evaluate(&candidate).action, Action::Redact);
    assert_eq!(
        forward.evaluate(&candidate).rule.map(|id| id.to_string()),
        Some("redact".to_owned())
    );
}

#[rstest]
fn equal_priority_is_broken_deterministically_by_id(
    rule: impl Fn(&str, i32, RuleExpression, Action) -> Rule,
    entity: impl Fn(EntityGroup, f64, &str) -> Entity,
) {
    let a = rule("a-rule", 100, RuleExpression::all([]), Action::Release);
    let b = rule("b-rule", 100, RuleExpression::all([]), Action::Redact);
    let candidate = entity(EntityGroup::PrivateEmail, 0.99, "a@real.com");

    // 判定结果必须与构造顺序无关，否则同一份配置在不同进程会给出不同结论。
    assert_eq!(
        RuleSet::new([a.clone(), b.clone()])
            .evaluate(&candidate)
            .action,
        Action::Release
    );
    assert_eq!(
        RuleSet::new([b, a]).evaluate(&candidate).action,
        Action::Release
    );
}

#[rstest]
fn disabled_rules_are_skipped(
    rule: impl Fn(&str, i32, RuleExpression, Action) -> Rule,
    entity: impl Fn(EntityGroup, f64, &str) -> Entity,
) {
    let mut disabled = rule("disabled", 1000, RuleExpression::all([]), Action::Redact);
    disabled.enabled = false;
    let rules = RuleSet::new([disabled]);
    let candidate = entity(EntityGroup::PrivateEmail, 0.99, "a@real.com");

    // 停用后回到兜底，而不是继续生效。
    assert_eq!(rules.evaluate(&candidate).action, Action::Release);
    assert!(rules.evaluate(&candidate).rule.is_none());
}

#[rstest]
fn first_matching_rule_wins_even_when_a_later_one_also_matches(
    rule: impl Fn(&str, i32, RuleExpression, Action) -> Rule,
    entity: impl Fn(EntityGroup, f64, &str) -> Entity,
) {
    let specific = rule(
        "secrets",
        1000,
        RuleExpression::entity(EntityGroup::Secret),
        Action::Redact,
    );
    let catch_all = rule("release-all", 10, RuleExpression::all([]), Action::Release);
    let rules = RuleSet::new([specific, catch_all]);

    let secret = rules.evaluate(&entity(EntityGroup::Secret, 0.99, "sk-live-123"));
    assert_eq!(secret.action, Action::Redact);
    assert_eq!(
        secret.rule.map(|id| id.to_string()),
        Some("secrets".to_owned())
    );

    // 未命中高优先级规则时，低优先级规则仍然生效。
    let email = rules.evaluate(&entity(EntityGroup::PrivateEmail, 0.99, "a@real.com"));
    assert_eq!(email.action, Action::Release);
    assert_eq!(
        email.rule.map(|id| id.to_string()),
        Some("release-all".to_owned())
    );
}

#[rstest]
fn decisions_expose_whether_content_is_released(
    rule: impl Fn(&str, i32, RuleExpression, Action) -> Rule,
    entity: impl Fn(EntityGroup, f64, &str) -> Entity,
) {
    let rules = RuleSet::new([rule(
        "release",
        10,
        RuleExpression::all([]),
        Action::Release,
    )]);
    assert!(
        rules
            .evaluate(&entity(EntityGroup::PrivateEmail, 0.9, "x"))
            .is_release()
    );

    let rules = RuleSet::new([]).with_fallback(Action::Redact);
    assert!(
        !rules
            .evaluate(&entity(EntityGroup::PrivateEmail, 0.9, "x"))
            .is_release()
    );
}

#[rstest]
fn rules_are_exposed_in_evaluation_order(rule: impl Fn(&str, i32, RuleExpression, Action) -> Rule) {
    let rules = RuleSet::new([
        rule("low", 1, RuleExpression::all([]), Action::Release),
        rule("high", 9, RuleExpression::all([]), Action::Release),
        rule("mid", 5, RuleExpression::all([]), Action::Release),
    ]);

    let order: Vec<&str> = rules.rules().iter().map(|r| r.id.as_str()).collect();
    assert_eq!(order, ["high", "mid", "low"]);
}
