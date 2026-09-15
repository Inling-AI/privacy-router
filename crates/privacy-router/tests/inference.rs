use privacy_filter::{Entity, EntityGroup, Error};
use privacy_filter_cache::{CacheUsage, CachedBatch};
use privacy_router::{inference::ContextualText, redaction::redact};
use privacy_rules::RuleSet;
use rstest::{fixture, rstest};
use std::sync::Arc;

/// 有上下文预算的模型边界替身；仅在完整实体可见时识别，暴露切块丢上下文的问题。
struct BoundedModel {
    needle: &'static str,
    limit: usize,
}

impl BoundedModel {
    fn classify(&self, texts: &[&str]) -> privacy_filter::Result<CachedBatch> {
        for text in texts {
            // 模拟不同文字的 token 密度，不能用全文平均字符数代替实际预算。
            let actual: usize = text.chars().map(|c| if c.is_ascii() { 1 } else { 3 }).sum();
            if actual > self.limit {
                return Err(Error::InputTooLong {
                    actual,
                    limit: self.limit,
                });
            }
        }
        Ok(CachedBatch {
            entities: texts
                .iter()
                .map(|text| {
                    Arc::from(
                        text.match_indices(self.needle)
                            .map(|(start, word)| Entity {
                                entity_group: EntityGroup::Secret,
                                score: 0.99,
                                start,
                                end: start + word.len(),
                                word: word.to_owned(),
                            })
                            .collect::<Vec<_>>(),
                    )
                })
                .collect(),
            usage: CacheUsage {
                inferred_fragments: texts.len(),
                ..Default::default()
            },
        })
    }
}

#[fixture]
fn model() -> BoundedModel {
    BoundedModel {
        needle: "secret1234",
        limit: 48,
    }
}

#[rstest]
fn every_boundary_position_preserves_and_redacts_the_complete_entity(
    model: BoundedModel,
    #[values(0, 1, 12, 20, 24, 28, 35, 42, 47, 60, 80)] prefix: usize,
    #[values(" ", "界")] filler: &str,
) {
    let text = format!(
        "{}{}{}",
        filler.repeat(prefix),
        model.needle,
        " ".repeat(96)
    );
    let result = ContextualText::new(&text)
        .classify_with(|texts| model.classify(texts))
        .unwrap();
    let entities = &result.entities[0];
    assert_eq!(entities.len(), 1, "重复窗口只能产生一条实体");
    assert_eq!(entities[0].word, model.needle);
    assert_eq!(&text[entities[0].start..entities[0].end], model.needle);
    let redacted = redact(&text, entities, &RuleSet::builtin());
    assert_eq!(
        redacted.redacted,
        text.replace(model.needle, "<redacted_secret>")
    );
}

#[rstest]
fn a_multibyte_entity_at_the_end_retains_its_byte_offsets() {
    let model = BoundedModel {
        needle: "机密值12345",
        limit: 64,
    };
    let text = format!("{}{}", "界 ".repeat(60), model.needle);
    let result = ContextualText::new(&text)
        .classify_with(|texts| model.classify(texts))
        .unwrap();
    assert_eq!(result.entities[0].len(), 1);
    assert_eq!(result.entities[0][0].end, text.len());
    assert_eq!(result.entities[0][0].word, model.needle);
}

#[rstest]
fn overlapping_partial_observations_are_reassembled_before_rule_evaluation() {
    let text = format!("{}{}{}", " ".repeat(20), "1".repeat(80), " ".repeat(20));
    let result = ContextualText::new(&text)
        .classify_with(|texts| {
            if let Some(text) = texts.iter().find(|text| text.len() > 32) {
                return Err(Error::InputTooLong {
                    actual: text.len(),
                    limit: 32,
                });
            }
            Ok(CachedBatch {
                entities: texts
                    .iter()
                    .map(|text| {
                        let found = text.find('1').map(|start| Entity {
                            entity_group: EntityGroup::Secret,
                            score: 0.99,
                            start,
                            end: text.rfind('1').unwrap() + 1,
                            word: text.trim().to_owned(),
                        });
                        Arc::from(found.into_iter().collect::<Vec<_>>())
                    })
                    .collect(),
                usage: CacheUsage::default(),
            })
        })
        .unwrap();
    assert_eq!(result.entities[0].len(), 1);
    assert_eq!(result.entities[0][0].word, "1".repeat(80));
    assert_eq!(
        redact(&text, &result.entities[0], &RuleSet::builtin()).redacted,
        format!("{}<redacted_secret>{}", " ".repeat(20), " ".repeat(20))
    );
}

#[rstest]
fn errors_in_window_inference_do_not_return_partial_results() {
    let text = "x".repeat(100);
    let outcome = ContextualText::new(&text).classify_with(|texts| {
        if texts == [text.as_str()] {
            Err(Error::InputTooLong {
                actual: 100,
                limit: 40,
            })
        } else {
            Err(Error::Inference("unavailable".into()))
        }
    });
    assert!(matches!(outcome, Err(Error::Inference(_))));
}

#[rstest]
fn unrepresentable_input_fails_instead_of_retrying_forever() {
    let outcome = ContextualText::new("界").classify_with(|_| {
        Err(Error::InputTooLong {
            actual: 3,
            limit: 1,
        })
    });
    assert!(matches!(outcome, Err(Error::InputTooLong { .. })));
}
