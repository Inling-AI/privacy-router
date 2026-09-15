use crate::{Error, Result, TokenLabel};
use std::ops::Range;

/// 原始 token 分类信息。logits 按 [`TokenLabel::id`] 排列，
/// offsets 是输入 UTF-8 文本中的字节范围，可用于自定义解码和诊断。
#[derive(Debug, Clone)]
pub struct TokenPrediction {
    pub offsets: Range<usize>,
    pub logits: [f32; TokenLabel::COUNT],
}

impl TokenPrediction {
    /// 独立预测的标签；相同分数时优先较小的标签编号。
    pub fn label(&self) -> Result<TokenLabel> {
        self.validate_logits()?;
        Ok(TokenLabel::all()
            .max_by(|a, b| {
                self.logits[a.id()]
                    .total_cmp(&self.logits[b.id()])
                    .then_with(|| b.id().cmp(&a.id()))
            })
            .expect("标签集合非空"))
    }

    /// 指定标签的稳定 softmax 概率。
    pub fn probability(&self, label: TokenLabel) -> Result<f64> {
        self.validate_logits()?;
        let max = self
            .logits
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max) as f64;
        let denominator: f64 = self.logits.iter().map(|&v| (v as f64 - max).exp()).sum();
        Ok((self.logits[label.id()] as f64 - max).exp() / denominator)
    }

    pub(crate) fn validate_logits(&self) -> Result<()> {
        if self.logits.iter().any(|v| !v.is_finite()) {
            return Err(Error::Inference("non-finite token logits".into()));
        }
        Ok(())
    }
}
