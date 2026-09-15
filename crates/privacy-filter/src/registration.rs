//! 管理员登记过的原文：出现即命中。
//!
//! 登记与形状识别不是一回事。形状过滤器回答的是「这段文字像不像邮箱」；登记回答的是
//! 「这段原文已经被人工认定过」。因此登记的命中没有概率可言：得分恒为
//! [`PatternFilter::SCORE`]，出现几次就命中几次，与模型有没有注意到它、把它切成多长无关。

use crate::filter::{Filter, FilterKind};
use crate::{Entity, EntityGroup, PatternFilter};
use std::ops::Range;

/// 一段被登记过的原文、它所属的类别，以及比较是否区分大小写。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    text: String,
    group: EntityGroup,
    case_sensitive: bool,
}

impl Registration {
    pub fn new(text: impl Into<String>, group: EntityGroup, case_sensitive: bool) -> Self {
        Self {
            text: text.into(),
            group,
            case_sensitive,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn group(&self) -> EntityGroup {
        self.group
    }

    /// 这段原文在文本里出现的每一处。
    ///
    /// 按字符窗口比对而不是按字节搜索：`to_lowercase` 可能把一个字符折成多个字符，按字符数
    /// 取窗口，返回的偏移就始终落在原文字符边界上。返回值是原文的字节范围。
    fn occurrences(&self, text: &str) -> Vec<Range<usize>> {
        let width = self.text.chars().count();
        if width == 0 {
            return Vec::new();
        }
        let head = self.text.chars().next().expect("宽度非零即首字符存在");
        let boundaries: Vec<usize> = text.char_indices().map(|(offset, _)| offset).collect();
        let mut found = Vec::new();
        for (position, &start) in boundaries.iter().enumerate() {
            let end = match boundaries.get(position + width) {
                Some(&end) => end,
                None if position + width == boundaries.len() => text.len(),
                None => break,
            };
            // 首字符先筛一遍：长登记在长文本里的逐窗口折写很贵，而绝大多数窗口第一个字符就对不上。
            let window = &text[start..end];
            if !window
                .chars()
                .next()
                .is_some_and(|character| self.head_matches(character, head))
            {
                continue;
            }
            if self.matches(window) {
                found.push(start..end);
            }
        }
        found
    }

    fn head_matches(&self, character: char, head: char) -> bool {
        if self.case_sensitive {
            character == head
        } else {
            character.to_lowercase().eq(head.to_lowercase())
        }
    }

    fn matches(&self, window: &str) -> bool {
        if self.case_sensitive {
            window == self.text
        } else {
            window.to_lowercase() == self.text.to_lowercase()
        }
    }
}

impl Filter for Registration {
    fn group(&self) -> EntityGroup {
        self.group
    }

    /// 登记只覆盖自己这段原文的写法，其他写法照旧由形状过滤器与模型说话。
    fn kind(&self) -> FilterKind {
        FilterKind::Additive
    }

    fn recognize(&self, text: &str) -> Vec<Entity> {
        self.occurrences(text)
            .into_iter()
            .map(|range| Entity {
                entity_group: self.group,
                score: PatternFilter::SCORE,
                start: range.start,
                end: range.end,
                word: text[range].to_owned(),
            })
            .collect()
    }
}
