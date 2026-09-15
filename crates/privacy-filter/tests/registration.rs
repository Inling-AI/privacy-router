//! 登记的识别契约：登记的原文出现几次就命中几次，得分是常数，位置能直接切出原文。

use privacy_filter::{EntityGroup, Filter, PatternFilter, Registration};
use rstest::rstest;

/// 命中位置与得分：每个出现处各自成一条命中。
#[rstest]
#[case("api_key=ah-1f2e\nah-1f2e\n", "ah-1f2e", 2)]
#[case("只出现一次 ah-1f2e", "ah-1f2e", 1)]
#[case("换个上下文 ah-1f2e 与 ah-1f2e2", "ah-1f2e", 2)]
fn every_occurrence_becomes_a_hit(
    #[case] text: &str,
    #[case] needle: &str,
    #[case] expected: usize,
) {
    let registration = Registration::new(needle, EntityGroup::Secret, false);

    let hits = registration.recognize(text);

    assert_eq!(hits.len(), expected);
    for hit in &hits {
        assert_eq!(hit.entity_group, EntityGroup::Secret);
        assert_eq!(hit.score, PatternFilter::SCORE);
        assert_eq!(&text[hit.start..hit.end], needle);
        assert_eq!(hit.word, needle);
    }
}

/// 默认不区分大小写：登记的是内容，不是它当时的书写。
#[rstest]
fn case_is_folded_unless_the_registration_asks_for_exact_case() {
    let folded = Registration::new("TimmyOVO", EntityGroup::PrivatePerson, false);
    let exact = Registration::new("TimmyOVO", EntityGroup::PrivatePerson, true);

    assert_eq!(folded.recognize("登录用户 timmyovo 已就位").len(), 1);
    assert!(exact.recognize("登录用户 timmyovo 已就位").is_empty());
    assert_eq!(exact.recognize("登录用户 TimmyOVO 已就位").len(), 1);
}

/// 登记不要求两侧是边界：值被写在 `key=` 后面同样算命中，命中范围仍然只盖住登记的那段。
#[rstest]
fn an_occurrence_inside_a_longer_token_is_still_a_hit() {
    let registration = Registration::new("ah-1f2e", EntityGroup::Secret, false);

    let hits = registration.recognize("smtp_password=ah-1f2e&other=1");

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].word, "ah-1f2e");
}
