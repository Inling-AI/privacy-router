use privacy_filter::{Decoding, Error, Options, PrivacyFilter};
use privacy_filter_cache::{CacheLimits, CachedPrivacyFilter};
use rstest::{fixture, rstest};

#[fixture]
fn classifier() -> PrivacyFilter {
    PrivacyFilter::from_dir_with_options(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../privacy-filter/tests/fixtures/tiny"),
        Options {
            max_tokens: 256,
            decoding: Decoding::Simple,
        },
    )
    .unwrap()
}

#[rstest]
fn prefix_cache_infers_only_new_and_changed_fragments(classifier: PrivacyFilter) {
    use privacy_filter::BatchLimits;
    let mut cached = CachedPrivacyFilter::new(classifier, CacheLimits::default()).unwrap();
    let first = cached
        .classify_batch(&["t1 t2", "t3", "t1 t2"], BatchLimits::default())
        .unwrap();
    assert_eq!(first.usage.inferred_fragments, 2);
    assert_eq!(first.usage.deduplicated_fragments, 1);
    assert_eq!(first.entities[0], first.entities[2]);
    let next = cached
        .classify_batch(&["t1 t2", "t3", "t4"], BatchLimits::default())
        .unwrap();
    assert_eq!(next.usage.cached_fragments, 2);
    assert_eq!(next.usage.inferred_fragments, 1);
    assert_eq!(first.entities[..2], next.entities[..2]);
    let edited = cached
        .classify_batch(&["t1 t5", "t3", "t4"], BatchLimits::default())
        .unwrap();
    assert_eq!(edited.usage.cached_fragments, 2);
    assert_eq!(edited.usage.inferred_fragments, 1);
    cached.clear_cache();
    let cleared = cached
        .classify_batch(&["t3"], BatchLimits::default())
        .unwrap();
    assert_eq!(cleared.usage.inferred_fragments, 1);
}

#[rstest]
fn cache_evicts_least_recently_used_fragments(classifier: PrivacyFilter) {
    use privacy_filter::BatchLimits;
    let mut cached = CachedPrivacyFilter::new(
        classifier,
        CacheLimits {
            max_entries: 2,
            max_bytes: 100_000,
        },
    )
    .unwrap();
    cached
        .classify_batch(&["t1", "t2"], BatchLimits::default())
        .unwrap();
    cached
        .classify_batch(&["t1"], BatchLimits::default())
        .unwrap();
    cached
        .classify_batch(&["t3"], BatchLimits::default())
        .unwrap();
    let retained = cached
        .classify_batch(&["t1", "t3"], BatchLimits::default())
        .unwrap();
    assert_eq!(retained.usage.cached_fragments, 2);
    let evicted = cached
        .classify_batch(&["t2"], BatchLimits::default())
        .unwrap();
    assert_eq!(evicted.usage.inferred_fragments, 1);
}

#[rstest]
fn cache_does_not_admit_entries_exceeding_byte_budget(classifier: PrivacyFilter) {
    use privacy_filter::BatchLimits;
    let mut cached = CachedPrivacyFilter::new(
        classifier,
        CacheLimits {
            max_entries: 100,
            max_bytes: 1,
        },
    )
    .unwrap();
    cached
        .classify_batch(&["t1"], BatchLimits::default())
        .unwrap();
    let repeated = cached
        .classify_batch(&["t1"], BatchLimits::default())
        .unwrap();
    assert_eq!(repeated.usage.inferred_fragments, 1);
}

#[rstest]
fn failed_batch_does_not_cache_partial_new_results(classifier: PrivacyFilter) {
    use privacy_filter::BatchLimits;
    let mut cached = CachedPrivacyFilter::new(classifier, CacheLimits::default()).unwrap();
    let error = cached
        .classify_batch(
            &["t1", "t2 t3"],
            BatchLimits {
                max_sequences: 16,
                max_tokens: 1,
            },
        )
        .unwrap_err();
    assert!(matches!(
        error,
        Error::InputTooLong {
            actual: 2,
            limit: 1
        }
    ));
    let retry = cached
        .classify_batch(&["t1"], BatchLimits::default())
        .unwrap();
    assert_eq!(retry.usage.inferred_fragments, 1);
}
