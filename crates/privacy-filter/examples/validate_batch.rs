//! 设备数值验证单独手动运行，避免 GPU 初始化及编译耗时进入常规测试。
use burn::tensor::backend::Backend;
use privacy_filter::{BatchLimits, Options, PrivacyFilter};
use serde::Deserialize;

#[derive(Deserialize)]
struct Reference {
    text: String,
    logits: Vec<Vec<f32>>,
}

struct Validation {
    model: std::path::PathBuf,
    reference: Vec<Reference>,
    entities_only: bool,
}
impl Validation {
    fn run<B: Backend>(&self, device: &B::Device) -> Result<(), Box<dyn std::error::Error>> {
        let max_tokens = self
            .reference
            .iter()
            .map(|r| r.logits.len())
            .sum::<usize>()
            .max(256);
        let model = PrivacyFilter::<B>::from_dir_on_device(
            &self.model,
            Options {
                max_tokens,
                ..Default::default()
            },
            device,
        )?;
        let reference = &self.reference;
        if reference.is_empty()
            || reference.iter().any(|r| {
                r.logits.iter().any(|row| {
                    row.len() != privacy_filter::TokenLabel::COUNT
                        || row.iter().any(|value| !value.is_finite())
                })
            })
        {
            return Err("参考数据为空、标签维度错误或包含非有限值".into());
        }
        let texts: Vec<_> = reference.iter().map(|r| r.text.as_str()).collect();
        let actual = model.predict_tokens_batch(
            &texts,
            BatchLimits {
                max_sequences: 16,
                max_tokens,
            },
        )?;
        let mut valid = true;
        for (actual, expected) in actual.iter().zip(reference) {
            if actual.len() != expected.logits.len() {
                return Err("推理 token 数量与参考数据不一致".into());
            }
            let error = actual
                .iter()
                .flat_map(|t| t.logits)
                .zip(expected.logits.iter().flatten())
                .map(|(a, b)| (a - b).abs())
                .fold(0.0_f32, f32::max);
            let mean = actual
                .iter()
                .flat_map(|t| t.logits)
                .zip(expected.logits.iter().flatten())
                .map(|(a, b)| (a - b).abs() as f64)
                .sum::<f64>()
                / (actual.len() * privacy_filter::TokenLabel::COUNT) as f64;
            let mismatches = actual
                .iter()
                .zip(&expected.logits)
                .filter(|(actual, expected)| {
                    let expected = expected
                        .iter()
                        .enumerate()
                        .max_by(|(_, a), (_, b)| a.total_cmp(b))
                        .unwrap()
                        .0;
                    actual.label().unwrap().id() != expected
                })
                .count();
            println!(
                "tokens={} max_logit_error={error} mean_logit_error={mean} label_mismatches={mismatches}",
                actual.len()
            );
            let expected_tokens: Vec<_> = actual
                .iter()
                .zip(&expected.logits)
                .map(|(actual, logits)| privacy_filter::TokenPrediction {
                    offsets: actual.offsets.clone(),
                    logits: logits.as_slice().try_into().expect("参考标签维度一致"),
                })
                .collect();
            for decoding in [
                privacy_filter::Decoding::Simple,
                privacy_filter::Decoding::Viterbi,
            ] {
                let actual_entities = decoding.decode(&expected.text, actual)?;
                let expected_entities = decoding.decode(&expected.text, &expected_tokens)?;
                let spans_equal = actual_entities
                    .iter()
                    .map(|e| (e.entity_group, &e.word))
                    .eq(expected_entities.iter().map(|e| (e.entity_group, &e.word)));
                let score_error = actual_entities
                    .iter()
                    .zip(&expected_entities)
                    .map(|(a, b)| (a.score - b.score).abs())
                    .fold(0.0_f64, f64::max);
                println!(
                    "decoding={decoding:?} entities={} spans_equal={spans_equal} max_score_error={score_error}",
                    actual_entities.len()
                );
                valid &= spans_equal && score_error <= 1e-3;
            }
            valid &= actual.len() == expected.logits.len()
                && mismatches == 0
                && (self.entities_only || error <= 2e-3);
        }
        if !valid {
            return Err("设备数值对照超出容差".into());
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .collect();
    let tiny = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny");
    let model = args
        .first()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| tiny.clone());
    let reference = args
        .get(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| tiny.join("reference.json"));
    let validation = Validation {
        model,
        entities_only: std::env::args().any(|a| a == "--entities"),
        reference: serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(
            reference,
        )?))?,
    };
    #[cfg(feature = "gpu")]
    if std::env::args().any(|a| a == "--gpu") {
        return validation.run::<privacy_filter::Gpu>(&Default::default());
    }
    validation.run::<privacy_filter::Cpu<f32>>(&Default::default())
}
