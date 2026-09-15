mod common;

use common::rule;
use privacy_rules::{Action, ComparisonOperator, Error, Pattern, RuleExpression};
use rstest::rstest;

#[rstest]
#[case("")]
#[case("   ")]
fn empty_keyword_filters_are_rejected_at_any_depth(
    rule: impl Fn(&str, i32, RuleExpression, Action) -> privacy_rules::Rule,
    #[case] text: &str,
) {
    let invalid = rule(
        "empty",
        1,
        RuleExpression::all([RuleExpression::any([RuleExpression::keyword(
            Pattern::exact(text),
        )])]),
        Action::Release,
    );
    assert!(matches!(invalid.validate(), Err(Error::EmptyPattern(_))));
}

#[rstest]
#[case(f64::NAN)]
#[case(-0.1)]
#[case(1.1)]
fn confidence_must_be_finite_probability(
    rule: impl Fn(&str, i32, RuleExpression, Action) -> privacy_rules::Rule,
    #[case] value: f64,
) {
    let invalid = rule(
        "confidence",
        1,
        RuleExpression::confidence(ComparisonOperator::Greater, value),
        Action::Redact,
    );
    assert!(matches!(
        invalid.validate(),
        Err(Error::InvalidConfidence { .. })
    ));
}

#[rstest]
fn valid_nested_expression_passes(
    rule: impl Fn(&str, i32, RuleExpression, Action) -> privacy_rules::Rule,
) {
    let valid = rule(
        "valid",
        1,
        RuleExpression::any([
            RuleExpression::keyword(Pattern::substring("example.com")),
            RuleExpression::character_length(ComparisonOperator::Greater, 8),
        ]),
        Action::Release,
    );
    valid.validate().expect("合法表达式应通过");
}
