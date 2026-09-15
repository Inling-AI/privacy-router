use privacy_filter::{Decoding, Error, Options, PrivacyFilter};
use rstest::{fixture, rstest};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Reference {
    text: String,
    logits: Vec<Vec<f32>>,
}

#[fixture]
fn model_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny")
}

#[fixture]
fn classifier(model_dir: PathBuf) -> PrivacyFilter {
    PrivacyFilter::from_dir_with_options(
        model_dir,
        Options {
            max_tokens: 256,
            decoding: Decoding::Simple,
        },
    )
    .unwrap()
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
fn public_predictions_match_transformers_reference(classifier: PrivacyFilter, #[case] case: usize) {
    let references: Vec<Reference> =
        serde_json::from_str(include_str!("fixtures/tiny/reference.json")).unwrap();
    let reference = &references[case];
    let actual = classifier.predict_tokens(&reference.text).unwrap();
    assert_eq!(actual.len(), reference.logits.len());
    let error = actual
        .iter()
        .flat_map(|token| &token.logits)
        .zip(reference.logits.iter().flatten())
        .map(|(actual, expected)| (actual - expected).abs())
        .fold(0.0_f32, f32::max);
    assert!(error < 1e-4, "最大 logit 误差为 {error}");
}

#[rstest]
fn rejects_input_beyond_limit(model_dir: PathBuf) {
    let classifier = PrivacyFilter::from_dir_with_options(
        model_dir,
        Options {
            max_tokens: 1,
            decoding: Decoding::Simple,
        },
    )
    .unwrap();
    let error = classifier.classify("t1 t2").unwrap_err();
    assert!(matches!(
        error,
        Error::InputTooLong {
            actual: 2,
            limit: 1
        }
    ));
}

#[rstest]
fn empty_text_produces_no_entities(classifier: PrivacyFilter) {
    assert!(classifier.classify("").unwrap().is_empty());
}

#[rstest]
#[case(1, 256)]
#[case(16, 256)]
#[case(16, 140)]
fn packed_batch_preserves_independent_predictions_and_order(
    classifier: PrivacyFilter,
    #[case] max_sequences: usize,
    #[case] max_tokens: usize,
) {
    let references: Vec<Reference> =
        serde_json::from_str(include_str!("fixtures/tiny/reference.json")).unwrap();
    let texts = [
        &references[2].text[..],
        "",
        &references[0].text,
        &references[1].text,
        &references[0].text,
    ];
    let actual = classifier
        .predict_tokens_batch(
            &texts,
            privacy_filter::BatchLimits {
                max_sequences,
                max_tokens,
            },
        )
        .unwrap();
    for (index, source) in [(0, 2), (2, 0), (3, 1), (4, 0)] {
        assert_eq!(actual[index].len(), references[source].logits.len());
        let error = actual[index]
            .iter()
            .flat_map(|t| t.logits)
            .zip(references[source].logits.iter().flatten())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f32, f32::max);
        assert!(error < 1e-4, "最大误差：{error}");
    }
    assert!(actual[1].is_empty());
}

#[rstest]
#[case(0, 256)]
#[case(16, 0)]
fn invalid_batch_budget_is_rejected(
    classifier: PrivacyFilter,
    #[case] max_sequences: usize,
    #[case] max_tokens: usize,
) {
    let error = classifier
        .classify_batch(
            &["t1"],
            privacy_filter::BatchLimits {
                max_sequences,
                max_tokens,
            },
        )
        .unwrap_err();
    assert!(matches!(error, Error::InvalidOptions(_)));
}
