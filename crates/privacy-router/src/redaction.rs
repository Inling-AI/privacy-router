//! 判定结果到替换文本的落地。
//!
//! 这里是隐私保证的最后一步：判为抹去的片段必须在转发前从文本中消失。规则判定本身由
//! `privacy-rules` 负责，本模块只负责按字节偏移执行替换并产出可审计的记录。

use privacy_filter::Entity;
use privacy_rules::{Action, Decision, EntityGroup, RuleSet};
use serde::{Deserialize, Serialize};

/// 一条命中的最终处置，连同它在原文中的位置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecidedSpan {
    pub entity_group: EntityGroup,
    pub score: f64,
    pub byte_start: usize,
    pub byte_end: usize,
    pub original_text: String,
    pub action: Action,
    pub matched_rule_id: Option<String>,
    /// 做出这条决定的规则优先级。没有规则命中时是最低优先级：兜底动作不是一条决定，
    /// 它压不倒任何规则，也不会被当成规则参与比较。
    pub priority: i32,
}

/// 兜底动作的优先级：低于任何一条真实规则。
const FALLBACK_PRIORITY: i32 = i32::MIN;

/// 一段文本的脱敏结果。
#[derive(Debug, Clone, PartialEq)]
pub struct Redaction {
    pub redacted: String,
    pub spans: Vec<DecidedSpan>,
}

impl Redaction {
    /// 是否真的改动了文本。未改动的片段不必进入内容池。
    pub fn changed(&self) -> bool {
        self.spans.iter().any(|span| span.action == Action::Redact)
    }

    /// 重叠窗口可能对同一位置给出不同类别；所有抹去决策的并集都必须消失。
    /// 放行只保留自身的审计决策，不能遮蔽另一个实体的抹去范围。
    fn from_spans(text: &str, spans: Vec<DecidedSpan>) -> Self {
        // 两个方向都可以压过对方，裁决标准只有一个：谁的规则优先级高，谁就是最终结果。
        // 管理员登记的放行因此压得住模型的抹去判定——模型说这段是密钥，管理员说这段要放行，
        // 以管理员的登记为准。被压过的决定不再出现在结果里：它没有生效，留着只会让审计说谎。
        let mut superseded = vec![false; spans.len()];
        for (index, span) in spans.iter().enumerate() {
            let overruled = span.matched_rule_id.is_some()
                && spans.iter().enumerate().any(|(other, competitor)| {
                    other != index
                        && competitor.matched_rule_id.is_some()
                        && competitor.action != span.action
                        && competitor.priority > span.priority
                        && competitor.byte_start < span.byte_end
                        && span.byte_start < competitor.byte_end
                });
            superseded[index] = overruled;
        }
        let mut remaining = Vec::with_capacity(spans.len());
        for (index, span) in spans.into_iter().enumerate() {
            if !superseded[index] {
                remaining.push(span);
            }
        }
        let mut spans = remaining;
        spans.sort_by_key(|span| (span.byte_start, span.byte_end));
        let mut ranges: Vec<(usize, usize, EntityGroup)> = Vec::new();
        for span in spans.iter().filter(|span| span.action == Action::Redact) {
            if let Some(previous) = ranges.last_mut()
                && span.byte_start < previous.1
            {
                previous.1 = previous.1.max(span.byte_end);
            } else {
                ranges.push((span.byte_start, span.byte_end, span.entity_group));
            }
        }
        let mut redacted = text.to_owned();
        for (start, end, group) in ranges.into_iter().rev() {
            redacted.replace_range(start..end, &placeholder(group));
        }
        Self { redacted, spans }
    }
}

/// 抹去后的占位文本。类别名与模型标签一致，便于上游与人工都看懂被去掉的是什么。
pub fn placeholder(group: EntityGroup) -> String {
    format!("<redacted_{group}>")
}

/// 对一段文本执行判定与替换。
///
/// 替换自右向左进行，避免前面的替换让后面的字节偏移失效。判定与替换共用同一份
/// [`Entity`] 偏移，不做子串搜索，因此同一文本中重复出现的值各自落在自己的位置上。
pub fn redact(text: &str, entities: &[Entity], rules: &RuleSet) -> Redaction {
    let content = crate::content::Content::new(text);
    let mut decided: Vec<_> = entities
        .iter()
        .filter(|entity| valid_range(text, entity))
        .flat_map(|entity| {
            let (start, end) = trim_whitespace(text, entity.start, entity.end);
            let credential = matches!(
                entity.entity_group,
                EntityGroup::Secret | EntityGroup::PrivateEmail | EntityGroup::PrivateUrl
            );
            // 登记的原文本身就是完整的单元：那是一个人手写下来的字符串，不需要再按词法补齐。
            // 补齐反而会把 `key=值` 里的 `key=` 一并吃进来，登记的那段原文就不再是它自己，
            // 针对它写的那条规则也就落空了。
            let range = if rules.operator_decision(&text[start..end]).is_some() {
                start..end
            } else {
                content.complete(start..end, credential)
            };
            content
                .text_ranges(range)
                .into_iter()
                .map(|range| {
                    let (start, end) = trim_whitespace(text, range.start, range.end);
                    let normalized = Entity {
                        start,
                        end,
                        word: text[start..end].to_owned(),
                        ..entity.clone()
                    };
                    let decision = rules.evaluate(&normalized);
                    DecidedSpan {
                        entity_group: entity.entity_group,
                        score: entity.score,
                        byte_start: start,
                        byte_end: end,
                        original_text: text[start..end].to_owned(),
                        action: decision.action,
                        priority: decision
                            .rule
                            .as_ref()
                            .and_then(|id| rules.priority_of(id))
                            .unwrap_or(FALLBACK_PRIORITY),
                        matched_rule_id: decision.rule.map(|id| id.to_string()),
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect();

    // 同一输入中已经确认抹去的完整凭据，在其他位置也按同一决策处理。
    // 不按短 token 扩散，不覆盖图片，也不把值作为另一个更长词的子串匹配。
    let confirmed = decided.clone();
    for span in confirmed.iter().filter(|span| {
        span.action == Action::Redact
            && matches!(
                span.entity_group,
                EntityGroup::Secret | EntityGroup::PrivateEmail | EntityGroup::PrivateUrl
            )
            && !span.original_text.is_empty()
    }) {
        for (start, value) in text.match_indices(&span.original_text) {
            // 补齐后的范围才是这处命中真正覆盖的文本。模型把值切短了一段（例如只给出
            // `-75d136…` 而掉了开头的 `ah`）时，补齐的结果正好回到完整的那个值；要求匹配范围
            // 逐字节等于已确认的片段，会把这种最该拦住的位置当成不匹配丢掉。
            let range = content.complete(start..start + value.len(), true);
            if content.text_ranges(range.clone()) != [range.clone()]
                || decided.iter().any(|other| {
                    other.action == Action::Redact
                        && other.byte_start <= range.start
                        && other.byte_end >= range.end
                })
            {
                continue;
            }
            decided.push(DecidedSpan {
                byte_start: range.start,
                byte_end: range.end,
                original_text: text[range.clone()].to_owned(),
                ..span.clone()
            });
        }
    }

    Redaction::from_spans(text, decided)
}

/// 判定一条实体但不改动文本；供只需要决策的场景使用。
pub fn decide(entity: &Entity, rules: &RuleSet) -> Decision {
    rules.evaluate(entity)
}

fn valid_range(text: &str, entity: &Entity) -> bool {
    entity.start <= entity.end
        && entity.end <= text.len()
        && text.is_char_boundary(entity.start)
        && text.is_char_boundary(entity.end)
}

/// 把两端空白排除在替换范围外，避免替换掉词前的空格；整段都是空白时保持原范围。
///
/// 模型返回的实体常带前导空格（例如 `" Harry Potter"`），直接替换会让相邻词黏在一起。
fn trim_whitespace(text: &str, start: usize, end: usize) -> (usize, usize) {
    let slice = &text[start..end];
    let leading = slice.len() - slice.trim_start().len();
    let trailing = slice.len() - slice.trim_end().len();
    if leading + trailing >= slice.len() {
        return (start, end);
    }
    (start + leading, end - trailing)
}
