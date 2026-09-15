//! 文本中的不透明图片与实体边界。坐标始终使用原文 UTF-8 字节偏移。
use base64::Engine;
use std::{borrow::Cow, ops::Range};

pub(crate) struct Content<'a> {
    text: &'a str,
    images: Vec<Range<usize>>,
}

impl<'a> Content<'a> {
    pub fn new(text: &'a str) -> Self {
        let mut images = Vec::new();
        // 仅识别有图片签名的标准 base64，不把高熵凭据当成二进制放行。
        for signature in ["iVBORw0KGgo", "/9j/", "R0lGODdh", "R0lGODlh"] {
            for (start, _) in text.match_indices(signature) {
                if start > 0 && Self::base64(text.as_bytes()[start - 1]) {
                    continue;
                }
                let length = text.as_bytes()[start..]
                    .iter()
                    .take_while(|&&b| Self::base64(b))
                    .count();
                if length >= 32 && Self::image(&text[start..start + length]) {
                    images.push(start..start + length);
                }
            }
        }
        images.sort_by_key(|range| range.start);
        Self { text, images }
    }

    fn base64(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')
    }

    fn image(encoded: &str) -> bool {
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
            return false;
        };
        (bytes.starts_with(b"\x89PNG\r\n\x1a\n")
            && bytes.get(12..16) == Some(b"IHDR")
            && bytes.ends_with(b"IEND\xaeB`\x82"))
            || (bytes.starts_with(b"\xff\xd8\xff") && bytes.ends_with(b"\xff\xd9"))
            || ((bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"))
                && bytes.ends_with(b";"))
    }

    pub fn inference_text(&self) -> Cow<'a, str> {
        if self.images.is_empty() {
            return Cow::Borrowed(self.text);
        }
        let mut text = self.text.to_owned();
        for range in &self.images {
            text.replace_range(range.clone(), &" ".repeat(range.len()));
        }
        Cow::Owned(text)
    }

    /// 跨越图片的实体只能修改图片之外的文本，不能在编码中插入占位词。
    pub fn text_ranges(&self, range: Range<usize>) -> Vec<Range<usize>> {
        let mut ranges = Vec::new();
        let mut cursor = range.start;
        for image in &self.images {
            if image.end <= cursor || image.start >= range.end {
                continue;
            }
            if cursor < image.start {
                ranges.push(cursor..image.start);
            }
            cursor = image.end.min(range.end);
        }
        if cursor < range.end {
            ranges.push(cursor..range.end);
        }
        ranges
    }

    /// 向外补齐被切开的词法单元；不跨越引号、空白或容器分隔符。
    pub fn complete(&self, range: Range<usize>, credential: bool) -> Range<usize> {
        let token = |c: char| {
            c.is_ascii_alphanumeric()
                || matches!(c, '_' | '-' | '.' | '@' | '%' | '+' | '~')
                || (credential
                    && matches!(
                        c,
                        '/' | ':' | '=' | '?' | '&' | '#' | '!' | '$' | '^' | '*' | '|'
                    ))
        };
        let start = self.text[..range.start]
            .char_indices()
            .rev()
            .take_while(|(_, c)| token(*c))
            .last()
            .map_or(range.start, |(i, _)| i);
        let end = self.text[range.end..]
            .char_indices()
            .take_while(|(_, c)| token(*c))
            .last()
            .map_or(range.end, |(i, c)| range.end + i + c.len_utf8());
        start..end
    }
}
