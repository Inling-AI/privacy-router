use crate::{Error, Result, TokenLabel};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub model_type: String,
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub head_dim: usize,
    pub num_attention_heads: usize,
    pub num_key_value_heads: usize,
    pub num_hidden_layers: usize,
    pub num_local_experts: usize,
    pub num_experts_per_tok: usize,
    pub vocab_size: usize,
    pub sliding_window: usize,
    pub max_position_embeddings: usize,
    pub rms_norm_eps: f32,
    pub attention_bias: bool,
    pub rope_parameters: Rope,
    pub id2label: BTreeMap<usize, String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Rope {
    pub rope_type: String,
    pub rope_theta: f64,
    pub factor: f64,
    pub beta_fast: f64,
    pub beta_slow: f64,
    pub original_max_position_embeddings: usize,
    pub truncate: bool,
    pub attention_factor: Option<f64>,
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        let r = &self.rope_parameters;
        if self.model_type != "openai_privacy_filter"
            || r.rope_type != "yarn"
            || !self.attention_bias
            || self.hidden_size == 0
            || self.intermediate_size == 0
            || self.head_dim == 0
            || !self.head_dim.is_multiple_of(2)
            || self.num_key_value_heads == 0
            || self.num_attention_heads == 0
            || !self
                .num_attention_heads
                .is_multiple_of(self.num_key_value_heads)
            || self.num_hidden_layers == 0
            || self.vocab_size == 0
            || self.num_experts_per_tok == 0
            || self.num_experts_per_tok > self.num_local_experts
            || !self.rms_norm_eps.is_finite()
            || self.rms_norm_eps <= 0.0
            || !r.rope_theta.is_finite()
            || r.rope_theta <= 1.0
            || !r.factor.is_finite()
            || r.factor < 1.0
            || !r.beta_slow.is_finite()
            || !r.beta_fast.is_finite()
            || r.beta_slow <= 0.0
            || r.beta_fast < r.beta_slow
            || r.original_max_position_embeddings == 0
            || r.attention_factor
                .is_some_and(|v| !v.is_finite() || v <= 0.0)
        {
            return Err(Error::InvalidModel(
                "invalid dimensions or unsupported architecture/RoPE configuration".into(),
            ));
        }
        if self.id2label.len() != TokenLabel::COUNT
            || TokenLabel::all()
                .any(|label| self.id2label.get(&label.id()) != Some(&label.to_string()))
        {
            return Err(Error::InvalidModel(format!(
                "expected the original {} BIOES labels",
                TokenLabel::COUNT
            )));
        }
        Ok(())
    }
}
