//! 手动观察模型进度；不输出输入内容，只报告聚合指标。
use burn::tensor::backend::Backend;
use privacy_filter::observation::{EventKind, InferenceEvent, Observation, Observer, Stage};
use privacy_filter::{BatchLimits, Options, PrivacyFilter};

struct Console;

impl Observer for Console {
    fn on_event(&self, event: &InferenceEvent) {
        match &event.kind {
            EventKind::BatchStarted { index, inputs } => {
                println!("batch={index} inputs={inputs:?}");
            }
            EventKind::InputCompleted(input) => {
                println!(
                    "input={} completed_tokens={} completed_inputs={}/{}",
                    input.index,
                    event.metrics.progress.completed_tokens,
                    event.metrics.progress.completed_inputs,
                    event.metrics.progress.total_inputs
                );
            }
            EventKind::Completed | EventKind::Failed { .. } => {
                println!(
                    "terminal={:?} elapsed={:?} prefill={:?} input_tokens_per_second={:?} first_result={:?}",
                    event.kind,
                    event.metrics.elapsed,
                    event.metrics.stages.duration(Stage::Prefill),
                    event.metrics.input_tokens_per_second(),
                    event.metrics.time_to_first_result
                );
            }
            _ => {}
        }
    }
}

struct Example;

impl Example {
    fn run<B: Backend>(path: &str, device: &B::Device) -> Result<(), Box<dyn std::error::Error>> {
        let model = PrivacyFilter::<B>::from_dir_on_device(path, Options::default(), device)?;
        let result = model.classify_batch_observed(
            &[
                "My name is Harry Potter.",
                "",
                "Contact harry.potter@hogwarts.edu.",
            ],
            BatchLimits::default(),
            &Observation::new(&Console),
        )?;
        println!("entities={}", result.iter().map(Vec::len).sum::<usize>());
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let path = args
        .iter()
        .find(|arg| !arg.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("models/privacy-filter");
    #[cfg(feature = "gpu")]
    if args.iter().any(|arg| arg == "--gpu") {
        return Example::run::<privacy_filter::Gpu>(path, &Default::default());
    }
    Example::run::<privacy_filter::Cpu<f32>>(path, &Default::default())
}
