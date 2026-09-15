//! 各测试目标共用的领域对象构造。放在子目录下，避免被当作独立测试目标编译。

use privacy_filter::{Entity, EntityGroup};
use privacy_rules::{Action, Rule, RuleExpression, RuleId, RuleSource};
use rstest::fixture;

/// 以「类别 + 置信度 + 文本」构造实体；偏移由文本长度导出，判定不读取它们。
#[fixture]
pub fn entity() -> impl Fn(EntityGroup, f64, &str) -> Entity {
    |entity_group, score, word| Entity {
        entity_group,
        score,
        start: 0,
        end: word.len(),
        word: word.to_owned(),
    }
}

#[fixture]
pub fn rule() -> impl Fn(&str, i32, RuleExpression, Action) -> Rule {
    |id, priority, condition, action| Rule {
        id: RuleId::new(id).expect("测试标识非空"),
        name: id.to_owned(),
        priority,
        condition,
        action,
        enabled: true,
        source: RuleSource::Console,
        category: None,
    }
}
