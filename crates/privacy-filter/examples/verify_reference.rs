//! 手动执行真实权重的端到端对照，不进入常规测试套件。
use burn::tensor::backend::Backend;
use privacy_filter::{Decoding, EntityGroup, PrivacyFilter};
use serde::Deserialize;

#[derive(Deserialize)]
struct Reference {
    text: String,
    logits: Vec<Vec<f32>>,
}

impl Reference {
    fn verify<B: Backend>(
        &self,
        classifier: &PrivacyFilter<B>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tokens = classifier.predict_tokens(&self.text)?;
        if tokens.len() != self.logits.len() {
            return Err("token 数量与参考数据不一致".into());
        }
        let error = tokens
            .iter()
            .flat_map(|token| &token.logits)
            .zip(self.logits.iter().flatten())
            .map(|(actual, expected)| (actual - expected).abs())
            .fold(0.0_f32, f32::max);
        if error >= 2e-3 {
            return Err(format!("最大 logit 误差超出容差：{error}").into());
        }
        for decoding in [Decoding::Simple, Decoding::Viterbi] {
            let entities = decoding.decode(&self.text, &tokens)?;
            let actual: Vec<_> = entities
                .iter()
                .map(|e| (e.entity_group, e.word.as_str()))
                .collect();
            if actual
                != [
                    (EntityGroup::PrivatePerson, " Harry Potter"),
                    (EntityGroup::PrivateEmail, " harry.potter@hogwarts.edu"),
                ]
            {
                return Err(format!("实体结果不一致：{entities:?}").into());
            }
            println!("{decoding:?}: {}", serde_json::to_string_pretty(&entities)?);
        }
        println!("{} 个 token 的最大 logit 绝对误差：{error}", tokens.len());
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "models/privacy-filter".into());
    let reference: Reference =
        serde_json::from_str(include_str!("../tests/fixtures/privacy-reference.json"))?;
    #[cfg(feature = "gpu")]
    if std::env::args().any(|a| a == "--gpu") {
        let classifier = PrivacyFilter::<privacy_filter::Gpu>::from_dir_on_device(
            path,
            Default::default(),
            &Default::default(),
        )?;
        return reference.verify(&classifier);
    }
    reference.verify(&PrivacyFilter::from_dir(path)?)
}
