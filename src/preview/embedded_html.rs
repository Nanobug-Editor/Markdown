//! Filter HTML5 tokens into native rich fragments, images and explicit link targets.
//! No DOM, browser, CSS engine or external effect is created while author markup is parsed.

use html5ever::tokenizer::{
    BufferQueue, Tag, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, states::RawKind,
};
use plugin_protocol::ui;
use std::ops::Range;

/// Render one immutable source block. HTML image fragments share that block's UTF-8 range;
/// the host remains responsible for resource resolution, permissions, decoding and cancellation.
pub(super) fn render(markup: &str, range: Range<usize>, id: String, locale: &str) -> ui::Node {
    let id = format!("{id}-embedded-html");
    let mut input = BufferQueue::default();
    input.push_back(markup.into());
    let mut tokenizer = Tokenizer::new(
        NativeHtml {
            id: id.clone(),
            range: range.clone(),
            locale,
            nodes: Vec::new(),
            output: String::new(),
            active: Vec::new(),
            blocked: Vec::new(),
            links: Vec::new(),
            visible: false,
            count: 0,
            limited: false,
        },
        Default::default(),
    );
    let _ = tokenizer.feed(&mut input);
    tokenizer.end();
    let mut sink = tokenizer.sink;
    if sink.limited {
        return ui::Node::text(
            id,
            if locale.starts_with("en") {
                "This HTML block exceeds the native preview limits."
            } else {
                "此 HTML 内容块超出原生预览限制。"
            },
        )
        .source_range(range);
    }
    sink.flush();
    if sink.nodes.len() == 1 {
        let mut node = sink.nodes.pop().unwrap();
        node.id = id;
        node
    } else {
        ui::Node::column(id, sink.nodes).gap(4.).source_range(range)
    }
}

/// Incremental renaming preserves this suffix; only this parser's leaves carry author HTML targets.
pub(crate) fn owns(node: &ui::Node) -> bool {
    node.id.contains("-embedded-html")
}

struct OpenTag {
    name: String,
    markup: String,
    link: Option<String>,
    /// Author alignment is projected onto native layout, never retained as executable CSS.
    align: Option<ui::Alignment>,
    scope: usize,
}

/// Tokenization is bounded before native scene validation; rejected markup never grants effects.
struct NativeHtml<'a> {
    id: String,
    range: Range<usize>,
    locale: &'a str,
    nodes: Vec<ui::Node>,
    output: String,
    active: Vec<OpenTag>,
    blocked: Vec<String>,
    links: Vec<ui::LinkTarget>,
    visible: bool,
    count: usize,
    limited: bool,
}

impl TokenSink for NativeHtml<'_> {
    type Handle = ();

    fn process_token(&mut self, token: Token, _: u64) -> TokenSinkResult<()> {
        self.count += 1;
        if self.count > 8192 || self.active.len() + self.blocked.len() >= 32 {
            self.limited = true;
        }
        if self.limited {
            return TokenSinkResult::Continue;
        }
        match token {
            Token::TagToken(tag) => self.tag(tag),
            Token::CharacterTokens(text) if self.blocked.is_empty() => {
                self.text(&text);
                TokenSinkResult::Continue
            }
            // Comments, doctypes and parse diagnostics never become visible markup or resources.
            _ => TokenSinkResult::Continue,
        }
    }
}

impl NativeHtml<'_> {
    /// Closing tags only close existing author containers; attributes cannot escape their quoted value.
    fn tag(&mut self, tag: Tag) -> TokenSinkResult<()> {
        let name = tag.name.to_string();
        if tag.kind == TagKind::EndTag {
            if let Some(index) = self.blocked.iter().rposition(|open| open == &name) {
                self.blocked.truncate(index);
            } else if self.blocked.is_empty() {
                if let Some(index) = self.active.iter().rposition(|open| open.name == name) {
                    if self.active[index..].iter().any(|open| open.align.is_some()) {
                        self.flush();
                    }
                    while self.active.len() > index {
                        let open = self.active.pop().unwrap();
                        self.output.push_str(&format!("</{}>", open.name));
                    }
                }
            }
            return TokenSinkResult::Continue;
        }
        if forbidden(&name) {
            // HTML raw-text rules matter: apparent images/links inside script or CSS are data.
            if !tag.self_closing || !matches!(name.as_str(), "svg" | "math") {
                self.blocked.push(name.clone());
            }
            return match name.as_str() {
                "script" => TokenSinkResult::RawData(RawKind::ScriptData),
                "style" | "iframe" | "xmp" | "noembed" | "noframes" => {
                    TokenSinkResult::RawData(RawKind::Rawtext)
                }
                _ => TokenSinkResult::Continue,
            };
        }
        if !self.blocked.is_empty() {
            return TokenSinkResult::Continue;
        }
        if name == "img" {
            self.image(&tag);
        } else if allowed(&name) {
            let align = if matches!(
                name.as_str(),
                "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
            ) {
                alignment(&attribute(&tag, "align"))
            } else {
                None
            };
            if align.is_some() {
                self.flush();
            }
            let link = (name == "a")
                .then(|| attribute(&tag, "href"))
                .filter(|uri| valid_uri(uri));
            let mut markup = format!("<{name}");
            if let Some(uri) = &link {
                markup.push_str(&format!(" href=\"{}\"", escape(uri)));
            }
            markup.push('>');
            self.output.push_str(&markup);
            if matches!(name.as_str(), "br" | "hr") {
                self.visible = true;
            } else {
                // HTML self-closing syntax does not close ordinary HTML elements.
                self.active.push(OpenTag {
                    name,
                    markup,
                    link,
                    align,
                    scope: self.count,
                });
            }
        }
        TokenSinkResult::Continue
    }

    /// Entity-decoded author text is escaped once for Base; callbacks keep the same decoded URI.
    fn text(&mut self, text: &str) {
        self.output.push_str(&escape(text));
        self.visible |= !text.trim().is_empty();
        if let Some(uri) = self.active.iter().rev().find_map(|tag| tag.link.as_ref()) {
            let index = self.links.iter().position(|link| &link.uri == uri);
            let link = if let Some(index) = index {
                &mut self.links[index]
            } else {
                self.links.push(ui::LinkTarget {
                    uri: uri.clone(),
                    label: String::new(),
                });
                self.links.last_mut().unwrap()
            };
            for ch in text.chars() {
                if link.label.len() + ch.len_utf8() > 256 {
                    break;
                }
                link.label.push(ch);
            }
        }
    }

    /// Every HTML image leaves rich markup and enters the ordinary negotiated image capability.
    fn image(&mut self, tag: &Tag) {
        self.flush();
        let source = attribute(tag, "src");
        let alt = attribute(tag, "alt");
        // Token ordinals stay unique when multiple images share one aligned row.
        let id = format!("{}-image-{}", self.id, self.count);
        let mut node = if valid_uri(&source) {
            ui::Node::image(id, source, alt.clone())
        } else {
            ui::Node::text(
                id,
                format!(
                    "{}: {}",
                    alt,
                    if self.locale.starts_with("en") {
                        "Invalid image reference."
                    } else {
                        "图片引用地址无效。"
                    }
                ),
            )
        }
        .source_range(self.range.clone());
        // Dimensions describe the native pixel area, not a browser/CSS layout. Ignore unsupported
        // or out-of-budget attribute values locally so one malformed size cannot retire the preview.
        if matches!(node.kind, ui::Kind::Image { .. }) {
            node.layout.width = pixel_dimension(tag, "width");
            node.layout.height = pixel_dimension(tag, "height");
        }
        if let Some(uri) = self.active.iter().rev().find_map(|tag| tag.link.as_ref()) {
            let mut label = alt;
            while label.len() > 256 {
                label.pop();
            }
            node.links.push(ui::LinkTarget {
                uri: uri.clone(),
                label,
            });
        }
        if let Some(open) = self.active.iter().rev().find(|open| open.align.is_some()) {
            let group_id = format!("{}-alignment-{}", self.id, open.scope);
            if let Some(last) = self.nodes.last_mut()
                && last.id == group_id
                && let ui::Kind::Row { children } = &mut last.kind
            {
                children.push(node);
            } else {
                let mut row = ui::Node::row(group_id, vec![node])
                    .wrap()
                    .gap(4.)
                    .source_range(self.range.clone());
                row.layout.align = open.align;
                self.nodes.push(row);
            }
        } else {
            self.nodes.push(node);
        }
    }

    /// Split only at controlled images, balancing and reopening all supported formatting containers.
    fn flush(&mut self) {
        if self.visible {
            for tag in self.active.iter().rev() {
                self.output.push_str(&format!("</{}>", tag.name));
            }
            let mut node = ui::Node::rich_text(
                format!("{}-part-{}", self.id, self.nodes.len()),
                std::mem::take(&mut self.output),
            )
            .source_range(self.range.clone());
            node.links = std::mem::take(&mut self.links);
            node.layout.align = self.active.iter().rev().find_map(|open| open.align);
            self.nodes.push(node);
        }
        self.output.clear();
        self.links.clear();
        self.visible = false;
        for tag in &self.active {
            self.output.push_str(&tag.markup);
        }
    }
}

/// GitHub-compatible paragraph alignment accepts the legacy HTML values without granting styles.
fn alignment(value: &str) -> Option<ui::Alignment> {
    match value.trim().to_ascii_lowercase().as_str() {
        "left" => Some(ui::Alignment::Start),
        "center" => Some(ui::Alignment::Center),
        "right" => Some(ui::Alignment::End),
        _ => None,
    }
}

/// Preserve the native renderer's text semantics while discarding styles, event handlers and embeds.
fn allowed(name: &str) -> bool {
    matches!(
        name,
        "p" | "div"
            | "span"
            | "section"
            | "article"
            | "details"
            | "summary"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "blockquote"
            | "pre"
            | "strong"
            | "b"
            | "em"
            | "i"
            | "del"
            | "s"
            | "u"
            | "code"
            | "mark"
            | "a"
            | "br"
            | "hr"
            | "table"
            | "thead"
            | "tbody"
            | "tfoot"
            | "tr"
            | "th"
            | "td"
            | "ul"
            | "ol"
            | "li"
    )
}

/// Unsupported executable/embedded subtrees disappear together with their contents and resources.
fn forbidden(name: &str) -> bool {
    matches!(
        name,
        "script"
            | "style"
            | "iframe"
            | "object"
            | "svg"
            | "math"
            | "template"
            | "head"
            | "title"
            | "xmp"
            | "noembed"
            | "noframes"
            | "form"
            | "button"
            | "select"
            | "textarea"
    )
}

fn attribute(tag: &Tag, name: &str) -> String {
    tag.attrs
        .iter()
        .find(|attr| attr.name.ns.is_empty() && attr.name.local.as_ref() == name)
        .map(|attr| attr.value.to_string())
        .unwrap_or_default()
}

/// HTML width/height attributes accept bounded nonnegative integer pixels; CSS and percentages stay unsupported.
fn pixel_dimension(tag: &Tag, name: &str) -> Option<f32> {
    let value = attribute(tag, name);
    let value = value.trim();
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value
        .parse::<u32>()
        .ok()
        .filter(|value| *value <= 10000)
        .map(|value| value as f32)
}

/// References stay declarations. The existing host/navigation boundary enforces paths and schemes.
fn valid_uri(uri: &str) -> bool {
    !uri.is_empty()
        && uri.len() <= 4096
        && !uri.chars().any(char::is_control)
        && !uri.split_once(':').is_some_and(|(scheme, _)| {
            matches!(
                scheme.to_ascii_lowercase().as_str(),
                "javascript" | "vbscript" | "data"
            )
        })
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests;
