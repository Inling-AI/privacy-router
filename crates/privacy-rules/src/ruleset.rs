use crate::{Action, Rule, RuleId, RuleSource};
use privacy_filter::{Entity, Registration};

/// 单条实体的判定结果。`rule` 为 `None` 表示没有任何规则命中，此时采用兜底动作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub action: Action,
    pub rule: Option<RuleId>,
}

impl Decision {
    pub fn is_release(&self) -> bool {
        self.action == Action::Release
    }
}

/// 有序规则集合。判定为「首个命中的规则胜出」。
///
/// 未命中时默认放行：只有启用且满足全部条件的抹去规则才能拦截实体。
/// 处理失败仍由代理拒绝请求，与规则成功评估后的兜底动作无关。
pub struct RuleSet {
    rules: Vec<Rule>,
    fallback: Action,
}

impl RuleSet {
    /// 管理员的确定性登记：登记的原文、它被判成的类别与方向。
    ///
    /// 这些判定不依赖模型的实体提议——登记的原文只要出现在文本里就是命中，出现几次算几次。
    /// 停用的规则不在此列：停用是运行状态，改的正是「还算不算数」。
    pub fn registrations(&self) -> Vec<Registration> {
        self.rules
            .iter()
            .filter(|rule| rule.enabled && rule.source == RuleSource::Operator)
            .filter_map(|rule| {
                let pattern = rule.condition.exact_keyword()?;
                let group = rule.category?;
                Some(Registration::new(
                    pattern.text.clone(),
                    group,
                    pattern.case_sensitive,
                ))
            })
            .collect()
    }

    /// 全部登记过的原文，不论方向、也不论当前是否启用。
    ///
    /// 内容池问的是「这段内容被人审过没有」：决定过就是审过。停用只是让决定不再生效，
    /// 不改变「已经决定过」这个事实——否则停用一条规则会让内容重新变回待审。
    pub fn registered_texts(&self) -> impl Iterator<Item = &str> {
        self.rules
            .iter()
            .filter(|rule| rule.source == RuleSource::Operator)
            .filter_map(|rule| rule.condition.exact_keyword())
            .map(|pattern| pattern.text.as_str())
    }

    /// 按优先级从高到低排列；同优先级按标识排序，保证判定与构造顺序无关。
    pub fn new(rules: impl IntoIterator<Item = Rule>) -> Self {
        let mut rules: Vec<Rule> = rules.into_iter().collect();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));
        Self {
            rules,
            fallback: Action::Release,
        }
    }

    pub fn with_fallback(mut self, fallback: Action) -> Self {
        self.fallback = fallback;
        self
    }

    /// 判定一条实体。停用的规则被跳过；没有规则命中时返回兜底动作。
    pub fn evaluate(&self, entity: &Entity) -> Decision {
        self.rules
            .iter()
            .filter(|rule| rule.enabled)
            .find(|rule| rule.condition.matches(entity))
            .map_or_else(
                || Decision {
                    action: self.fallback,
                    rule: None,
                },
                |rule| Decision {
                    action: rule.action,
                    rule: Some(rule.id.clone()),
                },
            )
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// 一条规则的优先级。判定只回答「哪条规则说了算」，优先级要比较时从这里读，
    /// 调用方不另存一份规则表。
    pub fn priority_of(&self, id: &RuleId) -> Option<i32> {
        self.rules
            .iter()
            .find(|rule| rule.id == *id)
            .map(|rule| rule.priority)
    }

    /// 管理员为这段文本留下的全部登记，按优先级从高到低。
    ///
    /// 登记的身份是原文本身，所以正常情况下这里至多一条。历史上「改主意」是往同一条文本上
    /// 再加一条反方向的规则，于是会有多条并存——它们仍然会被读出来，好让调用方收敛掉。
    pub fn registrations_of(&self, text: &str) -> impl Iterator<Item = &Rule> {
        let mut matching: Vec<&Rule> = self
            .rules
            .iter()
            .filter(|rule| {
                rule.source == RuleSource::Operator && rule.condition.is_exact_keyword_match(text)
            })
            .collect();
        // 优先级最高的排最前；同优先级按标识排序，结果不取决于插入顺序。
        matching.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        matching.into_iter()
    }

    /// 这段文本**算数**的那条登记；`None` 表示还没登记过。
    ///
    /// 多条并存时算数的是优先级最高的那条——判定路径本来就是这么裁决的，列表与去重必须
    /// 用同一个答案，否则界面会把「已登记：放行」写在一段实际被抹去的内容上。
    pub fn operator_decision(&self, text: &str) -> Option<&Rule> {
        self.registrations_of(text).next()
    }

    pub fn fallback(&self) -> Action {
        self.fallback
    }
}
