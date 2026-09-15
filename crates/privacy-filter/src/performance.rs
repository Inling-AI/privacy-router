use crate::observation::{InferenceMetrics, Stage};
use serde::{Deserialize, Serialize};

/// Accumulated actual model work; cached results contribute no tokens or timings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelPerformance {
    pub tokens: u64,
    pub first_result_ms: Option<f64>,
    pub tokenization_ms: f64,
    pub validation_ms: f64,
    pub forward_ms: f64,
    pub decoding_ms: f64,
}

impl ModelPerformance {
    pub fn record(&mut self, metrics: &InferenceMetrics, first_result_ms: Option<f64>) {
        self.tokens += metrics.progress.completed_tokens as u64;
        if self.first_result_ms.is_none() {
            self.first_result_ms = first_result_ms;
        }
        self.tokenization_ms += metrics.stages.duration(Stage::Tokenization).as_secs_f64() * 1000.0;
        self.validation_ms += metrics.stages.duration(Stage::Validation).as_secs_f64() * 1000.0;
        self.forward_ms += metrics.stages.duration(Stage::Prefill).as_secs_f64() * 1000.0;
        self.decoding_ms += metrics.stages.duration(Stage::Decoding).as_secs_f64() * 1000.0;
    }

    pub fn tokens_per_second(&self) -> Option<f64> {
        (self.tokens > 0 && self.forward_ms > 0.0)
            .then(|| self.tokens as f64 * 1000.0 / self.forward_ms)
    }
}
