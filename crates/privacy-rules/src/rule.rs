use crate::{Action, EntityGroup, Error, Pattern, Result, RuleExpression, RuleFilter};
use serde::{Deserialize, Serialize};

/// 优先级的语义分层。数值越大越先判定。
///
/// 分层是判定的对外契约——强制抹去不得被放行关键词绕过——因此层的取值只在这里定义，
/// 内置规则与管理员登记都从同一组常量取值。
pub mod priority {
    /// 满足类别、长度和置信度的默认保护，可被管理员显式放行覆盖。
    pub const DEFAULT_REDACT: i32 = 100;
    /// 密钥、账号一类的强制抹去。
    pub const FORCE_REDACT: i32 = 1000;
    /// 命中放行关键词的片段。
    pub const RELEASE_KEYWORD: i32 = 500;
    /// 低置信度的短碎片。
    pub const RELEASE_NOISE: i32 = 400;
}

/// 规则标识。内置规则使用稳定的点分标识，控制台创建的规则使用生成的标识。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuleId(String);

impl RuleId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(Error::EmptyRuleId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// 规则来源。仅用于说明规则最初如何产生，不赋予任何规则修改特权。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSource {
    Builtin,
    Console,
    /// 管理员在内容池逐条登记的决定。方向由 [`Rule::action`] 给出，来源只说「谁登记的」。
    ///
    /// 线上取值保持 `approval`：它先于「禁止」出现，已经是落库的历史取值，改它要重建规则表；
    /// 这个取舍只在这里说明一次，界面与调用方读到的都是本类型本身。
    #[serde(rename = "approval")]
    Operator,
}

/// 一条判定规则。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub id: RuleId,
    pub name: String,
    /// 数值越大越先判定。同一优先级按 [`RuleId`] 排序，使结果与输入顺序无关。
    pub priority: i32,
    pub condition: RuleExpression,
    pub action: Action,
    pub enabled: bool,
    pub source: RuleSource,
    /// 管理员登记这段原文时它被判成的类别。
    ///
    /// 类别不参与匹配——登记的身份是原文本身，匹配永远是「这段原文相等」——它只回答
    /// 「这段原文是什么东西」：抹去后的占位词、内容池里的类别标签都从这里读。内置规则
    /// 的类别写在条件里（它们认的是类别），因此这里留空。
    pub category: Option<EntityGroup>,
}

impl Rule {
    /// 管理员对一段文本登记的决定：该文本整段命中即执行 `action`。
    ///
    /// 登记不是特权：它与同动作的内置规则走同一套判定，只有优先级不同——放行与放行关键词
    /// 同层，禁止与强制抹去同层，因此禁止压得住放行。名称刻意不含被登记的文本，以免规则
    /// 列表被当作内容泄露的渠道。
    fn operator_decision(
        id: RuleId,
        action: Action,
        text: impl Into<String>,
        category: EntityGroup,
    ) -> Self {
        let (name, priority) = Self::operator_profile(action);
        Self {
            id,
            name: name.to_owned(),
            priority,
            condition: RuleExpression::keyword(Pattern::exact(text)),
            action,
            enabled: true,
            source: RuleSource::Operator,
            category: Some(category),
        }
    }

    /// 登记的方向决定名称与优先级：放行与放行关键词同层，禁止压过它。
    fn operator_profile(action: Action) -> (&'static str, i32) {
        let (name, priority) = match action {
            Action::Release => ("approval.release", priority::RELEASE_KEYWORD),
            Action::Redact => ("approval.redact", priority::FORCE_REDACT),
        };
        (name, priority)
    }

    /// 管理员放行一段文本：之后相同内容直接透传，除非有更高优先级的抹去规则认领它。
    pub fn operator_release(id: RuleId, text: impl Into<String>, category: EntityGroup) -> Self {
        Self::operator_decision(id, Action::Release, text, category)
    }

    /// 管理员禁止一段文本：之后相同内容强制抹去，放行关键词不能绕过。
    pub fn operator_redact(id: RuleId, text: impl Into<String>, category: EntityGroup) -> Self {
        Self::operator_decision(id, Action::Redact, text, category)
    }

    /// 改判：同一段文本换一个方向。
    ///
    /// 登记的身份是文本，方向不是，所以改判改的是同一条规则。另立一条反方向的规则会让列表
    /// 里同时躺着同一段文本的放行与禁止，判定虽然仍按优先级决出胜负，但没人能一眼看出哪条算数。
    pub fn redecided(self, action: Action) -> Self {
        let (name, priority) = Self::operator_profile(action);
        Self {
            name: name.to_owned(),
            priority,
            action,
            ..self
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.source == RuleSource::Operator && self.category.is_none() {
            return Err(Error::MissingCategory(self.id.clone()));
        }
        if let Some(pattern) = self.condition.exact_keyword()
            && pattern.text.trim().is_empty()
        {
            return Err(Error::EmptyPattern(self.id.clone()));
        }
        let mut error = None;
        self.condition.visit_filters(&mut |filter| match filter {
            RuleFilter::Keyword { pattern } if pattern.text.trim().is_empty() => {
                error.get_or_insert_with(|| Error::EmptyPattern(self.id.clone()));
            }
            RuleFilter::Confidence { value, .. }
                if !value.is_finite() || !(0.0..=1.0).contains(value) =>
            {
                error.get_or_insert_with(|| Error::InvalidConfidence {
                    rule: self.id.clone(),
                    value: *value,
                });
            }
            _ => {}
        });
        if let Some(error) = error {
            return Err(error);
        }
        Ok(())
    }
}
