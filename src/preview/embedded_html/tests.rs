//! Observe filtered preview output, source mapping and navigation at the ordinary parser boundary.

use crate::{navigation::Index, preview::blocks};
use plugin_protocol::ui::{self, Kind, Node};

/// Mappings and activation metadata require the same declarations as a published guest view.
fn document(node: Node) -> ui::Document {
    let mut document = ui::Document::new(node);
    document.source = Some(plugin_protocol::api::DocumentVersion {
        id: "source".into(),
        path: "notes.md".into(),
        revision: 1,
    });
    document.link_events = true;
    document
}

fn leaves(source: &str) -> Vec<Node> {
    let mut leaves = Vec::new();
    for root in blocks(source, "zh-CN").unwrap() {
        root.visit(&mut |node| {
            if !matches!(node.kind, Kind::Column { .. } | Kind::Row { .. }) {
                leaves.push(node.clone());
            }
        });
    }
    leaves
}

fn rich(nodes: &[Node]) -> String {
    nodes
        .iter()
        .filter_map(|node| match &node.kind {
            Kind::RichText { html } => Some(html.as_str()),
            _ => None,
        })
        .collect()
}

/// Author pixel dimensions must survive HTML tokenization into the public image node's geometry.
#[test]
fn html_image_dimensions_reach_native_layout() {
    let nodes = leaves("<img src='logo.png' width='128' height='128'>");
    let image = nodes
        .iter()
        .find(|node| matches!(node.kind, Kind::Image { .. }))
        .unwrap();
    assert_eq!(image.layout.width, Some(128.));
    assert_eq!(image.layout.height, Some(128.));
}

/// Scope changes preserve native alignment, image ownership and sibling text without granting CSS.
#[test]
fn html_paragraph_alignment_is_scoped_and_keeps_multiple_image_ids_unique() {
    let source = "<div><p align='center'><img src='a.png' width='128'><img src='b.png' width='128'><img src='c.png' width='128'></p><p align='right'>右</p><p>左</p></div>";
    let nodes = blocks(source, "zh-CN").unwrap();
    let doc = document(Node::column("root", nodes));
    assert!(doc.validate().is_ok());
    let mut centered_images = 0;
    let mut right = false;
    let mut plain = false;
    doc.root.visit(&mut |node| {
        if node.layout.align == Some(ui::Alignment::Center) {
            node.visit(&mut |child| {
                centered_images += usize::from(matches!(child.kind, Kind::Image { .. }))
            });
        }
        if let Kind::RichText { html } = &node.kind {
            if html.contains('右') {
                right |= node.layout.align == Some(ui::Alignment::End);
            }
            if html.contains('左') {
                plain |= node.layout.align.is_none();
            }
            assert!(!html.contains("align=") && !html.contains("style="));
        }
    });
    assert_eq!(centered_images, 3);
    assert!(right && plain);
}

/// Invalid sizes are ignored per attribute, preserving valid peers and the ordinary intrinsic fallback.
#[test]
fn html_image_dimensions_are_optional_and_bounded_without_invalidating_the_scene() {
    for (attributes, width, height) in [
        ("WIDTH=' 128 ' HEIGHT='064'", Some(128.), Some(64.)),
        ("width='0' height='10000'", Some(0.), Some(10000.)),
        ("width='240'", Some(240.), None),
        ("height='96'", None, Some(96.)),
        ("width='50%' height='-1'", None, None),
        ("width='NaN' height='10001'", None, None),
        ("width='128px' height='1.5'", None, None),
        (
            "width='128' height='999999999999999999999'",
            Some(128.),
            None,
        ),
    ] {
        let nodes = leaves(&format!("<img src='logo.png' {attributes}>"));
        let image = nodes
            .iter()
            .find(|node| matches!(node.kind, Kind::Image { .. }))
            .unwrap();
        assert_eq!(
            (image.layout.width, image.layout.height),
            (width, height),
            "{attributes}"
        );
        assert!(document(Node::column("root", nodes)).validate().is_ok());
    }
}

/// Inline and block tags render as native markup, preserving UTF-8 mapping and Markdown escaping.
#[test]
fn html_blocks_inline_styles_and_tables_preserve_native_markup() {
    let source = "<h2 onclick='bad()'>中文标题</h2>\n\n前 <u>下划线</u> <mark>高亮</mark><br>后 **粗体** &lt;字&gt;\n\n<table><tr><th>名</th><td><code>值</code></td></tr></table>\n";
    let nodes = leaves(source);
    let markup = rich(&nodes);
    for expected in [
        "<h2>中文标题</h2>",
        "<u>下划线</u>",
        "<mark>高亮</mark>",
        "<br>",
        "<strong>粗体</strong>",
        "&lt;字&gt;",
        "<table>",
        "<code>值</code>",
    ] {
        assert!(markup.contains(expected), "missing {expected}: {markup}");
    }
    assert!(!markup.contains("onclick"));
    for node in nodes {
        let range = node.source_range.unwrap();
        assert!(range.end <= source.len());
        assert!(source.is_char_boundary(range.start) && source.is_char_boundary(range.end));
    }
}

/// HTML5 attributes decode once; images never remain in rich markup or inherit arbitrary event/CSS attributes.
#[test]
fn quoted_attributes_entities_and_mixed_images_use_controlled_nodes() {
    let source = "前 <strong>粗<img SRC='图&gt; &amp; x.png' ALT='中文 &quot;图&quot;' onerror='bad()'>后 ![真](real.png)</strong> 末";
    let nodes = leaves(source);
    let images: Vec<_> = nodes
        .iter()
        .filter(|node| matches!(node.kind, Kind::Image { .. }))
        .collect();
    assert_eq!(images.len(), 2);
    assert!(matches!(&images[0].kind, Kind::Image { source, alt }
        if source == "图> & x.png" && alt == "中文 \"图\""));
    assert!(matches!(&images[1].kind, Kind::Image { source, .. } if source == "real.png"));
    let markup = rich(&nodes);
    assert!(markup.contains("<strong>粗</strong>"));
    assert!(markup.contains("<strong>后 "));
    assert!(!markup.contains("<img") && !markup.contains("onerror"));
}

/// Executable/embedded content, comments and code cannot acquire image or navigation resources.
#[test]
fn forbidden_subtrees_and_literal_code_do_not_declare_effects() {
    let source = "<div>前<script>\"<img src='script.png'>\"</script><style>x{content:'<a href=x.md>'}</style><iframe src='https://example.com'><img src='frame.png'></iframe><svg><a href='svg.md'>隐藏</a></svg><!-- <img src='comment.png'> --><svg/>后</div>\n\n`<img src='code.png'>`\n\n```html\n<a href='code.md'>代码</a>\n```\n";
    let nodes = leaves(source);
    assert!(
        nodes
            .iter()
            .all(|node| !matches!(node.kind, Kind::Image { .. }) && node.links.is_empty())
    );
    let markup = rich(&nodes);
    assert!(markup.contains("前后"));
    assert!(markup.contains("&lt;img src='code.png'&gt;"));
    // Unsupported form/raw-text bodies cannot turn their apparent markup into resource declarations.
    for tag in ["form", "button", "select", "textarea"] {
        let nodes = leaves(&format!(
            "<div>前<{tag}><img src='form.png'><a href='form.md'>隐藏</a></{tag}>后</div>\n"
        ));
        assert!(
            nodes
                .iter()
                .all(|node| !matches!(node.kind, Kind::Image { .. }) && node.links.is_empty())
        );
        assert!(rich(&nodes).contains("前后"));
    }
    for removed in [
        "script.png",
        "frame.png",
        "svg.md",
        "comment.png",
        "<script",
        "<style",
        "<iframe",
    ] {
        assert!(!markup.contains(removed), "retained {removed}");
    }
}

/// Coarse HTML block ranges do not grant a neighbouring fragment another fragment's link.
#[test]
fn html_links_bind_to_the_exact_fragment_and_image_after_incremental_edits() {
    let source = "<div><a href='next.md?a=1&amp;b=2'>中文<img src='missing.png' alt='图'>后</a><img src='plain.png'>尾</div>\n";
    let index = Index::parse(source);
    let mut nodes = blocks(source, "zh-CN").unwrap();
    index.annotate(&mut nodes);
    let all = leaves(source);
    for node in &all {
        let linked = node.links.iter().any(|link| link.uri == "next.md?a=1&b=2");
        assert_eq!(
            index.resolve(&nodes, &node.id, "next.md?a=1&b=2").is_some(),
            linked
        );
    }
    assert_eq!(all.iter().filter(|node| !node.links.is_empty()).count(), 3);
    let mut parsed = crate::incremental::Parsed::default();
    let first = "前 <a href='next.md'>中文</a> 后\n";
    parsed.update(first, "zh-CN");
    parsed.update(&first.replace("中文", "新增中文"), "zh-CN");
    let index = Index::parse(&first.replace("中文", "新增中文"));
    index.annotate(&mut parsed.nodes);
    let node = &parsed.nodes[0];
    assert_eq!(node.links[0].label, "新增中文");
    assert_eq!(
        index.resolve(&parsed.nodes, &node.id, "next.md"),
        Some("next.md")
    );
    assert!(
        document(Node::column("root", parsed.nodes))
            .validate()
            .is_ok()
    );
}

/// Incomplete author tags remain balanced, invalid URIs cannot introduce attributes, and nesting is bounded.
#[test]
fn malformed_tags_and_oversized_nesting_leave_a_valid_readonly_scene() {
    let nodes = leaves("前 <strong>未闭合 <a href='javascript:bad()'>链接</a> 后\n");
    assert!(rich(&nodes).contains("</strong>"));
    assert!(nodes.iter().all(|node| node.links.is_empty()));
    for locale in ["zh-CN", "en-US"] {
        let node = super::render(
            &format!("{}字", "<strong>".repeat(40)),
            0..1,
            "limit".into(),
            locale,
        );
        assert!(matches!(node.kind, Kind::Text { .. }));
        assert!(document(node).validate().is_ok());
    }
}
