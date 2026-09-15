use crate::classifier::{Classification, ClassifiedField, Classifier};
use crate::protocol::adapter;
use privacy_filter::{Entity, Error, Result};
use privacy_store::{ApiFormat, InferenceNode, Store};
use serde_json::Value;
use std::sync::Arc;

struct Turn {
    hash: [u8; 32],
    parent: [u8; 32],
    fields: Vec<usize>,
}

/// Request-local view of a persistent chain. Used only on a blocking worker.
pub struct PrefixClassifier {
    classifier: Arc<dyn Classifier>,
    store: Arc<Store>,
    runtime: tokio::runtime::Handle,
    turns: Vec<Turn>,
    texts: Vec<String>,
}

impl PrefixClassifier {
    pub fn new(
        body: &Value,
        format: ApiFormat,
        classifier: Arc<dyn Classifier>,
        store: Arc<Store>,
        runtime: tokio::runtime::Handle,
    ) -> crate::Result<Self> {
        let fields = adapter(format).collect(body)?;
        let mut turns = Vec::new();
        if let Some(identity) = classifier.cache_identity() {
            let mut root = blake3::Hasher::new();
            root.update(b"privacy-turn-chain-v1");
            root.update(identity.as_bytes());
            root.update(format!("{format:?}").as_bytes());
            let mut parent = *root.finalize().as_bytes();
            let paths: Vec<String> = match format {
                ApiFormat::OpenAiResponses => {
                    let mut paths = vec!["/instructions".to_owned()];
                    match body.get("input").and_then(Value::as_array) {
                        Some(items) => {
                            paths.extend((0..items.len()).map(|i| format!("/input/{i}")))
                        }
                        None => paths.push("/input".to_owned()),
                    }
                    paths
                }
                ApiFormat::OpenAiChat | ApiFormat::AnthropicMessages => {
                    let mut paths = vec!["/system".to_owned()];
                    if let Some(items) = body.get("messages").and_then(Value::as_array) {
                        paths.extend((0..items.len()).map(|i| format!("/messages/{i}")));
                    }
                    paths
                }
            };
            for path in paths {
                let Some(value) = body.pointer(&path) else {
                    continue;
                };
                let mut hash = blake3::Hasher::new();
                hash.update(&parent);
                serde_json::to_writer(&mut hash, value)
                    .map_err(|e| Error::Inference(e.to_string()))?;
                let prefix = format!("{path}/");
                let indices: Vec<usize> = fields
                    .iter()
                    .enumerate()
                    .filter(|(_, field)| {
                        field.pointer == path || field.pointer.starts_with(&prefix)
                    })
                    .map(|(i, _)| i)
                    .collect();
                // Bind the result layout as well as the turn content to the node identity.
                for &i in &indices {
                    let relative = &fields[i].pointer[path.len()..];
                    hash.update(&(relative.len() as u64).to_le_bytes());
                    hash.update(relative.as_bytes());
                    hash.update(&(fields[i].text.len() as u64).to_le_bytes());
                    hash.update(fields[i].text.as_bytes());
                }
                let next = *hash.finalize().as_bytes();
                turns.push(Turn {
                    hash: next,
                    parent,
                    fields: indices,
                });
                parent = next;
            }
            if turns.iter().map(|turn| turn.fields.len()).sum::<usize>() != fields.len() {
                return Err(Error::Inference("unassigned prefix content fields".into()).into());
            }
        }
        Ok(Self {
            classifier,
            store,
            runtime,
            turns,
            texts: fields.into_iter().map(|field| field.text).collect(),
        })
    }

    fn run(&self, texts: &[&str]) -> Result<Classification> {
        if self.turns.is_empty() {
            return self.classifier.classify_recorded(texts);
        }
        if !self
            .texts
            .iter()
            .map(String::as_str)
            .eq(texts.iter().copied())
        {
            return Err(Error::Inference("prefix request content mismatch".into()));
        }
        let hashes: Vec<_> = self.turns.iter().map(|turn| turn.hash).collect();
        let existing = self
            .runtime
            .block_on(self.store.inference().find_many(&hashes))
            .map_err(|_| Error::Inference("persistent prefix lookup failed".into()))?;
        let mut output = Classification {
            fields: vec![ClassifiedField::inferred(Vec::new()); texts.len()],
            ..Default::default()
        };
        let mut missing = Vec::new();
        let mut pending = Vec::new();
        for turn in &self.turns {
            if let Some(node) = existing.get(&turn.hash) {
                if node.parent_hash != turn.parent || node.entities.len() != turn.fields.len() {
                    return Err(Error::Inference("invalid persisted turn result".into()));
                }
                for (&index, entities) in turn.fields.iter().zip(&node.entities) {
                    if entities.iter().any(|entity| {
                        texts[index].get(entity.start..entity.end) != Some(entity.word.as_str())
                    }) {
                        return Err(Error::Inference("invalid persisted entity offsets".into()));
                    }
                    output.fields[index] = ClassifiedField::replayed(entities.clone());
                }
                output.usage.cached_fragments += turn.fields.len();
            } else {
                pending.push(turn);
                missing.extend(turn.fields.iter().copied());
            }
        }
        if !missing.is_empty() {
            let inputs: Vec<_> = missing.iter().map(|&i| texts[i]).collect();
            let result = self.classifier.classify_recorded(&inputs)?;
            if result.fields.len() != missing.len() {
                return Err(Error::Inference(
                    "prefix classification result count mismatch".into(),
                ));
            }
            for (index, field) in missing.into_iter().zip(result.fields) {
                if field.entities.iter().any(|entity| {
                    texts[index].get(entity.start..entity.end) != Some(entity.word.as_str())
                        || !entity.score.is_finite()
                }) {
                    return Err(Error::Inference(
                        "invalid inferred entity offsets or score".into(),
                    ));
                }
                output.fields[index] = field;
            }
            output.usage.cached_fragments += result.usage.cached_fragments;
            output.usage.deduplicated_fragments = result.usage.deduplicated_fragments;
            output.usage.inferred_fragments = result.usage.inferred_fragments;
            output.elapsed_ms = result.elapsed_ms;
            output.performance = result.performance;
        }
        if !pending.is_empty() {
            let nodes: Vec<_> = pending
                .into_iter()
                .map(|turn| InferenceNode {
                    hash: turn.hash,
                    parent_hash: turn.parent,
                    entities: turn
                        .fields
                        .iter()
                        .map(|&i| output.fields[i].entities.clone())
                        .collect(),
                })
                .collect();
            self.runtime
                .block_on(self.store.inference().insert(&nodes))
                .map_err(|_| Error::Inference("persistent prefix write failed".into()))?;
        }
        Ok(output)
    }
}

impl Classifier for PrefixClassifier {
    fn classify(&self, texts: &[&str]) -> Result<Vec<Vec<Entity>>> {
        Ok(self
            .run(texts)?
            .fields
            .into_iter()
            .map(|field| field.entities)
            .collect())
    }

    fn classify_recorded(&self, texts: &[&str]) -> Result<Classification> {
        self.run(texts)
    }
}
