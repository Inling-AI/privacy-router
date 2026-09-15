//! 隐私判定的规则领域：条件、优先级与内置默认策略。
//!
//! 本 crate 只做纯计算：给定一条已识别实体与一组规则，得出「放行」或「抹去」。
//! 它不读写数据库、不解析协议、不记录日志，也不关心调用方是代理还是控制台。
#![doc = include_str!("../README.md")]

mod action;
mod builtin;
mod condition;
mod rule;
mod ruleset;

pub use action::Action;
pub use condition::{ComparisonOperator, Pattern, PatternKind, RuleExpression, RuleFilter};
pub use rule::{Rule, RuleId, RuleSource, priority};
pub use ruleset::{Decision, RuleSet};

pub use privacy_filter::EntityGroup;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("rule id must not be empty")]
    EmptyRuleId,
    #[error("rule '{0}' has an empty keyword pattern")]
    EmptyPattern(RuleId),
    #[error("operator rule '{0}' does not say which category it registered")]
    MissingCategory(RuleId),
    #[error("rule '{rule}' has an invalid confidence value: {value}")]
    InvalidConfidence { rule: RuleId, value: f64 },
}

pub type Result<T> = std::result::Result<T, Error>;
