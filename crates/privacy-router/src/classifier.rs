use privacy_filter::{Entity, Filters, Result};
use privacy_filter_cache::CacheUsage;

/// 一个内容字段的识别结果。
///
/// 实体与「这段内容此前是否已经审过」是同一次识别的两个侧面，因此绑在同一个类型里，
/// 而不是两条必须靠约定对齐的平行数组——长度一旦写错就是错位替换。
#[derive(Debug, Clone, PartialEq)]
pub struct ClassifiedField {
    pub entities: Vec<Entity>,
    /// 命中了持久化的 turn 链：这段内容在更早的请求里已经判过，本次只是重放。
    pub already_audited: bool,
}

impl ClassifiedField {
    /// 本次新识别出来的内容。
    pub fn inferred(entities: Vec<Entity>) -> Self {
        Self {
            entities,
            already_audited: false,
        }
    }

    /// 已经审过的内容，实体来自持久化链。
    pub fn replayed(entities: Vec<Entity>) -> Self {
        Self {
            entities,
            already_audited: true,
        }
    }
}

/// 一次批量识别的结果与它的工作量。
#[derive(Debug, Clone, Default)]
pub struct Classification {
    /// 与输入等长、顺序一致。
    pub fields: Vec<ClassifiedField>,
    pub usage: CacheUsage,
    pub elapsed_ms: i64,
    pub performance: privacy_filter::performance::ModelPerformance,
}

/// 文本到隐私实体的识别能力。
///
/// 实现是**阻塞**的：本地模型推理占用 CPU 或 GPU，调用方负责把它放到阻塞线程池执行。
/// 保持同步让 trait 对象安全，运行时可选择 CPU 或 GPU 后端而不必把泛型扩散到调用方。
pub trait Classifier: Send + Sync {
    /// Stable fingerprint of model weights, tokenizer, and inference semantics.
    /// Classifiers without a fingerprint do not use persistent predictions.
    fn cache_identity(&self) -> Option<&str> {
        None
    }
    /// 批量识别。输出顺序与输入一致，长度也必须一致。
    fn classify(&self, texts: &[&str]) -> Result<Vec<Vec<Entity>>>;

    /// 与 [`Classifier::classify`] 相同，但附带缓存用量与耗时。
    ///
    /// 默认实现不报告工作量，只保证实体结果；真实引擎覆盖它以提供统计。
    fn classify_recorded(&self, texts: &[&str]) -> Result<Classification> {
        Ok(Classification {
            fields: self
                .classify(texts)?
                .into_iter()
                .map(ClassifiedField::inferred)
                .collect(),
            usage: CacheUsage::default(),
            elapsed_ms: 0,
            performance: Default::default(),
        })
    }
}

/// 在识别链末端接上确定性过滤器：形状固定的类别由过滤器给出命中，与模型命中一起进入判定。
///
/// 装饰器包住整条识别链（含持久化链的重放与真实推理），两类内容因此走同一条合并路径：
/// 只改识别结果，不动判定与替换。合并规则与过滤器名单由 `privacy_filter::Filters` 独家持有。
pub struct FilteredClassifier<C> {
    inner: C,
    filters: Filters,
}

impl<C: Classifier> FilteredClassifier<C> {
    /// 使用全部内置确定性过滤器。
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            filters: Filters::deterministic(),
        }
    }

    fn reconcile(&self, text: &str, entities: &mut Vec<Entity>) {
        *entities = self.filters.combine(text, std::mem::take(entities));
    }
}

impl<C: Classifier> Classifier for FilteredClassifier<C> {
    fn cache_identity(&self) -> Option<&str> {
        self.inner.cache_identity()
    }

    fn classify(&self, texts: &[&str]) -> Result<Vec<Vec<Entity>>> {
        let mut result = self.inner.classify(texts)?;
        for (text, entities) in texts.iter().zip(result.iter_mut()) {
            self.reconcile(text, entities);
        }
        Ok(result)
    }

    fn classify_recorded(&self, texts: &[&str]) -> Result<Classification> {
        let mut classification = self.inner.classify_recorded(texts)?;
        for (text, field) in texts.iter().zip(classification.fields.iter_mut()) {
            self.reconcile(text, &mut field.entities);
        }
        Ok(classification)
    }
}
