//! 完整文本片段的有界结果缓存；依赖底层 privacy-filter，不包含协议解析。
#![doc = include_str!("../README.md")]

use burn::tensor::backend::Backend;
use lru::LruCache;
use privacy_filter::{BatchLimits, Entity, Error, PrivacyFilter, Result};
use std::{collections::HashMap, sync::Arc};

/// 缓存同时受条目数及保留数据字节预算限制。
/// 字节预算包含实体字符串和条目数据，不代表分配器的精确 RSS。
#[derive(Debug, Clone, Copy)]
pub struct CacheLimits {
    pub max_entries: usize,
    pub max_bytes: usize,
}

impl Default for CacheLimits {
    fn default() -> Self {
        Self {
            max_entries: 4096,
            max_bytes: 16 * 1024 * 1024,
        }
    }
}

/// 每次请求的缓存决策，便于确认增量推理的实际工作量。
#[derive(Debug, Default, Clone, Copy, serde::Serialize)]
pub struct CacheUsage {
    pub cached_fragments: usize,
    pub deduplicated_fragments: usize,
    pub inferred_fragments: usize,
}

/// 按输入顺序返回结果；重复片段共享同一实体数组。
#[derive(Debug)]
pub struct CachedBatch {
    pub entities: Vec<Arc<[Entity]>>,
    pub usage: CacheUsage,
}

/// 模型、分词器及解码选项由实例拥有，缓存不能跨配置复用。
/// 同一租户或会话范围内可复用此实例；缓存键是完整片段的 BLAKE3 摘要。
pub struct CachedPrivacyFilter<B: Backend = privacy_filter::Cpu<f32>> {
    classifier: PrivacyFilter<B>,
    cache: ResultCache,
}

impl<B: Backend> CachedPrivacyFilter<B> {
    /// 用独立的上层结果缓存包裹底层推理引擎。
    pub fn new(classifier: PrivacyFilter<B>, limits: CacheLimits) -> Result<Self> {
        if limits.max_entries == 0 || limits.max_bytes == 0 {
            return Err(Error::InvalidOptions(
                "cache limits must be positive".into(),
            ));
        }
        Ok(CachedPrivacyFilter {
            classifier,
            cache: ResultCache {
                entries: LruCache::unbounded(),
                bytes: 0,
                limits,
            },
        })
    }
}

impl<B: Backend> CachedPrivacyFilter<B> {
    /// 查缓存与批内去重发生在分词之前；只对新片段执行分词和模型。
    pub fn classify_batch(&mut self, texts: &[&str], limits: BatchLimits) -> Result<CachedBatch> {
        self.classify(texts, limits, None)
    }

    /// Observe only cache misses that actually enter the model.
    pub fn classify_batch_observed(
        &mut self,
        texts: &[&str],
        limits: BatchLimits,
        observation: &privacy_filter::observation::Observation<'_>,
    ) -> Result<CachedBatch> {
        self.classify(texts, limits, Some(observation))
    }

    fn classify(
        &mut self,
        texts: &[&str],
        limits: BatchLimits,
        observation: Option<&privacy_filter::observation::Observation<'_>>,
    ) -> Result<CachedBatch> {
        limits.validate()?;
        let mut usage = CacheUsage::default();
        let mut outputs = vec![None; texts.len()];
        let mut unique = Vec::new();
        let mut misses: HashMap<[u8; 32], usize> = HashMap::new();
        let mut destinations: Vec<Vec<usize>> = Vec::new();
        for (index, text) in texts.iter().enumerate() {
            if text.is_empty() {
                outputs[index] = Some(Arc::from([]));
                continue;
            }
            let key = *blake3::hash(text.as_bytes()).as_bytes();
            if let Some(entry) = self.cache.entries.get(&key) {
                outputs[index] = Some(entry.entities.clone());
                usage.cached_fragments += 1;
            } else if let Some(&slot) = misses.get(&key) {
                destinations[slot].push(index);
                usage.deduplicated_fragments += 1;
            } else {
                misses.insert(key, unique.len());
                unique.push(*text);
                destinations.push(vec![index]);
            }
        }
        usage.inferred_fragments = unique.len();
        if !unique.is_empty() {
            let inferred = match observation {
                Some(observation) => {
                    self.classifier
                        .classify_batch_observed(&unique, limits, observation)?
                }
                None => self.classifier.classify_batch(&unique, limits)?,
            };
            // 所有推理成功后才写入结果缓存，失败不留下部分新结果。
            let mut ordered: Vec<_> = misses.into_iter().collect();
            ordered.sort_unstable_by_key(|(_, slot)| *slot);
            for (key, slot) in ordered {
                let entities: Arc<[Entity]> = inferred[slot].clone().into();
                for &index in &destinations[slot] {
                    outputs[index] = Some(entities.clone());
                }
                self.cache.insert(key, entities);
            }
        }
        Ok(CachedBatch {
            entities: outputs
                .into_iter()
                .map(|output| output.expect("每条输入已分配缓存或推理结果"))
                .collect(),
            usage,
        })
    }

    /// 丢弃该实例持有的历史结果；调用方仍持有的 Arc 结果不受影响。
    pub fn clear_cache(&mut self) {
        self.cache.entries.clear();
        self.cache.bytes = 0;
    }
}

struct CacheEntry {
    entities: Arc<[Entity]>,
    bytes: usize,
}

struct ResultCache {
    entries: LruCache<[u8; 32], CacheEntry>,
    bytes: usize,
    limits: CacheLimits,
}

impl ResultCache {
    fn insert(&mut self, key: [u8; 32], entities: Arc<[Entity]>) {
        let bytes = std::mem::size_of::<CacheEntry>()
            + key.len()
            + entities.len() * std::mem::size_of::<Entity>()
            + entities.iter().map(|e| e.word.capacity()).sum::<usize>();
        if bytes > self.limits.max_bytes {
            return;
        }
        while self.entries.len() >= self.limits.max_entries
            || self.bytes + bytes > self.limits.max_bytes
        {
            if let Some((_, entry)) = self.entries.pop_lru() {
                self.bytes -= entry.bytes;
            }
        }
        self.entries.put(key, CacheEntry { entities, bytes });
        self.bytes += bytes;
    }
}
