// 架构参考 Transformers 的 OpenAIPrivacyFilter 实现。
use crate::{Error, Result, TokenLabel, config::Config, weights::Weights};
use burn::{
    nn::{Linear, RmsNorm, RotaryEncoding, RotaryEncodingConfig},
    tensor::{
        Int, Tensor, TensorData,
        activation::{sigmoid, softmax},
        backend::Backend,
    },
};
use std::path::Path;

type T<B, const D: usize> = Tensor<B, D>;

pub(crate) struct Model<B: Backend> {
    device: B::Device,
    pub config: Config,
    embedding: T<B, 2>,
    layers: Vec<Layer<B>>,
    norm: RmsNorm<B>,
    score: Linear<B>,
}

struct Layer<B: Backend> {
    input_norm: RmsNorm<B>,
    post_norm: RmsNorm<B>,
    q: Linear<B>,
    k: Linear<B>,
    v: Linear<B>,
    o: Linear<B>,
    experts: Experts<B>,
    sinks: T<B, 1>,
}

impl<B: Backend> Model<B> {
    pub fn load(path: impl AsRef<Path>, config: Config, device: &B::Device) -> Result<Self> {
        let weights = Weights::open(path, device)?;
        let c = &config;
        let h = c.hidden_size;
        let embedding = weights.tensor("model.embed_tokens.weight", [c.vocab_size, h])?;
        let layers = (0..c.num_hidden_layers)
            .map(|i| Layer::load(&weights, c, i))
            .collect::<Result<Vec<_>>>()?;
        let norm = weights.norm("model.norm", h, c.rms_norm_eps)?;
        let score = weights.linear("score", h, TokenLabel::COUNT)?;
        Ok(Self {
            device: device.clone(),
            config,
            embedding,
            layers,
            norm,
            score,
        })
    }

    pub fn forward(
        &self,
        sequences: &[&[u32]],
        mut submitted: impl FnMut(usize, usize),
    ) -> Result<Vec<[f32; TokenLabel::COUNT]>> {
        let c = &self.config;
        let ids: Vec<u32> = sequences
            .iter()
            .flat_map(|ids| ids.iter().copied())
            .collect();
        if ids.iter().any(|&id| id as usize >= c.vocab_size) {
            return Err(Error::Inference(
                "token ID outside embedding vocabulary".into(),
            ));
        }
        let indices = Tensor::<B, 1, Int>::from_data(
            TensorData::new(
                ids.iter().map(|&id| id as i64).collect::<Vec<_>>(),
                [ids.len()],
            ),
            &self.device,
        );
        let mut x = self.embedding.clone().select(0, indices);
        let plan = AttentionPlan::new(sequences, c, &self.device);
        for (index, layer) in self.layers.iter().enumerate() {
            let attn = layer.attention(layer.input_norm.forward(x.clone()), &plan, c);
            x = x + attn;
            let normalized = layer.post_norm.forward(x.clone());
            let mlp = layer.experts.forward(normalized, c)?;
            x = x + mlp;
            submitted(index + 1, self.layers.len());
        }
        let data = self
            .score
            .forward(self.norm.forward(x))
            .into_data()
            .convert::<f32>()
            .to_vec::<f32>()
            .map_err(|e| Error::Inference(format!("{e:?}")))?;
        if data.iter().any(|v| !v.is_finite()) {
            return Err(Error::Inference("non-finite output logits".into()));
        }
        Ok(data
            .chunks_exact(TokenLabel::COUNT)
            .map(|r| r.try_into().expect("分类头形状已验证"))
            .collect())
    }
}

struct Experts<B: Backend> {
    router: Linear<B>,
    gate_up: Vec<T<B, 2>>,
    gate_bias: Vec<T<B, 1>>,
    down: Vec<T<B, 2>>,
    down_bias: Vec<T<B, 1>>,
}

impl<B: Backend> Experts<B> {
    fn load(weights: &Weights<B>, c: &Config, prefix: &str) -> Result<Self> {
        let h = c.hidden_size;
        let m = c.intermediate_size;
        let e = c.num_local_experts;
        Ok(Self {
            router: weights.linear(&format!("{prefix}.router"), h, e)?,
            gate_up: weights.experts(&format!("{prefix}.experts.gate_up_proj"), [e, h, 2 * m])?,
            gate_bias: weights
                .experts(&format!("{prefix}.experts.gate_up_proj_bias"), [e, 2 * m])?,
            down: weights.experts(&format!("{prefix}.experts.down_proj"), [e, m, h])?,
            down_bias: weights.experts(&format!("{prefix}.experts.down_proj_bias"), [e, h])?,
        })
    }

    fn forward(&self, x: T<B, 2>, c: &Config) -> Result<T<B, 2>> {
        let device = x.device();
        let logits = self.router.forward(x.clone());
        let [n, h] = x.dims();
        let (values, indices) = logits.topk_with_indices(c.num_experts_per_tok, 1);
        // 参考实现先将路由权重除以 top_k，再将最终 MLP 输出乘以 top_k。
        // 保留此运算顺序，以对齐 f32 累加行为。
        let probabilities = (softmax(values, 1) / c.num_experts_per_tok as f32)
            .reshape([n * c.num_experts_per_tok]);
        let indices = indices
            .into_data()
            .convert::<i64>()
            .to_vec::<i64>()
            .map_err(|e| Error::Inference(format!("{e:?}")))?;

        let mut routed = vec![Vec::new(); c.num_local_experts];
        for (slot, &expert) in indices.iter().enumerate() {
            routed[expert as usize].push(slot as i64);
        }
        let mut out = T::<B, 2>::zeros([n, h], &device);
        for (expert, slots) in routed.into_iter().enumerate() {
            let tokens: Vec<i64> = slots
                .iter()
                .map(|&slot| slot / c.num_experts_per_tok as i64)
                .collect();
            if tokens.is_empty() {
                continue;
            }
            let count = tokens.len();
            let token_indices =
                Tensor::<B, 1, Int>::from_data(TensorData::new(tokens, [count]), &device);
            let projected = x
                .clone()
                .select(0, token_indices.clone())
                .matmul(self.gate_up[expert].clone())
                + self.gate_bias[expert].clone().unsqueeze();
            let gate = projected
                .clone()
                .slice([0..count, 0..c.intermediate_size])
                .clamp_max(7.0);
            let up = projected
                .slice([0..count, c.intermediate_size..2 * c.intermediate_size])
                .clamp(-7.0, 7.0);
            let activated = gate.clone() * sigmoid(gate * 1.702) * (up + 1.0);
            let result = activated.matmul(self.down[expert].clone())
                + self.down_bias[expert].clone().unsqueeze();
            let slot_indices =
                Tensor::<B, 1, Int>::from_data(TensorData::new(slots, [count]), &device);
            let scores = probabilities
                .clone()
                .select(0, slot_indices)
                .reshape([count, 1]);
            out = out.select_assign(
                0,
                token_indices,
                result * scores,
                burn::tensor::IndexingUpdateOp::Add,
            );
        }
        Ok(out * c.num_experts_per_tok as f32)
    }
}

impl<B: Backend> Layer<B> {
    fn load(weights: &Weights<B>, c: &Config, index: usize) -> Result<Self> {
        let p = format!("model.layers.{index}");
        let h = c.hidden_size;
        let q = c.num_attention_heads * c.head_dim;
        let kv = c.num_key_value_heads * c.head_dim;
        Ok(Self {
            input_norm: weights.norm(&format!("{p}.input_layernorm"), h, c.rms_norm_eps)?,
            post_norm: weights.norm(&format!("{p}.post_attention_layernorm"), h, c.rms_norm_eps)?,
            q: weights.linear(&format!("{p}.self_attn.q_proj"), h, q)?,
            k: weights.linear(&format!("{p}.self_attn.k_proj"), h, kv)?,
            v: weights.linear(&format!("{p}.self_attn.v_proj"), h, kv)?,
            o: weights.linear(&format!("{p}.self_attn.o_proj"), q, h)?,
            experts: Experts::load(weights, c, &format!("{p}.mlp"))?,
            sinks: weights.tensor(&format!("{p}.self_attn.sinks"), [c.num_attention_heads])?,
        })
    }

    fn attention(&self, x: T<B, 2>, plan: &AttentionPlan<B>, c: &Config) -> T<B, 2> {
        let n = x.dims()[0];
        let heads = c.num_attention_heads;
        let kv = c.num_key_value_heads;
        let d = c.head_dim;
        let scale = (d as f32).powf(-0.25);
        let q = plan.rope.forward(
            self.q
                .forward(x.clone())
                .reshape([n, heads, d])
                .swap_dims(0, 1),
        ) * scale;
        let k = plan.rope.forward(
            self.k
                .forward(x.clone())
                .reshape([n, kv, d])
                .swap_dims(0, 1),
        ) * scale;
        let v = self.v.forward(x).reshape([n, kv, d]).swap_dims(0, 1);
        let k = k.select(0, plan.kv_mapping.clone());
        let v = v.select(0, plan.kv_mapping.clone());
        let mut blocks = Vec::new();
        for block in &plan.blocks {
            let AttentionBlock {
                start, end, lo, hi, ..
            } = *block;
            let queries = end - start;
            let keys = hi - lo;
            let scores = q
                .clone()
                .slice([0..heads, start..end, 0..d])
                .matmul(k.clone().slice([0..heads, lo..hi, 0..d]).swap_dims(1, 2))
                + block.mask.clone();
            let sinks = self
                .sinks
                .clone()
                .reshape([heads, 1, 1])
                .expand([heads, queries, 1]);
            let probs =
                softmax(T::cat(vec![scores, sinks], 2), 2).slice([0..heads, 0..queries, 0..keys]);
            blocks.push(probs.matmul(v.clone().slice([0..heads, lo..hi, 0..d])));
        }
        self.o
            .forward(T::cat(blocks, 1).swap_dims(0, 1).reshape([n, heads * d]))
    }
}

impl crate::config::Rope {
    pub(crate) fn init<B: Backend>(
        &self,
        head_dim: usize,
        length: usize,
        device: &B::Device,
    ) -> RotaryEncoding<B> {
        let r = self;
        let correction = |rotations: f64| {
            head_dim as f64
                * (r.original_max_position_embeddings as f64
                    / (rotations * 2.0 * std::f64::consts::PI))
                    .ln()
                / (2.0 * r.rope_theta.ln())
        };
        let mut low = correction(r.beta_fast);
        let mut high = correction(r.beta_slow);
        if r.truncate {
            low = low.floor();
            high = high.ceil();
        }
        low = low.max(0.0);
        high = high.min(head_dim as f64 - 1.0);
        if high == low {
            high += 0.001;
        }
        // 按参考实现的 f32 运算顺序混合插值/外推频率。
        // 长文本会放大预先合并系数与 exp(log(theta)) 带来的舍入差异。
        let frequencies: Vec<f32> = (0..head_dim / 2)
            .map(|i| {
                let position_frequency =
                    (r.rope_theta as f32).powf((2 * i) as f32 / head_dim as f32);
                let extrapolation = 1.0_f32 / position_frequency;
                let interpolation = 1.0_f32 / (r.factor as f32 * position_frequency);
                let ramp = ((i as f32 - low as f32) / (high - low) as f32).clamp(0.0, 1.0);
                let extrapolation_factor = 1.0 - ramp;
                interpolation * (1.0 - extrapolation_factor) + extrapolation * extrapolation_factor
            })
            .collect();
        let mut rope = RotaryEncodingConfig::new(length, head_dim)
            .with_theta(r.rope_theta as f32)
            .init_with_frequency_scaling(
                |_| {
                    T::<B, 1>::from_data(
                        TensorData::new(frequencies.clone(), [head_dim / 2]),
                        device,
                    )
                },
                device,
            );
        rope.freq_complex =
            rope.freq_complex * r.attention_factor.unwrap_or(1.0 + 0.1 * r.factor.ln()) as f32;
        rope
    }
}

/// 同一批次的 RoPE、GQA 索引和窗口掩码在所有层之间复用。
struct AttentionPlan<B: Backend> {
    rope: RotaryEncoding<B>,
    kv_mapping: Tensor<B, 1, Int>,
    blocks: Vec<AttentionBlock<B>>,
}

struct AttentionBlock<B: Backend> {
    start: usize,
    end: usize,
    lo: usize,
    hi: usize,
    mask: T<B, 3>,
}

impl<B: Backend> AttentionPlan<B> {
    fn new(sequences: &[&[u32]], c: &Config, device: &B::Device) -> Self {
        let maximum = sequences.iter().map(|s| s.len()).max().expect("批次非空");
        let positions: Vec<i64> = sequences
            .iter()
            .flat_map(|s| (0..s.len()).map(|p| p as i64))
            .collect();
        let count = positions.len();
        let positions = Tensor::<B, 1, Int>::from_data(TensorData::new(positions, [count]), device);
        let mut rope = c.rope_parameters.init(c.head_dim, maximum, device);
        // 每条消息的位置从零开始，打包不会让另一条消息改变其位置编码。
        rope.freq_complex = rope.freq_complex.select(0, positions);
        let heads = c.num_attention_heads;
        let kv_mapping = Tensor::<B, 1, Int>::from_data(
            TensorData::new(
                (0..heads)
                    .map(|h| (h / (heads / c.num_key_value_heads)) as i64)
                    .collect::<Vec<_>>(),
                [heads],
            ),
            device,
        );
        let mut blocks = Vec::new();
        let mut offset = 0;
        for sequence in sequences {
            let length = sequence.len();
            for relative in (0..length).step_by(128) {
                let start = offset + relative;
                let end = offset + (relative + 128).min(length);
                let lo = offset + relative.saturating_sub(c.sliding_window);
                let hi = (end + c.sliding_window).min(offset + length);
                let mask: Vec<f32> = (start..end)
                    .flat_map(|q| {
                        (lo..hi).map(move |k| {
                            if q.abs_diff(k) <= c.sliding_window {
                                0.0
                            } else {
                                f32::NEG_INFINITY
                            }
                        })
                    })
                    .collect();
                let mask =
                    T::<B, 3>::from_data(TensorData::new(mask, [1, end - start, hi - lo]), device);
                blocks.push(AttentionBlock {
                    start,
                    end,
                    lo,
                    hi,
                    mask,
                });
            }
            offset += length;
        }
        Self {
            rope,
            kv_mapping,
            blocks,
        }
    }
}
