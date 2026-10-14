//! Converts CommonMark/GFM and embedded HTML into readonly native blocks and controlled resources.

use plugin_protocol::ui;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, html};
use std::ops::Range;

pub(crate) mod embedded_html;
mod inline;
mod style;

/// Apply the guest's documented content defaults and sibling spacing to one immutable native scene.
pub(super) fn style_document(document: &mut ui::Document) {
    style::defaults(document);
}

pub(super) fn space_blocks(nodes: &mut [ui::Node]) {
    style::blocks(nodes);
}

/// Opt derived native blocks into the public GitHub palette without changing the editor's global theme.
/// A block that already carries a deliberate role (a table header cell) keeps its own preset.
pub(super) fn apply_theme(node: &mut ui::Node) {
    if node.role.is_empty() {
        node.role = "github".into();
    }
    match &mut node.kind {
        ui::Kind::Column { children } | ui::Kind::Row { children } => {
            for child in children {
                apply_theme(child);
            }
        }
        ui::Kind::Scroll { content } => apply_theme(content),
        ui::Kind::Tabs { tabs, .. } => {
            for tab in tabs {
                apply_theme(&mut tab.content);
            }
        }
        _ => {}
    }
}

/// Parser events retain nesting and UTF-8 ranges until the native tree has been derived.
struct Element<'a> {
    tag: Option<Tag<'a>>,
    range: Range<usize>,
    children: Vec<Part<'a>>,
}

enum Part<'a> {
    Element(Element<'a>),
    Event(Event<'a>, Range<usize>),
}

impl Part<'_> {
    /// Every rendered identity uses the parser's byte offset, independently of translated labels.
    fn range(&self) -> Range<usize> {
        match self {
            Self::Element(element) => element.range.clone(),
            Self::Event(_, range) => range.clone(),
        }
    }
}

/// Bound eager parser input as well as its output; native source editing has no preview-size limit.
pub(super) const SOURCE_BUDGET: usize = 256 * 1024;

/// Parse approved extensions with finite intermediate work before native quota validation.
/// Oversized events/depth return a limit result, preserving source editing and the guest instance.
pub(super) fn blocks(source: &str, locale: &str) -> Result<Vec<ui::Node>, ()> {
    // pulldown-cmark scans the full input during construction, before event quotas can run.
    if source.len() > SOURCE_BUDGET {
        return Err(());
    }
    let mut stack = vec![Element {
        tag: None,
        range: 0..source.len(),
        children: Vec::new(),
    }];
    for (count, (event, range)) in parser(source).into_offset_iter().enumerate() {
        // Native scenes allow 2048 nodes, depth 32 and 64 KiB per leaf. Bound the
        // earlier event tree as well so rejected previews cannot exhaust dispatch fuel.
        if count >= 8192
            || stack.len() >= 32
            || matches!(&event,Event::Text(text)|Event::Code(text)|Event::Html(text)|Event::InlineHtml(text) if text.len()>65536)
        {
            return Err(());
        }
        match event {
            Event::Start(tag) => stack.push(Element {
                tag: Some(tag),
                range,
                children: Vec::new(),
            }),
            Event::End(_) => {
                // pulldown-cmark always balances tags, including incomplete Markdown being edited.
                if stack.len() > 1 {
                    let mut element = stack.pop().unwrap();
                    element.range.end = element.range.end.max(range.end);
                    stack
                        .last_mut()
                        .unwrap()
                        .children
                        .push(Part::Element(element));
                }
            }
            event => stack
                .last_mut()
                .unwrap()
                .children
                .push(Part::Event(event, range)),
        }
    }
    Ok(Renderer { locale }.blocks(&stack.pop().unwrap().children))
}

/// Navigation and visible blocks use exactly the same approved Markdown extensions and source events.
pub(super) fn parser(source: &str) -> Parser<'_> {
    Parser::new_ext(
        source,
        Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH,
    )
}

struct Renderer<'a> {
    locale: &'a str,
}

/// Muted quote text is separate from the container's border and insets, avoiding nested decoration.
fn quote_roles(nodes: &mut [ui::Node]) {
    for node in nodes {
        match &mut node.kind {
            ui::Kind::RichText { .. } | ui::Kind::Text { .. } if node.role.is_empty() => {
                node.role = "github-quote-text".into();
            }
            ui::Kind::Column { children } | ui::Kind::Row { children } => quote_roles(children),
            _ => {}
        }
    }
}

impl Renderer<'_> {
    /// Tight lists have bare inline events; buffer them as one paragraph rather than separate text nodes.
    fn blocks(&self, parts: &[Part<'_>]) -> Vec<ui::Node> {
        let mut nodes = Vec::new();
        let mut inline_start = 0;
        for (index, part) in parts.iter().enumerate() {
            if self.is_block(part) {
                self.flush_inline(&parts[inline_start..index], &mut nodes);
                nodes.push(self.block(part));
                inline_start = index + 1;
            }
        }
        self.flush_inline(&parts[inline_start..], &mut nodes);
        nodes
    }

    /// The selected parser flags emit no footnotes, math, metadata or definition-list blocks.
    fn is_block(&self, part: &Part<'_>) -> bool {
        match part {
            Part::Element(element) => matches!(
                element.tag,
                Some(
                    Tag::Paragraph
                        | Tag::Heading { .. }
                        | Tag::BlockQuote(_)
                        | Tag::CodeBlock(_)
                        | Tag::HtmlBlock
                        | Tag::List(_)
                        | Tag::Item
                        | Tag::Table(_)
                )
            ),
            Part::Event(event, _) => matches!(event, Event::Rule | Event::Html(_)),
        }
    }

    /// Native list, quote, code, separator and task nodes keep their semantics outside HTML parsing.
    fn block(&self, part: &Part<'_>) -> ui::Node {
        let range = part.range();
        match part {
            Part::Element(element) => match element.tag.as_ref().unwrap() {
                Tag::CodeBlock(kind) => {
                    let language = match kind {
                        CodeBlockKind::Indented => None,
                        CodeBlockKind::Fenced(info) => info
                            .split_whitespace()
                            .next()
                            .filter(|language| {
                                !language.is_empty()
                                    && language.len() <= 100
                                    && language.bytes().all(|byte| {
                                        byte.is_ascii_alphanumeric() || b"._+-#".contains(&byte)
                                    })
                            })
                            .map(str::to_owned),
                    };
                    ui::Node::code_block(
                        identity(&range, "code"),
                        plain_text(&element.children),
                        language,
                    )
                    .source_range(range)
                }
                Tag::List(start) => self.list(element, *start),
                Tag::BlockQuote(_) => {
                    let mut children = self.blocks(&element.children);
                    quote_roles(&mut children);
                    ui::Node::column(identity(&range, "quote"), children)
                        .role("github-quote")
                        .gap(16.)
                        .source_range(range)
                }
                Tag::HtmlBlock => self.rich(&element.children, range, "html", false),
                Tag::Heading { .. } => {
                    self.content(std::slice::from_ref(part), range, "heading", false)
                }
                Tag::Table(_) => self.table(element),
                Tag::Paragraph => {
                    self.content(std::slice::from_ref(part), range, "paragraph", false)
                }
                _ => ui::Node::column(identity(&range, "block"), self.blocks(&element.children))
                    .gap(6.)
                    .source_range(range),
            },
            Part::Event(Event::Rule, _) => {
                ui::Node::new(identity(&range, "rule"), ui::Kind::Separator).source_range(range)
            }
            Part::Event(Event::Html(_), _) => {
                self.rich(std::slice::from_ref(part), range, "html", false)
            }
            _ => self.content(std::slice::from_ref(part), range, "paragraph", true),
        }
    }

    /// Cells retain independent rich markup and controlled images. Guest-owned style roles provide
    /// the grid, bold white header and alternating body rows without exposing browser/CSS effects.
    /// Per-row cell identities stay unique even when GFM pads several missing cells at the same byte offset.
    fn table(&self, element: &Element<'_>) -> ui::Node {
        let mut rows = Vec::new();
        for (row_index, part) in element.children.iter().enumerate() {
            let Part::Element(row) = part else { continue };
            let header = matches!(row.tag, Some(Tag::TableHead));
            if !header && !matches!(row.tag, Some(Tag::TableRow)) {
                continue;
            }
            let mut cells = Vec::new();
            for (cell_index, part) in row.children.iter().enumerate() {
                let Part::Element(cell) = part else { continue };
                if !matches!(cell.tag, Some(Tag::TableCell)) {
                    continue;
                }
                let wrappers = if header {
                    vec![Tag::Paragraph, Tag::Strong]
                } else {
                    vec![Tag::Paragraph]
                };
                let mut cell = inline::flow(
                    &cell.children,
                    cell.range.clone(),
                    &format!("table-cell-{row_index}-{cell_index}"),
                    &wrappers,
                    self.locale,
                )
                .grow();
                cell.role = if header {
                    "github-table-header"
                } else if row_index % 2 == 1 {
                    "github-table-alt"
                } else {
                    "github-table-cell"
                }
                .into();
                if let Some(Tag::Table(alignments)) = &element.tag {
                    cell.layout.align = match alignments.get(cell_index) {
                        Some(pulldown_cmark::Alignment::Center) => Some(ui::Alignment::Center),
                        Some(pulldown_cmark::Alignment::Right) => Some(ui::Alignment::End),
                        _ => Some(ui::Alignment::Start),
                    };
                }
                cells.push(cell);
            }
            rows.push(
                ui::Node::row(
                    identity(&row.range, &format!("table-row-{row_index}")),
                    cells,
                )
                .gap(0.)
                .source_range(row.range.clone()),
            );
        }
        ui::Node::column(identity(&element.range, "table"), rows)
            .role("github-table")
            .gap(0.)
            .source_range(element.range.clone())
    }

    /// Ordered starts and recursive item bodies cannot depend on an HTML parser's list defaults.
    /// Task state is always parsed from this source version; native toggles request an edit instead of changing it here.
    fn list(&self, element: &Element<'_>, start: Option<u64>) -> ui::Node {
        let mut items = Vec::new();
        for (index, part) in element.children.iter().enumerate() {
            let Part::Element(item) = part else { continue };
            let marker = if let Some((checked, range)) = task(&item.children) {
                ui::Node::checkbox(identity(&range, "task"), "", checked)
                    .tooltip(if self.locale.starts_with("en") {
                        if checked {
                            "Mark task incomplete"
                        } else {
                            "Mark task complete"
                        }
                    } else if checked {
                        "标记任务未完成"
                    } else {
                        "标记任务已完成"
                    })
                    .source_range(range)
            } else {
                let marker = start.map_or_else(
                    || "•".to_owned(),
                    |start| format!("{}.", start.saturating_add(index as u64)),
                );
                let width = (marker.chars().count() as f32 * 8. + 4.).max(20.);
                ui::Node::text(identity(&item.range, "list-marker"), marker).width(width)
            };
            let mut body = self.blocks(&item.children);
            // One block already supplies its own layout and source range. An extra column
            // adds no spacing but spends a native node for every ordinary list item.
            // Multiple blocks still need a column to retain paragraph/nested-list spacing.
            let body = if body.len() == 1 {
                body.pop().unwrap().grow()
            } else {
                ui::Node::column(identity(&item.range, "item-body"), body)
                    .gap(6.)
                    .grow()
            };
            items.push(
                ui::Node::row(identity(&item.range, "item"), vec![marker, body])
                    .gap(6.)
                    .source_range(item.range.clone()),
            );
        }
        ui::Node::column(identity(&element.range, "list"), items)
            .role("github-list")
            .gap(4.)
            .source_range(element.range.clone())
    }

    /// Omitted task markers still contribute their original byte range to the checkbox above.
    fn flush_inline(&self, parts: &[Part<'_>], nodes: &mut Vec<ui::Node>) {
        let Some(first) = parts.first() else { return };
        let range = first.range().start..parts.last().unwrap().range().end;
        let node = self.content(parts, range, "paragraph", true);
        if !matches!(&node.kind, ui::Kind::RichText { html } if html == "<p></p>\n") {
            nodes.push(node);
        }
    }

    /// Only image-bearing content needs native fragment composition; ordinary rich blocks keep their layout.
    fn content(
        &self,
        parts: &[Part<'_>],
        range: Range<usize>,
        kind: &str,
        paragraph: bool,
    ) -> ui::Node {
        if has_html(parts) {
            self.rich(parts, range, kind, paragraph)
        } else if has_image(parts) {
            let wrappers = if paragraph {
                vec![Tag::Paragraph]
            } else {
                Vec::new()
            };
            inline::flow(parts, range, kind, &wrappers, self.locale)
        } else {
            self.rich(parts, range, kind, paragraph)
        }
    }

    /// The public writer escapes Markdown; embedded HTML is then filtered into native-only content.
    fn rich(
        &self,
        parts: &[Part<'_>],
        range: Range<usize>,
        kind: &str,
        paragraph: bool,
    ) -> ui::Node {
        let mut events = Vec::new();
        if paragraph {
            events.push(Event::Start(Tag::Paragraph));
        }
        self.safe_events(parts, &mut events);
        if paragraph {
            events.push(Event::End(Tag::Paragraph.to_end()));
        }
        let mut output = String::new();
        html::push_html(&mut output, events.into_iter());
        if has_html(parts) {
            let id = identity(&range, kind);
            embedded_html::render(&output, range, id, self.locale)
        } else {
            ui::Node::rich_text(identity(&range, kind), output).source_range(range)
        }
    }

    /// Embedded HTML and Markdown images share the filtered native path; task controls stay source-owned.
    fn safe_events<'a>(&self, parts: &[Part<'a>], events: &mut Vec<Event<'a>>) {
        for part in parts {
            match part {
                Part::Element(element) => {
                    let tag = element.tag.as_ref().unwrap();
                    events.push(Event::Start(tag.clone()));
                    self.safe_events(&element.children, events);
                    events.push(Event::End(tag.to_end()));
                }
                Part::Event(Event::TaskListMarker(_), _) => {}
                Part::Event(event, _) => events.push(event.clone()),
            }
        }
    }
}

/// HTML inside image alternative text is literal, just like its Markdown formatting.
fn has_html(parts: &[Part<'_>]) -> bool {
    parts.iter().any(|part| match part {
        Part::Element(element) if matches!(element.tag, Some(Tag::Image { .. })) => false,
        Part::Element(element) => has_html(&element.children),
        Part::Event(Event::Html(_) | Event::InlineHtml(_), _) => true,
        _ => false,
    })
}

/// Images can be nested inside emphasis or links; detection does not flatten their surrounding semantics.
fn has_image(parts: &[Part<'_>]) -> bool {
    parts.iter().any(|part| match part {
        Part::Element(element) => {
            matches!(element.tag, Some(Tag::Image { .. })) || has_image(&element.children)
        }
        Part::Event(_, _) => false,
    })
}

/// Keep native IDs independent of source text, locale and theme, while retaining per-block range identity.
fn identity(range: &Range<usize>, kind: &str) -> String {
    format!("b-{}-{kind}", range.start)
}

/// Preserve literal code, raw HTML and image alternative text without interpreting their markup.
fn plain_text(parts: &[Part<'_>]) -> String {
    let mut text = String::new();
    for part in parts {
        match part {
            Part::Element(element) => text.push_str(&plain_text(&element.children)),
            Part::Event(
                Event::Text(value)
                | Event::Code(value)
                | Event::Html(value)
                | Event::InlineHtml(value),
                _,
            ) => {
                text.push_str(value);
            }
            Part::Event(Event::SoftBreak | Event::HardBreak, _) => text.push('\n'),
            _ => {}
        }
    }
    text
}

/// A nested list owns its own task marker; only this item's first paragraph can label its checkbox.
fn task(parts: &[Part<'_>]) -> Option<(bool, Range<usize>)> {
    for part in parts {
        match part {
            Part::Event(Event::TaskListMarker(checked), range) => {
                return Some((*checked, range.clone()));
            }
            Part::Element(element) if matches!(element.tag, Some(Tag::Paragraph)) => {
                if let Some(marker) = task(&element.children) {
                    return Some(marker);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests;
