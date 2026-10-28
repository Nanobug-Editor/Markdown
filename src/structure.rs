//! Pure Markdown outline/folds use immutable host snapshots; preview size, selection and live requests stay independent.
use plugin_protocol::{api, structure};
use pulldown_cmark::{Event, Tag, TagEnd};
use std::ops::Range;

mod source;

/// One parser heading retains display text and exact source destinations until its section boundary is known.
struct Heading {
    /// Markdown heading level, including Setext H1/H2; skipped levels do not create artificial ancestors.
    level: u8,
    /// Decoded visible inline content, excluding Markdown delimiters and HTML tags.
    name: String,
    /// Full heading syntax supplied by CommonMark events, including a Setext underline.
    syntax: Range<usize>,
    /// First through last visible source event; empty headings fall back to their marker.
    definition: Option<api::TextRange>,
}

impl Heading {
    /// Append visible `value` while extending its exact raw UTF-8 content `range` through the last inline event.
    fn content(&mut self, value: &str, range: Range<usize>) {
        self.name.push_str(value);
        if range.start < range.end {
            if let Some(definition) = &mut self.definition {
                definition.end = range.end;
            } else {
                self.definition = Some(api::TextRange {
                    start: range.start,
                    end: range.end,
                });
            }
        }
    }
}

/// Describe only the registered provider and original document/revision; invalid or over-budget replies fail as a whole.
pub(super) fn describe(request: &structure::Request) -> Result<structure::Proposal, api::Failure> {
    if !request.valid() || request.provider != "structure" {
        return Err(api::Failure::new(
            api::ErrorCode::InvalidRequest,
            "Invalid Markdown structure request",
        ));
    }
    let text = &request.source.text;
    let headings = headings(text)?;
    let mut nodes = Vec::new();
    let mut folds = Vec::new();
    let mut open: Vec<(u8, usize, structure::Node)> = Vec::new();
    for heading in headings {
        let start = text[..heading.syntax.start]
            .rfind(['\r', '\n'])
            .map_or(0, |offset| offset + 1);
        while open
            .last()
            .is_some_and(|(level, _, _)| *level >= heading.level)
        {
            finish(&mut open, &mut nodes, &mut folds, text, start);
        }
        let name = label(&heading.name, heading.level);
        let definition = heading.definition.unwrap_or(api::TextRange {
            start: heading.syntax.start,
            end: heading.syntax.start + 1,
        });
        let fold_start = text[..heading.syntax.end]
            .trim_end_matches(['\r', '\n'])
            .len();
        open.push((
            heading.level,
            fold_start,
            structure::Node {
                name,
                kind: format!("heading-{}", heading.level),
                icon: None,
                range: api::TextRange {
                    start,
                    end: text.len(),
                },
                definition,
                children: Vec::new(),
            },
        ));
    }
    while !open.is_empty() {
        finish(&mut open, &mut nodes, &mut folds, text, text.len());
    }
    let proposal = structure::Proposal {
        request: request.request,
        document: request.source.document.clone(),
        nodes,
        folds,
    };
    if !proposal.valid_for(request) {
        return Err(limit());
    }
    Ok(proposal)
}

/// CommonMark decides ATX/Setext, quote/list nesting and code/HTML exclusions; no line-pattern parser invents headings.
fn headings(text: &str) -> Result<Vec<Heading>, api::Failure> {
    let mut headings = Vec::new();
    let mut current: Option<Heading> = None;
    let mapped = source::Mapped::new(text);
    for (event, range) in crate::preview::parser(&mapped.text).into_offset_iter() {
        let range = mapped.original(range);
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some(Heading {
                    level: level as u8,
                    name: String::new(),
                    syntax: range,
                    definition: None,
                })
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(mut heading) = current.take() {
                    heading.syntax.end = heading.syntax.end.max(range.end);
                    headings.push(heading);
                    if headings.len() > 4096 {
                        return Err(limit());
                    }
                }
            }
            Event::Text(value) => {
                if let Some(heading) = &mut current {
                    heading.content(&value, range);
                }
            }
            Event::Code(value) => {
                if let Some(heading) = &mut current {
                    heading.content(&value, code_content_range(text, range));
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(heading) = &mut current {
                    heading.name.push(' ');
                }
            }
            _ => {}
        }
    }
    Ok(headings)
}

/// Code event ranges include the backtick run and one normalized padding space; definitions select visible content.
/// `text` is the original source and `range` its parser-proven code span; only ASCII delimiters/line breaks are removed.
fn code_content_range(text: &str, range: Range<usize>) -> Range<usize> {
    let raw = &text[range.clone()];
    let ticks = raw.bytes().take_while(|byte| *byte == b'`').count();
    let mut content = range.start + ticks..range.end - ticks;
    let value = &text[content.clone()];
    let left = if value.starts_with("\r\n") {
        2
    } else {
        usize::from(value.starts_with([' ', '\r', '\n']))
    };
    let right = if value.ends_with("\r\n") {
        2
    } else {
        usize::from(value.ends_with([' ', '\r', '\n']))
    };
    if left > 0
        && right > 0
        && value
            .bytes()
            .any(|byte| !matches!(byte, b' ' | b'\r' | b'\n'))
    {
        content.start += left;
        content.end -= right;
    }
    content
}

/// Close a section before its next peer/ancestor, attaching it only to an actual preceding shallower heading.
fn finish(
    open: &mut Vec<(u8, usize, structure::Node)>,
    roots: &mut Vec<structure::Node>,
    folds: &mut Vec<api::TextRange>,
    text: &str,
    end: usize,
) {
    let (_, fold_start, mut node) = open.pop().expect("an open section");
    node.range.end = end;
    let fold_end = text[..end].trim_end_matches(['\r', '\n']).len();
    if fold_start < fold_end && text[fold_start..fold_end].contains(['\r', '\n']) {
        folds.push(api::TextRange {
            start: fold_start,
            end: fold_end,
        });
    }
    if let Some((_, _, parent)) = open.last_mut() {
        parent.children.push(node);
    } else {
        roots.push(node);
    }
}

/// Native names collapse multiline/control whitespace and stop on a UTF-8 boundary within the public 512-byte limit.
fn label(text: &str, level: u8) -> String {
    let mut result = String::new();
    let mut space = false;
    for ch in text.chars() {
        if ch.is_whitespace() || ch.is_control() {
            space = !result.is_empty();
            continue;
        }
        let needed = ch.len_utf8() + usize::from(space);
        if result.len() + needed > 512 {
            break;
        }
        if space {
            result.push(' ');
        }
        result.push(ch);
        space = false;
    }
    if result.is_empty() {
        "#".repeat(level as usize)
    } else {
        result
    }
}

/// Limits are reported instead of returning a tree with missing headings or invalid native ranges.
fn limit() -> api::Failure {
    api::Failure::new(
        api::ErrorCode::LimitExceeded,
        "Markdown structure exceeds the public outline limits",
    )
}
