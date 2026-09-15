use privacy_filter::{Entity, EntityGroup};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// 数值过滤器的比较方式。比较是严格的；包含边界时显式组合 Equal。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    Greater,
    Equal,
    Less,
}

impl ComparisonOperator {
    fn matches<T: PartialOrd + PartialEq>(self, actual: T, expected: T) -> bool {
        match self {
            Self::Greater => actual > expected,
            Self::Equal => actual == expected,
            Self::Less => actual < expected,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternKind {
    Exact,
    Substring,
    Glob,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pattern {
    pub text: String,
    pub kind: PatternKind,
    pub case_sensitive: bool,
}

impl Pattern {
    pub fn exact(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: PatternKind::Exact,
            case_sensitive: false,
        }
    }

    pub fn substring(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: PatternKind::Substring,
            case_sensitive: false,
        }
    }

    pub fn matches(&self, text: &str) -> bool {
        match self.kind {
            PatternKind::Exact => self.folded(text) == self.folded(&self.text),
            PatternKind::Substring => {
                let haystack = self.folded(text);
                haystack.contains(self.folded(&self.text).as_ref())
            }
            PatternKind::Glob => {
                let pattern = self.folded(&self.text);
                let text = self.folded(text);
                Self::glob_matches(
                    &pattern.chars().collect::<Vec<_>>(),
                    &text.chars().collect::<Vec<_>>(),
                )
            }
        }
    }

    fn folded<'a>(&self, value: &'a str) -> Cow<'a, str> {
        if self.case_sensitive {
            Cow::Borrowed(value)
        } else {
            Cow::Owned(value.to_lowercase())
        }
    }

    fn glob_matches(pattern: &[char], text: &[char]) -> bool {
        let (mut pattern_index, mut text_index) = (0, 0);
        let (mut star, mut resume) = (None, 0);
        while text_index < text.len() {
            if pattern_index < pattern.len()
                && (pattern[pattern_index] == '?' || pattern[pattern_index] == text[text_index])
            {
                pattern_index += 1;
                text_index += 1;
            } else if pattern_index < pattern.len() && pattern[pattern_index] == '*' {
                star = Some(pattern_index);
                resume = text_index;
                pattern_index += 1;
            } else if let Some(position) = star {
                pattern_index = position + 1;
                resume += 1;
                text_index = resume;
            } else {
                return false;
            }
        }
        pattern[pattern_index..]
            .iter()
            .all(|&character| character == '*')
    }
}

/// 一个不可再分的匹配条件。逻辑组合只存在于 RuleExpression。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RuleFilter {
    Entity {
        group: EntityGroup,
    },
    Keyword {
        pattern: Pattern,
    },
    CharacterLength {
        operator: ComparisonOperator,
        value: usize,
    },
    Confidence {
        operator: ComparisonOperator,
        value: f64,
    },
}

impl RuleFilter {
    pub fn matches(&self, entity: &Entity) -> bool {
        match self {
            Self::Entity { group } => entity.entity_group == *group,
            Self::Keyword { pattern } => pattern.matches(&entity.word),
            Self::CharacterLength { operator, value } => {
                operator.matches(entity.word.chars().count(), *value)
            }
            Self::Confidence { operator, value } => operator.matches(entity.score, *value),
        }
    }
}

/// 规则的布尔表达式。All/Any 可递归嵌套，表达任意 AND/OR 组合。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RuleExpression {
    All { conditions: Vec<RuleExpression> },
    Any { conditions: Vec<RuleExpression> },
    Filter { filter: RuleFilter },
}

impl RuleExpression {
    pub fn all(conditions: impl IntoIterator<Item = Self>) -> Self {
        Self::All {
            conditions: conditions.into_iter().collect(),
        }
    }

    pub fn any(conditions: impl IntoIterator<Item = Self>) -> Self {
        Self::Any {
            conditions: conditions.into_iter().collect(),
        }
    }

    pub fn filter(filter: RuleFilter) -> Self {
        Self::Filter { filter }
    }

    pub fn entity(group: EntityGroup) -> Self {
        Self::filter(RuleFilter::Entity { group })
    }

    pub fn keyword(pattern: Pattern) -> Self {
        Self::filter(RuleFilter::Keyword { pattern })
    }

    pub fn character_length(operator: ComparisonOperator, value: usize) -> Self {
        Self::filter(RuleFilter::CharacterLength { operator, value })
    }

    pub fn confidence(operator: ComparisonOperator, value: f64) -> Self {
        Self::filter(RuleFilter::Confidence { operator, value })
    }

    pub fn matches(&self, entity: &Entity) -> bool {
        match self {
            Self::All { conditions } => conditions.iter().all(|item| item.matches(entity)),
            Self::Any { conditions } => conditions.iter().any(|item| item.matches(entity)),
            Self::Filter { filter } => filter.matches(entity),
        }
    }

    /// 表达式里那条「整段相等」的关键词，它就是这条登记的身份。
    ///
    /// 登记是围绕一段原文写的规则，因此它的关键词必然在表达式里；组合条件（例如
    /// 「这个类别 + 这段原文」）也照样能找出来，调用方不必知道条件被包了几层。
    pub fn exact_keyword(&self) -> Option<&Pattern> {
        match self {
            Self::Filter {
                filter: RuleFilter::Keyword { pattern },
            } if pattern.kind == PatternKind::Exact => Some(pattern),
            Self::All { conditions } | Self::Any { conditions } => {
                conditions.iter().find_map(Self::exact_keyword)
            }
            _ => None,
        }
    }

    pub fn is_exact_keyword_match(&self, text: &str) -> bool {
        self.exact_keyword()
            .is_some_and(|pattern| pattern.matches(text))
    }

    pub(crate) fn visit_filters(&self, visitor: &mut impl FnMut(&RuleFilter)) {
        match self {
            Self::All { conditions } | Self::Any { conditions } => {
                for condition in conditions {
                    condition.visit_filters(visitor);
                }
            }
            Self::Filter { filter } => visitor(filter),
        }
    }
}
