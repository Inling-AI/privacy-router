use privacy_filter::observation::{Clock, EventKind, InferenceEvent, Observation, Observer};
use privacy_filter::{BatchLimits, Error, Options, PrivacyFilter};
use rstest::{fixture, rstest};
use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

/// 时钟仅在调用时推进，不读取真实时间，也不假设内部读取次数。
#[derive(Default)]
struct TestClock(Cell<u64>);

impl Clock for TestClock {
    fn now(&self) -> Duration {
        let tick = self.0.get();
        self.0.set(tick + 1);
        Duration::from_micros(tick)
    }
}

#[derive(Default)]
struct Recorder(RefCell<Vec<InferenceEvent>>);

impl Observer for Recorder {
    fn on_event(&self, event: &InferenceEvent) {
        self.0.borrow_mut().push(event.clone());
    }
}

impl Recorder {
    fn terminal(&self) -> InferenceEvent {
        let events = self.0.borrow();
        let terminals: Vec<_> = events
            .iter()
            .filter(|event| matches!(event.kind, EventKind::Completed | EventKind::Failed { .. }))
            .collect();
        assert_eq!(terminals.len(), 1, "每个调用只能有一个终止事件");
        terminals[0].clone()
    }
}

#[fixture]
fn classifier() -> PrivacyFilter {
    PrivacyFilter::from_dir_with_options(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny"),
        Options {
            max_tokens: 256,
            ..Default::default()
        },
    )
    .unwrap()
}

#[rstest]
#[case(1, 3)]
#[case(16, 6)]
fn observed_batch_preserves_results_and_reports_original_input_progress(
    classifier: PrivacyFilter,
    #[case] max_sequences: usize,
    #[case] max_tokens: usize,
) {
    let texts = ["t1 t2", "", "t3", "t4 t5 t6"];
    let limits = BatchLimits {
        max_sequences,
        max_tokens,
    };
    let recorder = Recorder::default();
    let clock = TestClock::default();
    let observed = classifier
        .classify_batch_observed(&texts, limits, &Observation::with_clock(&recorder, &clock))
        .unwrap();
    assert_eq!(observed, classifier.classify_batch(&texts, limits).unwrap());
    let terminal = recorder.terminal();
    assert!(matches!(terminal.kind, EventKind::Completed));
    let progress = &terminal.metrics.progress;
    assert_eq!(progress.total_inputs, texts.len());
    assert_eq!(progress.tokenized_inputs, texts.len());
    assert_eq!(progress.completed_inputs, texts.len());
    assert_eq!(progress.total_tokens, Some(6));
    assert_eq!(progress.completed_tokens, 6);
    assert_eq!(progress.total_batches, Some(progress.completed_batches));
    assert!(progress.completed_batches > 0);
    let first_result = terminal.metrics.time_to_first_result.unwrap();
    assert!(first_result < terminal.metrics.elapsed);
    assert!(
        terminal
            .metrics
            .input_tokens_per_second()
            .unwrap()
            .is_finite()
    );
    let events = recorder.0.borrow();
    let mut completed: Vec<_> = events
        .iter()
        .filter_map(|event| match event.kind {
            EventKind::InputCompleted(work) => Some((work.index, work.tokens)),
            _ => None,
        })
        .collect();
    completed.sort_unstable();
    assert_eq!(completed, [(0, 2), (1, 0), (2, 1), (3, 3)]);
    for pair in events.windows(2) {
        assert!(pair[0].metrics.elapsed <= pair[1].metrics.elapsed);
        assert!(
            pair[0].metrics.progress.completed_tokens <= pair[1].metrics.progress.completed_tokens
        );
        assert!(
            pair[0].metrics.progress.completed_inputs <= pair[1].metrics.progress.completed_inputs
        );
    }
}

#[rstest]
#[case(vec![])]
#[case(vec!["", ""])]
fn empty_work_finishes_without_fabricating_throughput(
    classifier: PrivacyFilter,
    #[case] texts: Vec<&str>,
) {
    let recorder = Recorder::default();
    let clock = TestClock::default();
    let output = classifier
        .classify_batch_observed(
            &texts,
            BatchLimits::default(),
            &Observation::with_clock(&recorder, &clock),
        )
        .unwrap();
    assert_eq!(output.len(), texts.len());
    assert!(output.iter().all(Vec::is_empty));
    let terminal = recorder.terminal();
    assert!(matches!(terminal.kind, EventKind::Completed));
    assert_eq!(terminal.metrics.progress.completed_inputs, texts.len());
    assert_eq!(terminal.metrics.progress.completed_batches, 0);
    assert_eq!(terminal.metrics.progress.completed_tokens, 0);
    assert!(terminal.metrics.input_tokens_per_second().is_none());
    assert!(terminal.metrics.time_to_first_result.is_none());
}

#[rstest]
fn rejected_input_reports_failure_without_claiming_completed_work(classifier: PrivacyFilter) {
    let recorder = Recorder::default();
    let clock = TestClock::default();
    let error = classifier
        .classify_batch_observed(
            &["t1", "t2 t3"],
            BatchLimits {
                max_sequences: 2,
                max_tokens: 1,
            },
            &Observation::with_clock(&recorder, &clock),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        Error::InputTooLong {
            actual: 2,
            limit: 1
        }
    ));
    let terminal = recorder.terminal();
    assert!(matches!(terminal.kind, EventKind::Failed { .. }));
    assert_eq!(terminal.metrics.progress.completed_inputs, 0);
    assert_eq!(terminal.metrics.progress.completed_tokens, 0);
    assert!(terminal.metrics.time_to_first_result.is_none());
}

#[rstest]
fn separate_calls_do_not_share_progress(classifier: PrivacyFilter) {
    let clock = TestClock::default();
    let first = Recorder::default();
    let second = Recorder::default();
    classifier
        .predict_tokens_observed("t1 t2", &Observation::with_clock(&first, &clock))
        .unwrap();
    classifier
        .predict_tokens_observed("t3", &Observation::with_clock(&second, &clock))
        .unwrap();
    assert_eq!(first.terminal().metrics.progress.completed_tokens, 2);
    assert_eq!(second.terminal().metrics.progress.completed_tokens, 1);
    assert_eq!(second.terminal().metrics.progress.completed_inputs, 1);
}
