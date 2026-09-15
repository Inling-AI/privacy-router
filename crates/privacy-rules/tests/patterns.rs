use privacy_rules::{Pattern, PatternKind};
use rstest::rstest;

#[rstest]
#[case("hello", "hello", PatternKind::Exact, false, true)]
#[case("HELLO", "hello", PatternKind::Exact, false, true)]
#[case("HELLO", "hello", PatternKind::Exact, true, false)]
#[case("user@example.com", "example.com", PatternKind::Substring, false, true)]
#[case("user@example.com", "*@example.?om", PatternKind::Glob, false, true)]
#[case("李雷", "??", PatternKind::Glob, false, true)]
#[case("李雷", "?", PatternKind::Glob, false, false)]
fn pattern_modes_match_as_declared(
    #[case] text: &str,
    #[case] needle: &str,
    #[case] kind: PatternKind,
    #[case] case_sensitive: bool,
    #[case] expected: bool,
) {
    let pattern = Pattern {
        text: needle.to_owned(),
        kind,
        case_sensitive,
    };
    assert_eq!(pattern.matches(text), expected);
}
