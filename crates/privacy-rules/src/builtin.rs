//! 随版本发布的最小安全基线。示例地址、占位词和模型噪声不属于系统不变式，不做预置。
//!
//! 每个受保护的类别只有一条规则，门槛只写在识别阶段尚未定死的地方：命中足够可信就抹去，
//! 概率类别还要足够长。形状由确定性过滤器定义的类别（[`privacy_filter::FilterKind::Authoritative`]）
//! 命中的就是事实——过滤器把得分写成常数 1.0，规则再问一次置信度只是把这个常数重算一遍，
//! 所以这些类别只问「是不是这个类别」。不满足门槛的命中不认领任何规则，交给兜底策略——
//! 因此这里不需要「短匹配放行」这类反向规则。
//! 日期不设档位：模型对它的判定噪声最大，短日期尤其不可靠，是否抹去交给兜底策略与
//! 管理员自己的规则决定。

use crate::{
    Action, ComparisonOperator, Rule, RuleExpression, RuleId, RuleSet, RuleSource, priority,
};
use privacy_filter::{EntityGroup, FilterKind, Filters};

const MIN_CONFIDENCE: f64 = 0.8;

#[derive(Clone, Copy)]
struct ProtectionProfile {
    group: EntityGroup,
    /// 抹去门槛：命中文本的字符数必须**大于**这个值。
    ///
    /// `None` 表示这个类别的形状由确定性识别保证，长度不再是判据。
    longer_than: Option<usize>,
}

impl ProtectionProfile {
    /// `None` 表示这一类别不设预置档位（日期）：是否抹去完全由兜底策略决定。
    ///
    /// 逐个列出全部类别，新增类别时编译器会强制在这里做出选择。
    fn for_group(group: EntityGroup) -> Option<Self> {
        let longer_than = match group {
            EntityGroup::AccountNumber => Some(3),
            EntityGroup::PrivateAddress => Some(4),
            EntityGroup::PrivatePerson => Some(4),
            EntityGroup::PrivatePhone => Some(6),
            EntityGroup::PrivateUrl => Some(4),
            EntityGroup::Secret => Some(7),
            EntityGroup::PrivateDate => return None,
            // 下面这些类别由确定性过滤器给出完整形状，规则再筛长度是多余的一道猜测。
            EntityGroup::BankCard
            | EntityGroup::CreditCode
            | EntityGroup::IdCard
            | EntityGroup::IpAddress
            | EntityGroup::MacAddress
            | EntityGroup::PrivateEmail => None,
        };
        Some(Self { group, longer_than })
    }

    /// `recognition` 是识别阶段的过滤器集合——「这个类别的命中是不是事实」只有它知道，
    /// 规则不再自己维护一份确定性类别的名单。
    fn into_rule(self, recognition: &Filters) -> Rule {
        let id = RuleId::new(format!("builtin.redact.{}", self.group)).expect("内置标识非空");
        let mut conditions = vec![RuleExpression::entity(self.group)];
        if recognition.kind_of(self.group) != Some(FilterKind::Authoritative) {
            conditions.push(RuleExpression::confidence(
                ComparisonOperator::Greater,
                MIN_CONFIDENCE,
            ));
        }
        conditions.extend(self.longer_than.map(|longer_than| {
            RuleExpression::character_length(ComparisonOperator::Greater, longer_than)
        }));
        Rule {
            name: id.to_string(),
            id,
            priority: priority::DEFAULT_REDACT,
            condition: RuleExpression::all(conditions),
            action: Action::Redact,
            enabled: true,
            source: RuleSource::Builtin,
            // 内置规则认的是类别，条件本身就是类别的定义；没有「这段原文」可登记。
            category: None,
        }
    }
}

impl RuleSet {
    pub fn builtin() -> Self {
        let recognition = Filters::deterministic();
        Self::new(
            EntityGroup::all()
                .filter_map(ProtectionProfile::for_group)
                .map(|profile| profile.into_rule(&recognition)),
        )
    }
}
