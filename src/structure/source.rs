//! Compact only inert prose runs before CommonMark parsing, preserving source coordinates and all heading syntax.
use std::{borrow::Cow, ops::Range};

/// A transient parser input maps retained byte spans back to the immutable authoritative snapshot.
pub(super) struct Mapped<'a> {
    /// Ordinary documents keep a borrowed input; dense prose may omit redundant continuation lines.
    pub(super) text: Cow<'a, str>,
    /// Contiguous retained parser ranges and their original start; omitted prose cannot contain a heading.
    spans: Vec<(Range<usize>, usize)>,
}

impl<'a> Mapped<'a> {
    /// Avoid a second pass on small files; large inputs retain CommonMark's block context while reducing inert lines.
    pub(super) fn new(text: &'a str) -> Self {
        if text.len() <= 256 * 1024 {
            Self {
                text: Cow::Borrowed(text),
                spans: Vec::new(),
            }
        } else {
            Self::compact(text)
        }
    }

    /// Only equal-indent, punctuation-free nonempty lines may be elided after their first continuation.
    /// Inspect the whole physical paragraph first: later Setext/reference syntax can reinterpret all earlier lines.
    fn compact(text: &'a str) -> Self {
        let mut paragraphs = Vec::new();
        let mut paragraph_start = 0;
        let mut at = 0;
        // ASCII byte checks return at the first prose byte; Unicode trim would be costly and semantically wrong.
        // Delimiter searches run once per complete paragraph, using the standard byte-search implementation.
        for line in text.split_inclusive('\n') {
            let start = at;
            at += line.len();
            if line
                .bytes()
                .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
            {
                paragraphs.push(paragraph_start..start);
                paragraph_start = at;
            }
        }
        paragraphs.push(paragraph_start..text.len());
        let mut retained = 0;
        let mut result = Self {
            text: Cow::Owned(String::new()),
            spans: Vec::new(),
        };
        for paragraph in paragraphs {
            let bytes = text[paragraph.clone()].as_bytes();
            if bytes.contains(&b'[') || bytes.contains(&b'=') || bytes.contains(&b'-') {
                continue;
            }
            // An unindented run with no ASCII punctuation/spaces has no per-line container state.
            // Handle it in one byte pass, avoiding thousands of tiny iterator/indent checks in WASM.
            if let Some(end) = text[paragraph.clone()].find('\n')
                && end + 1 < paragraph.len()
                && bytes.iter().all(|byte| {
                    *byte >= 128 || byte.is_ascii_alphanumeric() || matches!(byte, b'\r' | b'\n')
                })
            {
                result.append(text, retained..paragraph.start + end + 1);
                retained = paragraph.end;
                continue;
            }
            let mut lines = text[paragraph.clone()].split_inclusive('\n').peekable();
            let mut at = paragraph.start;
            while let Some(line) = lines.next() {
                at += line.len();
                let Some(indent) = plain_indent(line) else {
                    continue;
                };
                let first_end = at;
                while lines
                    .peek()
                    .is_some_and(|next| plain_indent(next) == Some(indent))
                {
                    at += lines.next().unwrap().len();
                }
                if at != first_end {
                    result.append(text, retained..first_end);
                    retained = at;
                }
            }
        }
        if result.spans.is_empty() {
            return Self {
                text: Cow::Borrowed(text),
                spans: Vec::new(),
            };
        }
        result.append(text, retained..text.len());
        result
    }

    /// Append an unmodified source interval so mapping never guesses positions inside UTF-8 text.
    fn append(&mut self, text: &str, range: Range<usize>) {
        if !range.is_empty() {
            let start = self.text.len();
            self.text.to_mut().push_str(&text[range.clone()]);
            self.spans.push((start..self.text.len(), range.start));
        }
    }

    /// Starts at a join belong to the next retained span; ends belong to the preceding span.
    pub(super) fn original(&self, range: Range<usize>) -> Range<usize> {
        if self.spans.is_empty() {
            return range;
        }
        let start = self
            .spans
            .partition_point(|(span, _)| span.end <= range.start)
            .min(self.spans.len() - 1);
        let end = self
            .spans
            .partition_point(|(span, _)| span.end < range.end)
            .min(self.spans.len() - 1);
        let (left, original_start) = &self.spans[start];
        let (right, original_end) = &self.spans[end];
        original_start + range.start - left.start..original_end + range.end - right.start
    }
}

/// Plain prose cannot open/close a code, HTML, list, quote or link construct; four-space indentation remains exact.
fn plain_indent(line: &str) -> Option<usize> {
    let line = line.strip_suffix('\n').unwrap_or(line);
    let line = line.strip_suffix('\r').unwrap_or(line);
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    (indent <= 3
        && indent < line.len()
        && line
            .bytes()
            .all(|byte| byte >= 128 || byte.is_ascii_alphanumeric() || byte == b' '))
    .then_some(indent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Event, Tag, TagEnd};

    /// Compare the actual parser's full heading events/ranges; prose simplification must not affect Markdown semantics.
    fn headings(text: &str, mapped: &Mapped<'_>) -> Vec<(String, Range<usize>)> {
        let mut inside = false;
        let mut events = Vec::new();
        for (event, range) in crate::preview::parser(&mapped.text).into_offset_iter() {
            if matches!(event, Event::Start(Tag::Heading { .. })) {
                inside = true;
            }
            if inside {
                events.push((format!("{event:?}"), mapped.original(range)));
            }
            if matches!(event, Event::End(TagEnd::Heading(_))) {
                inside = false;
            }
        }
        assert!(
            events
                .iter()
                .all(|(_, range)| text.get(range.clone()).is_some())
        );
        events
    }

    /// Container continuations, raw blocks, CRLF and multiline references use the same headings as unmodified CommonMark.
    #[test]
    fn inert_prose_compaction_preserves_heading_events_and_utf8_ranges() {
        let prose = "普通正文\n连续正文\n末行正文\n";
        let cases = [
            format!("# 根\n\n{prose}\n## `中文`\n"),
            format!("# 根\r\n\r\n{}\r\n## 中文\r\n", prose.replace('\n', "\r\n")),
            format!("{prose}===\n\n# 尾\n"),
            format!("{prose}**末尾**\n===\n\n# 尾\n"),
            format!("{prose}[末尾](next.md)\n===\n\n# 尾\n"),
            format!("{prose}`末尾`\n===\n\n# 尾\n"),
            format!("{prose}<i>末尾</i>\n===\n\n# 尾\n"),
            format!("{prose}\u{a0}\n**末尾**\n===\n\n# 尾\n"),
            format!("{prose}\u{3000}\n**末尾**\n===\n\n# 尾\n"),
            format!("> 首行\n{prose}> ---\n\n# 尾\n"),
            format!("- 首行\n{prose}\n# 尾\n"),
            format!("```md\n{prose}# 假标题\n```\n\n# 尾\n"),
            format!("<div>\n{prose}# 假标题\n</div>\n\n# 尾\n"),
            format!("[标签\n{prose}]: next.md\n\n# [标签 连续正文 末行正文]\n"),
            format!("[标签]: next.md \"首行\n{prose}尾行\"\n\n# [标签]\n"),
            format!("普通首行\n{prose}\n[标签]: next.md\n\n# 尾\n"),
        ];
        for text in cases {
            let identity = Mapped {
                text: Cow::Borrowed(&text),
                spans: Vec::new(),
            };
            let compact = Mapped::compact(&text);
            assert_eq!(
                headings(&text, &compact),
                headings(&text, &identity),
                "{text}"
            );
        }
    }
}
