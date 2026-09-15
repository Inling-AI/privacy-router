//! 过滤器：文本到隐私命中的识别能力。
//!
//! 模型是一个过滤器，确定性规则是另一些过滤器。过滤器之间互不知晓、互不调用，谁也不给谁兜底；
//! 每个过滤器用 [`FilterKind`] 声明自己的命中意味着什么，合并规则只在 [`Filters`] 里写一遍。
//! 新增一种识别方式就是新增一个 [`Filter`] 实现。

use crate::pattern;
use crate::{Entity, EntityGroup};

/// 一个过滤器的命中意味着什么。
///
/// 这是过滤器对外声明的参数：决定命中的得分语义、规则是否还要筛形状、以及模型在同一类别上
/// 还有没有发言权。由过滤器自己声明，消费方不再另行推断。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    /// 概率判断：命中带置信度，可能错。模型属于这一类。
    Probabilistic,
    /// 形状就是这个类别的定义：概率类过滤器在该类别上的命中全部作废。
    Authoritative,
    /// 形状规则只覆盖一部分写法：与之重叠的概率类命中让位，其余仍然有效。
    Additive,
}

/// 找出文本中的隐私命中。
///
/// 实现是**纯函数式**的：不持有跨请求状态、不依赖外部环境，同一段文本永远给出同一份命中。
/// 偏移指向入参文本的字节位置，`word` 等于该范围内的原文。
pub trait Filter: Send + Sync {
    /// 这个过滤器给出命中的类别。
    fn group(&self) -> EntityGroup;

    /// 命中的性质。
    fn kind(&self) -> FilterKind;

    /// 找出文本中的全部命中，按出现顺序返回且互不重叠。
    fn recognize(&self, text: &str) -> Vec<Entity>;
}

/// 引用转发，使共享的内置过滤器（`LazyLock<PatternFilter>`）可以直接作为过滤器使用。
impl<T: Filter + ?Sized> Filter for &T {
    fn group(&self) -> EntityGroup {
        (**self).group()
    }

    fn kind(&self) -> FilterKind {
        (**self).kind()
    }

    fn recognize(&self, text: &str) -> Vec<Entity> {
        (**self).recognize(text)
    }
}

/// 一组过滤器与它们同模型命中的合并规则。
///
/// 合并只发生在识别结果层面：判定与替换照旧，因此过滤器再多也不改变规则引擎的输入形状。
pub struct Filters {
    filters: Vec<Box<dyn Filter>>,
}

impl Filters {
    /// 只有内置的确定性过滤器。
    pub fn deterministic() -> Self {
        Self {
            filters: pattern::builtin()
                .into_iter()
                .map(|filter| Box::new(filter) as Box<dyn Filter>)
                .collect(),
        }
    }

    /// 追加一个过滤器。自定义识别方式从这里接入，不必参与别的过滤器的实现。
    pub fn push(&mut self, filter: impl Filter + 'static) {
        self.filters.push(Box::new(filter));
    }

    /// 这个类别的确定性识别由哪一类过滤器负责；`None` 表示只有模型在认它。
    ///
    /// 这是「这个类别怎么被识别」的唯一事实来源：命中是不是事实、规则要不要再问置信度、
    /// 界面该怎么称呼它，都从这里读，消费方不另建一份名单。
    pub fn kind_of(&self, group: EntityGroup) -> Option<FilterKind> {
        self.filters
            .iter()
            .find(|filter| filter.group() == group)
            .map(|filter| filter.kind())
    }

    /// 这段文本的全部确定性命中，重叠处按「证据更具体者优先」取舍。
    ///
    /// 18 位数字既可能是身份证也可能是银行卡，`::` 开头的 IPv6 里也藏着一段合法 MAC。
    /// 两者都命中时只能报一个类别，否则同一段文本会得到两条互相矛盾的判定。
    pub fn recognize(&self, text: &str) -> Vec<Entity> {
        let mut candidates: Vec<Entity> = self
            .filters
            .iter()
            .filter(|filter| filter.kind() != FilterKind::Probabilistic)
            .flat_map(|filter| filter.recognize(text))
            .collect();
        candidates
            .sort_by_key(|entity| (entity.entity_group.specificity(), entity.start, entity.end));
        let mut accepted: Vec<Entity> = Vec::new();
        for entity in candidates {
            if accepted
                .iter()
                .any(|other| entity.start < other.end && other.start < entity.end)
            {
                continue;
            }
            accepted.push(entity);
        }
        accepted.sort_by_key(|entity| (entity.start, entity.end));
        accepted
    }

    /// 这段文本的最终命中：确定性命中与概率类（模型）命中按类别合并。
    ///
    /// 两处取舍都指向同一条规则——**证据更具体的一方说了算**：
    ///
    /// * 形状即定义的类别（邮箱），模型的同类命中不是命中，否则 `-rwxr-xr-x@` 这类误报会被
    ///   重新带回来。
    /// * 其余确定性类别（手机号），与确定性命中重叠的模型命中让位：命中既定，模型在同一段
    ///   文本上的分数不再有发言权。
    pub fn combine(&self, text: &str, from_model: Vec<Entity>) -> Vec<Entity> {
        let recognized = self.recognize(text);
        let mut combined: Vec<Entity> = from_model
            .into_iter()
            .filter(|entity| {
                self.kind_of(entity.entity_group) != Some(FilterKind::Authoritative)
                    && !recognized.iter().any(|hit| {
                        hit.entity_group == entity.entity_group
                            && hit.start < entity.end
                            && entity.start < hit.end
                    })
            })
            .collect();
        combined.extend(recognized);
        combined.sort_by_key(|entity| (entity.start, entity.end));
        combined
    }
}
