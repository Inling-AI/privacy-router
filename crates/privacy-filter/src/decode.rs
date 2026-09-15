use crate::{Boundary, Entity, Error, ModelLabel, Result, TokenLabel, TokenPrediction};

/// 将 token 标签解码为连续实体的策略。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Decoding {
    /// 对每个 token 取 argmax，再按 BIOES 边界聚合。
    #[default]
    Simple,
    /// 求全局最优的合法 BIOES 路径，使用原始模型默认工作点的零转移偏置。
    Viterbi,
}

impl Decoding {
    /// 将同一输入文本的 token 分数解码为实体。
    /// 偏移必须单调、不越界；允许字节级 BPE 在同一 Unicode 字符内重叠。
    pub fn decode(self, text: &str, predictions: &[TokenPrediction]) -> Result<Vec<Entity>> {
        let mut previous = 0..0;
        for token in predictions {
            token.validate_logits()?;
            let range = &token.offsets;
            if range.start > range.end
                || range.end > text.len()
                || range.start < previous.start
                || range.end < previous.end
            {
                return Err(Error::Inference(
                    "invalid or unordered token offsets".into(),
                ));
            }
            previous = range.clone();
        }
        let path = self.labels(predictions)?;
        let mut spans = SpanBuilder::new(text);
        for (label, token) in path.into_iter().zip(predictions) {
            spans.push(label, token)?;
        }
        Ok(spans.finish())
    }

    fn labels(self, predictions: &[TokenPrediction]) -> Result<Vec<TokenLabel>> {
        if predictions.is_empty() {
            return Ok(Vec::new());
        }
        if self == Self::Simple {
            return predictions.iter().map(TokenPrediction::label).collect();
        }
        let labels: Vec<_> = TokenLabel::all().collect();
        let mut previous = [f64::NEG_INFINITY; TokenLabel::COUNT];
        for &label in &labels {
            if label.can_start() {
                previous[label.id()] = predictions[0].logits[label.id()] as f64;
            }
        }
        let mut back = vec![[TokenLabel::Outside; TokenLabel::COUNT]; predictions.len()];
        for (t, token) in predictions.iter().enumerate().skip(1) {
            let mut next = [f64::NEG_INFINITY; TokenLabel::COUNT];
            for &target in &labels {
                for &source in &labels {
                    let value = previous[source.id()] + token.logits[target.id()] as f64;
                    if source.allows_next(target) && value > next[target.id()] {
                        next[target.id()] = value;
                        back[t][target.id()] = source;
                    }
                }
            }
            previous = next;
        }
        let mut path = vec![TokenLabel::Outside; predictions.len()];
        path[predictions.len() - 1] = labels
            .into_iter()
            .filter(|l| l.can_end())
            .max_by(|a, b| {
                previous[a.id()]
                    .total_cmp(&previous[b.id()])
                    .then_with(|| b.id().cmp(&a.id()))
            })
            .expect("背景标签允许结束路径");
        for t in (1..predictions.len()).rev() {
            path[t - 1] = back[t][path[t].id()];
        }
        Ok(path)
    }
}

struct ActiveSpan {
    group: ModelLabel,
    start: usize,
    end: usize,
    probability_sum: f64,
    tokens: usize,
}

/// 聚合状态归属于构建器，避免在解码过程中传递无名状态元组。
struct SpanBuilder<'a> {
    text: &'a str,
    active: Option<ActiveSpan>,
    entities: Vec<Entity>,
}

impl<'a> SpanBuilder<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            active: None,
            entities: Vec::new(),
        }
    }

    fn push(&mut self, label: TokenLabel, token: &TokenPrediction) -> Result<()> {
        if token.offsets.is_empty() {
            return Ok(());
        }
        let TokenLabel::Entity { group, boundary } = label else {
            self.flush();
            return Ok(());
        };
        if self.active.as_ref().is_some_and(|span| {
            span.group != group || matches!(boundary, Boundary::Begin | Boundary::Single)
        }) {
            self.flush();
        }
        let probability = token.probability(label)?;
        if let Some(span) = &mut self.active {
            span.end = span.end.max(token.offsets.end);
            span.probability_sum += probability;
            span.tokens += 1;
        } else {
            self.active = Some(ActiveSpan {
                group,
                start: token.offsets.start,
                end: token.offsets.end,
                probability_sum: probability,
                tokens: 1,
            });
        }
        if matches!(boundary, Boundary::End | Boundary::Single) {
            self.flush();
        }
        Ok(())
    }

    fn flush(&mut self) {
        if let Some(mut span) = self.active.take() {
            // 字节级 BPE 可能将同一个 Unicode 字符拆到多个 token。
            while span.start > 0 && !self.text.is_char_boundary(span.start) {
                span.start -= 1;
            }
            while span.end < self.text.len() && !self.text.is_char_boundary(span.end) {
                span.end += 1;
            }
            self.entities.push(Entity {
                entity_group: span.group.category(),
                score: span.probability_sum / span.tokens as f64,
                start: span.start,
                end: span.end,
                word: self.text[span.start..span.end].to_owned(),
            });
        }
    }

    fn finish(mut self) -> Vec<Entity> {
        self.flush();
        self.entities
    }
}
