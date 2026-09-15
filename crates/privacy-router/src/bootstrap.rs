//! Startup owns its terminal presentation; the model layer only emits progress events.

use clap::ValueEnum;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use privacy_model::{DownloadBackend, Downloader, Event, Hub, ModelFile, Repository};
use privacy_router::{
    classifier::Classifier,
    config::{InferenceBackend, LogStyle, ModelAction, ServerArgs},
};
use std::{sync::Arc, time::Duration};

pub struct Bootstrap {
    args: Arc<ServerArgs>,
    bar: ProgressBar,
}

impl Bootstrap {
    pub fn new(args: Arc<ServerArgs>) -> Self {
        let target = match args.log_format {
            LogStyle::Human => ProgressDrawTarget::stderr(),
            LogStyle::Json => ProgressDrawTarget::hidden(),
        };
        let bar = ProgressBar::with_draw_target(None, target);
        Self { args, bar }
    }

    pub fn stage(&self, message: impl Into<String>) {
        self.bar.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg} [{elapsed_precise}]")
                .expect("spinner template"),
        );
        self.bar.set_message(message.into());
        self.bar.enable_steady_tick(Duration::from_millis(100));
    }

    fn event(&self, event: Event<'_>) {
        match event {
            Event::Checking(file) => self.stage(format!("Verifying {}", file.as_ref())),
            Event::Probing => {
                self.stage("Testing download provider latency and speed");
                self.bar.suspend(|| tracing::info!(stage = "model_probe", "testing providers before download"));
            }
            Event::Measured(measurement) => self.bar.suspend(|| {
                tracing::info!(stage = "model_probe", provider = %measurement.provider,
                    first_byte_ms = measurement.first_byte.as_secs_f64() * 1000.0,
                    elapsed_ms = measurement.elapsed.as_secs_f64() * 1000.0,
                    bytes_per_second = measurement.bytes_per_second(), "provider benchmark completed");
            }),
            Event::Rejected { provider, reason } => self.bar.suspend(|| {
                tracing::warn!(stage = "model_download", provider, reason, "provider attempt failed");
            }),
            Event::Selected(provider) => {
                self.stage(format!("Using {provider}"));
                self.bar.suspend(|| tracing::info!(stage = "model_download", provider, "selected download provider"));
            }
            Event::Downloading { file, provider, bytes, total } => {
                if bytes == 0 {
                    self.bar.reset();
                    self.bar.set_style(ProgressStyle::with_template("{spinner:.cyan} {msg} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} {bytes_per_sec} ETA {eta}").expect("download template"));
                    self.bar.set_length(total);
                    self.bar.set_message(format!("{} via {provider}", file.as_ref()));
                    self.bar.enable_steady_tick(Duration::from_millis(100));
                    self.bar.suspend(|| tracing::info!(stage = "model_download", file = file.as_ref(), provider, total_bytes = total, "downloading model artifact"));
                }
                self.bar.set_position(bytes);
            }
            Event::Ready(file) => self.bar.suspend(|| tracing::info!(stage = "model_verify", file = file.as_ref(), "model artifact verified")),
        }
    }

    pub async fn prepare(&self) -> Result<(), String> {
        self.model(if self.args.offline {
            ModelAction::Verify
        } else {
            ModelAction::Download
        })
        .await
    }

    pub async fn model(&self, action: ModelAction) -> Result<(), String> {
        let artifacts: Vec<_> = ModelFile::all().map(ModelFile::artifact).collect();
        if matches!(action, ModelAction::Verify)
            || (self.args.offline && matches!(action, ModelAction::Download))
        {
            for artifact in &artifacts {
                self.event(Event::Checking(artifact.file));
                Downloader::verify(&self.args.model_dir, artifact)
                    .await
                    .map_err(|error| error.to_string())?;
                self.event(Event::Ready(artifact.file));
            }
            return Ok(());
        }
        if self.args.offline {
            return Err("provider benchmarking is unavailable in offline mode".into());
        }
        let hubs: Vec<_> = self
            .args
            .model_source
            .map_or_else(|| Hub::all().collect(), |hub| vec![hub]);
        let backends = hubs
            .into_iter()
            .map(|hub| Arc::new(Repository::new(hub)) as Arc<dyn DownloadBackend>)
            .collect();
        let downloader = Downloader::new(backends).map_err(|error| error.to_string())?;
        match action {
            ModelAction::Benchmark => {
                downloader
                    .benchmark(|event| self.event(event))
                    .await
                    .map_err(|error| error.to_string())?;
            }
            ModelAction::Download => downloader
                .ensure(&self.args.model_dir, &artifacts, |event| self.event(event))
                .await
                .map_err(|error| error.to_string())?,
            ModelAction::Verify => unreachable!("handled before constructing HTTP client"),
        }
        Ok(())
    }

    pub async fn load(&self) -> Result<Arc<dyn Classifier>, String> {
        self.stage("Loading tokenizer, configuration and model weights onto the device");
        let args = self.args.clone();
        tokio::task::spawn_blocking(move || Self::load_on_thread(&args))
            .await
            .map_err(|error| format!("model loading task failed: {error}"))?
    }

    fn load_on_thread(args: &ServerArgs) -> Result<Arc<dyn Classifier>, String> {
        tracing::info!(
            stage = "startup",
            backend = args
                .backend
                .to_possible_value()
                .expect("backend is a CLI value")
                .get_name(),
            "loading model"
        );
        match args.backend {
            InferenceBackend::Cpu => Self::load_backend::<privacy_filter::Cpu>(args),
            #[cfg(feature = "gpu")]
            InferenceBackend::Gpu => Self::load_backend::<privacy_filter::Gpu>(args),
        }
    }

    fn load_backend<B: burn::tensor::backend::Backend>(
        args: &ServerArgs,
    ) -> Result<Arc<dyn Classifier>, String> {
        let engine = privacy_router::inference::load::<B>(
            &args.model_dir,
            &Default::default(),
            privacy_filter::Options {
                max_tokens: args.max_tokens,
                ..Default::default()
            },
            args.cache_limits(),
            args.batch_limits(),
        )
        .map_err(|error| format!("unable to load model: {error}"))?;
        Ok(Arc::new(engine))
    }

    pub fn finish(&self) {
        self.bar.finish_and_clear();
    }
}

impl Drop for Bootstrap {
    fn drop(&mut self) {
        self.finish();
    }
}
