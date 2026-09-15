//! 控制台类别名与识别方式声明的一致性。
//!
//! 类别名里的「确定性规则 / 智能识别」不是文案装饰，它是识别方式的呈现。事实只有一个来源：
//! 过滤器在 `FilterKind` 里声明自己认什么、命中意味着什么。控制台只负责把它翻译成文字，
//! 本文件把两者钉在一起——任何一边单独改了，这里就不再通过。
//!
//! 类别名单同样不重抄：遍历 `EntityGroup::all()`，新增类别自动纳入检查。

use privacy_filter::{EntityGroup, FilterKind, Filters};
use rstest::rstest;
use serde_json::Value;

const CHINESE: &str = include_str!("../../../apps/console/lib/i18n/zh_CN.i18n.json");
const ENGLISH: &str = include_str!("../../../apps/console/lib/i18n/en.i18n.json");

/// 类别名前缀说的是「这次判定由谁负责」。
///
/// 形状即结论的类别标 `确定性规则`：模型在同一类别上的命中会被作废。模型仍在该类别上发言的
/// 标 `智能识别`——手机号就是这种：国内移动号码的形状规则只覆盖一部分写法，其余写法仍然靠模型。
fn prefixes(kind: Option<FilterKind>) -> (&'static str, &'static str) {
    match kind {
        Some(FilterKind::Authoritative) => ("确定性规则·", "Deterministic · "),
        Some(FilterKind::Additive | FilterKind::Probabilistic) | None => ("智能识别·", "Model · "),
    }
}

#[rstest]
fn console_entity_names_state_how_the_category_is_recognized() {
    let recognition = Filters::deterministic();
    let chinese: Value = serde_json::from_str(CHINESE).expect("中文语言文件可解析");
    let english: Value = serde_json::from_str(ENGLISH).expect("英文语言文件可解析");

    for group in EntityGroup::all() {
        let key = console_key(group);
        let (chinese_prefix, english_prefix) = prefixes(recognition.kind_of(group));

        for (language, document, prefix) in [
            ("zh_CN", &chinese, chinese_prefix),
            ("en", &english, english_prefix),
        ] {
            let name = document["model"]["entity"][key.as_str()]
                .as_str()
                .unwrap_or_else(|| panic!("{language} 缺少类别名 {key}"));
            assert!(
                name.starts_with(prefix),
                "{language} 的 {key} 没有说明识别方式：应以 {prefix} 开头，实际是 {name}"
            );
        }
    }
}

/// 控制台把接口上的名称写成小驼峰：`private_email` → `privateEmail`。
fn console_key(group: EntityGroup) -> String {
    let wire = serde_json::to_value(group).expect("类别可序列化");
    let wire = wire.as_str().expect("类别名是字符串");

    let mut key = String::with_capacity(wire.len());
    let mut capitalize = false;
    for character in wire.chars() {
        if character == '_' {
            capitalize = true;
            continue;
        }
        key.push(if capitalize {
            character.to_ascii_uppercase()
        } else {
            character
        });
        capitalize = false;
    }
    key
}
