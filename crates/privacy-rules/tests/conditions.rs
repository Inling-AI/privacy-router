mod common;

use common::entity;
use privacy_filter::EntityGroup;
use privacy_rules::{ComparisonOperator, Pattern, RuleExpression, RuleFilter};
use rstest::rstest;

#[rstest]
#[case(ComparisonOperator::Greater, 0.8, 0.7, true)]
#[case(ComparisonOperator::Greater, 0.7, 0.7, false)]
#[case(ComparisonOperator::Equal, 0.7, 0.7, true)]
#[case(ComparisonOperator::Less, 0.6, 0.7, true)]
#[case(ComparisonOperator::Less, 0.7, 0.7, false)]
fn confidence_filter_applies_strict_comparison(
    entity: impl Fn(EntityGroup, f64, &str) -> privacy_filter::Entity,
    #[case] operator: ComparisonOperator,
    #[case] score: f64,
    #[case] value: f64,
    #[case] expected: bool,
) {
    let filter = RuleFilter::Confidence { operator, value };
    assert_eq!(
        filter.matches(&entity(EntityGroup::PrivateEmail, score, "x")),
        expected
    );
}

#[rstest]
#[case(ComparisonOperator::Greater, "李雷", 1, true)]
#[case(ComparisonOperator::Equal, "李雷", 2, true)]
#[case(ComparisonOperator::Less, "李雷", 3, true)]
fn character_length_filter_counts_characters(
    entity: impl Fn(EntityGroup, f64, &str) -> privacy_filter::Entity,
    #[case] operator: ComparisonOperator,
    #[case] text: &str,
    #[case] value: usize,
    #[case] expected: bool,
) {
    let filter = RuleFilter::CharacterLength { operator, value };
    assert_eq!(
        filter.matches(&entity(EntityGroup::PrivatePerson, 0.9, text)),
        expected
    );
}

#[rstest]
fn all_requires_every_child(entity: impl Fn(EntityGroup, f64, &str) -> privacy_filter::Entity) {
    let expression = RuleExpression::all([
        RuleExpression::entity(EntityGroup::PrivateEmail),
        RuleExpression::keyword(Pattern::substring("example.com")),
    ]);

    assert!(expression.matches(&entity(EntityGroup::PrivateEmail, 0.9, "a@example.com")));
    assert!(!expression.matches(&entity(EntityGroup::PrivatePerson, 0.9, "a@example.com")));
}

#[rstest]
fn any_accepts_either_child(entity: impl Fn(EntityGroup, f64, &str) -> privacy_filter::Entity) {
    let expression = RuleExpression::any([
        RuleExpression::entity(EntityGroup::Secret),
        RuleExpression::confidence(ComparisonOperator::Less, 0.5),
    ]);

    assert!(expression.matches(&entity(EntityGroup::Secret, 0.9, "x")));
    assert!(expression.matches(&entity(EntityGroup::PrivateEmail, 0.4, "x")));
    assert!(!expression.matches(&entity(EntityGroup::PrivateEmail, 0.9, "x")));
}

#[rstest]
fn logical_groups_can_nest_arbitrarily(
    entity: impl Fn(EntityGroup, f64, &str) -> privacy_filter::Entity,
) {
    let expression = RuleExpression::any([
        RuleExpression::all([
            RuleExpression::entity(EntityGroup::PrivateEmail),
            RuleExpression::confidence(ComparisonOperator::Greater, 0.8),
        ]),
        RuleExpression::all([
            RuleExpression::entity(EntityGroup::PrivatePerson),
            RuleExpression::character_length(ComparisonOperator::Less, 3),
        ]),
    ]);

    assert!(expression.matches(&entity(EntityGroup::PrivateEmail, 0.9, "long@example.com")));
    assert!(expression.matches(&entity(EntityGroup::PrivatePerson, 0.9, "李雷")));
    assert!(!expression.matches(&entity(EntityGroup::PrivatePerson, 0.9, "Harry")));
}
