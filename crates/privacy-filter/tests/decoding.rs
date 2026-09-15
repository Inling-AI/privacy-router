use privacy_filter::{
    Boundary, Decoding, EntityGroup, Error, ModelLabel, TokenLabel, TokenPrediction,
};
use rstest::{fixture, rstest};
use std::ops::Range;

#[fixture]
fn person_token() -> impl Fn(Boundary, Range<usize>) -> TokenPrediction {
    |boundary, offsets| {
        let mut logits = [-10.0; TokenLabel::COUNT];
        logits[TokenLabel::Entity {
            group: ModelLabel::PrivatePerson,
            boundary,
        }
        .id()] = 10.0;
        TokenPrediction { offsets, logits }
    }
}

#[rstest]
#[case(" Harry Potter", vec![(Boundary::Begin, 0..6), (Boundary::End, 6..13)], vec![" Harry Potter"])]
#[case("AliceBob", vec![(Boundary::Single, 0..5), (Boundary::Single, 5..8)], vec!["Alice", "Bob"])]
#[case(" 李雷", vec![(Boundary::Begin, 0..2), (Boundary::Inside, 2..4), (Boundary::End, 4..7)], vec![" 李雷"])]
fn aggregates_entities_without_losing_boundaries_or_text(
    person_token: impl Fn(Boundary, Range<usize>) -> TokenPrediction,
    #[case] text: &str,
    #[case] tokens: Vec<(Boundary, Range<usize>)>,
    #[case] expected: Vec<&str>,
    #[values(Decoding::Simple, Decoding::Viterbi)] decoding: Decoding,
) {
    let tokens: Vec<_> = tokens
        .into_iter()
        .map(|(boundary, range)| person_token(boundary, range))
        .collect();
    let entities = decoding.decode(text, &tokens).unwrap();
    assert_eq!(
        entities
            .iter()
            .map(|entity| entity.word.as_str())
            .collect::<Vec<_>>(),
        expected
    );
    assert!(
        entities
            .iter()
            .all(|entity| entity.entity_group == EntityGroup::PrivatePerson
                && entity.score > 0.99999)
    );
}

#[rstest]
fn averages_selected_token_probabilities(
    person_token: impl Fn(Boundary, Range<usize>) -> TokenPrediction,
) {
    let mut first = person_token(Boundary::Begin, 0..6);
    let mut last = person_token(Boundary::End, 6..13);
    first.logits[TokenLabel::Outside.id()] = 10.0;
    last.logits[TokenLabel::Outside.id()] = 9.0;
    let entities = Decoding::Viterbi
        .decode(" Harry Potter", &[first, last])
        .unwrap();
    // 两个 token 的所选标签概率分别约为 0.5 和 sigmoid(1)。
    let expected = (0.5 + 1.0 / (1.0 + (-1.0_f64).exp())) / 2.0;
    assert_eq!(entities.len(), 1);
    assert!((entities[0].score - expected).abs() < 1e-6);
}

#[rstest]
fn viterbi_recovers_a_complete_span_from_orphan_inside_label(
    person_token: impl Fn(Boundary, Range<usize>) -> TokenPrediction,
) {
    let mut first = person_token(Boundary::Inside, 0..6);
    first.logits[TokenLabel::Entity {
        group: ModelLabel::PrivatePerson,
        boundary: Boundary::Begin,
    }
    .id()] = 9.0;
    let last = person_token(Boundary::End, 6..13);
    let entities = Decoding::Viterbi
        .decode(" Harry Potter", &[first, last])
        .unwrap();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].word, " Harry Potter");
    assert!(entities[0].score > 0.63 && entities[0].score < 0.64);
}

#[rstest]
#[case(4..5)]
#[case(Range { start: 2, end: 1 })]
fn rejects_invalid_offsets(
    person_token: impl Fn(Boundary, Range<usize>) -> TokenPrediction,
    #[case] offsets: Range<usize>,
) {
    let error = Decoding::Simple
        .decode("abc", &[person_token(Boundary::Single, offsets)])
        .unwrap_err();
    assert!(matches!(error, Error::Inference(_)));
}

#[rstest]
fn rejects_non_finite_logits(person_token: impl Fn(Boundary, Range<usize>) -> TokenPrediction) {
    let mut token = person_token(Boundary::Single, 0..3);
    token.logits[0] = f32::NAN;
    let error = Decoding::Simple.decode("abc", &[token]).unwrap_err();
    assert!(matches!(error, Error::Inference(_)));
}

#[fixture]
fn outside_token() -> impl Fn(Range<usize>) -> TokenPrediction {
    |offsets| {
        let mut logits = [-10.0; TokenLabel::COUNT];
        logits[TokenLabel::Outside.id()] = 10.0;
        TokenPrediction { offsets, logits }
    }
}

#[rstest]
#[case(" Harry Potter", vec![(Boundary::Begin, 0..6), (Boundary::End, 6..13)])]
#[case(" 李雷", vec![(Boundary::Begin, 0..2), (Boundary::Inside, 2..4), (Boundary::End, 4..7)])]
fn reported_offsets_slice_back_to_the_reported_word(
    person_token: impl Fn(Boundary, Range<usize>) -> TokenPrediction,
    #[case] text: &str,
    #[case] tokens: Vec<(Boundary, Range<usize>)>,
    #[values(Decoding::Simple, Decoding::Viterbi)] decoding: Decoding,
) {
    let tokens: Vec<_> = tokens
        .into_iter()
        .map(|(boundary, range)| person_token(boundary, range))
        .collect();
    let entities = decoding.decode(text, &tokens).unwrap();

    assert!(!entities.is_empty());
    // 上层只依赖该片段的不变式来替换原文，不再搜索 word 子串。
    for entity in &entities {
        assert_eq!(&text[entity.start..entity.end], entity.word);
    }
}

#[rstest]
fn identical_values_at_different_positions_keep_distinct_offsets(
    person_token: impl Fn(Boundary, Range<usize>) -> TokenPrediction,
    outside_token: impl Fn(Range<usize>) -> TokenPrediction,
) {
    let text = "Alice and Alice";
    let entities = Decoding::Simple
        .decode(
            text,
            &[
                person_token(Boundary::Single, 0..5),
                outside_token(5..10),
                person_token(Boundary::Single, 10..15),
            ],
        )
        .unwrap();

    assert_eq!(entities.len(), 2);
    assert_eq!(entities[0].word, entities[1].word);
    // 两次出现必须落在各自的位置上，否则替换会改错地方。
    assert_eq!((entities[0].start, entities[0].end), (0, 5));
    assert_eq!((entities[1].start, entities[1].end), (10, 15));
}

#[rstest]
fn token_offsets_inside_a_character_widen_to_character_boundaries(
    person_token: impl Fn(Boundary, Range<usize>) -> TokenPrediction,
) {
    // " 李雷" 的字节边界是 0、1、4、7；token 边界 2 落在“李”内部。
    let text = " 李雷";
    let entities = Decoding::Simple
        .decode(
            text,
            &[
                person_token(Boundary::Begin, 2..4),
                person_token(Boundary::End, 4..7),
            ],
        )
        .unwrap();

    assert_eq!(entities.len(), 1);
    assert_eq!((entities[0].start, entities[0].end), (1, 7));
    assert_eq!(entities[0].word, "李雷");
    assert_eq!(&text[entities[0].start..entities[0].end], entities[0].word);
}

#[rstest]
fn empty_predictions_produce_no_entities(
    #[values(Decoding::Simple, Decoding::Viterbi)] decoding: Decoding,
) {
    assert!(decoding.decode("", &[]).unwrap().is_empty());
}
