//! Actual packaged structure behavior crosses public WIT instead of fabricated SDK ownership.
use super::*;

/// Build an original immutable structure request independently of whatever preview is visible.
fn request(text: &str) -> plugin_protocol::structure::Request {
    plugin_protocol::structure::Request {
        request: 11,
        provider: "structure".into(),
        source: plugin_protocol::language::SourceSnapshot {
            document: document("outline", "大纲.md", 3),
            text: text.into(),
        },
        settings: Default::default(),
    }
}

/// Recursively read public proposal names in source order, retaining intentional duplicate headings.
fn names(nodes: &[plugin_protocol::structure::Node], out: &mut Vec<String>) {
    for node in nodes {
        out.push(node.name.clone());
        names(&node.children, out);
    }
}

/// Inline marks/entities/images, empty and duplicate headings keep readable labels without introducing fake code headings.
#[test]
fn structure_names_keep_commonmark_content_and_ignore_literal_code_and_html() {
    let text = "#\n\n## **中文** &amp; `代码` ![图](x) [链接](next.md)\n\n## 同名\n\n## 同名\n\n> ### 引用标题\n\n- #### 列表标题\n\n```md\n# 假标题\n```\n\n    # 缩进代码\n\n<div>\n# 原始 HTML\n</div>\n\n# 尾部\n";
    let request = request(text);
    let mut guest = Guest::new("");
    let proposal = guest
        .call(api::Input::Event {
            panel: None,
            event: api::Notification::LanguageStructure(request.clone()),
        })
        .unwrap()
        .language_structure
        .unwrap();
    assert!(proposal.valid_for(&request));
    let mut actual = Vec::new();
    names(&proposal.nodes, &mut actual);
    assert_eq!(
        actual,
        [
            "#",
            "中文 & 代码 图 链接",
            "同名",
            "同名",
            "引用标题",
            "列表标题",
            "尾部"
        ]
    );
    assert_eq!(
        proposal.nodes[0].children[2].children[0].children[0].kind,
        "heading-4"
    );
}

/// Empty files produce a complete empty outline; long Unicode titles obey the public name limit at char boundaries.
#[test]
fn structure_empty_files_and_long_unicode_titles_remain_valid() {
    let mut guest = Guest::new("");
    for text in [String::new(), format!("# {}\n", "中".repeat(300))] {
        let request = request(&text);
        let proposal = guest
            .call(api::Input::Event {
                panel: None,
                event: api::Notification::LanguageStructure(request.clone()),
            })
            .unwrap()
            .language_structure
            .unwrap();
        assert!(proposal.valid_for(&request));
        if text.is_empty() {
            assert!(proposal.nodes.is_empty() && proposal.folds.is_empty());
        } else {
            assert_eq!(proposal.nodes[0].name, "中".repeat(170));
        }
    }
}

/// CommonMark accepts LF, CRLF and standalone CR; section starts and folds must use the same physical lines.
#[test]
fn structure_sections_and_folds_accept_all_commonmark_line_endings() {
    let mut guest = Guest::new("");
    for newline in ["\n", "\r\n", "\r"] {
        let source = ["# 第一根", "", "正文", "", "# 第二根", "", "尾部", ""].join(newline);
        let request = request(&source);
        let proposal = guest
            .call(api::Input::Event {
                panel: None,
                event: api::Notification::LanguageStructure(request.clone()),
            })
            .unwrap()
            .language_structure
            .unwrap();
        assert!(proposal.valid_for(&request));
        assert_eq!(proposal.nodes.len(), 2);
        assert_eq!(
            proposal.nodes[0].range.end,
            source.find("# 第二根").unwrap()
        );
        assert_eq!(proposal.folds.len(), 2);
    }
}

/// Production pure workers prepare the same component and reject all preview publications before the callback.
#[test]
fn structure_preparation_returns_no_preview_scene_or_host_allocation() {
    let mut guest = Guest::new("");
    let output = guest
        .call(api::Input::Prepare {
            environment: Default::default(),
            api: api::Negotiated {
                base: "1.0.0".parse().unwrap(),
                capabilities: BTreeMap::new(),
            },
            snapshot: None,
        })
        .unwrap();
    assert!(output.views.is_empty() && output.view_patches.is_empty());
    assert!(guest.store.data().started.is_empty());
}

/// Inline code's parser event includes delimiters and CommonMark padding; navigation targets its visible content.
#[test]
fn structure_inline_code_definitions_skip_delimiters_and_normalized_padding() {
    let text = "# `中文`\n\n# `` `里` ``\n\n换行 `\n代码\n`\n==========\n";
    let request = request(text);
    let mut guest = Guest::new("");
    let proposal = guest
        .call(api::Input::Event {
            panel: None,
            event: api::Notification::LanguageStructure(request.clone()),
        })
        .unwrap()
        .language_structure
        .unwrap();
    assert!(proposal.valid_for(&request));
    assert_eq!(
        &text[proposal.nodes[0].definition.start..proposal.nodes[0].definition.end],
        "中文"
    );
    assert_eq!(
        &text[proposal.nodes[1].definition.start..proposal.nodes[1].definition.end],
        "`里`"
    );
    assert!(proposal.nodes[2].name.contains("代码"));
}

/// Outline has its own 1 MiB source contract and does not inherit the preview's 256 KiB fallback.
#[test]
fn structure_parses_large_source_and_rejects_excess_headings_as_a_whole() {
    let mut guest = Guest::new("");
    let text = format!("# 根\n\n{}", "正文。".repeat(40_000));
    assert!(text.len() > 256 * 1024 && text.len() < 1024 * 1024);
    let request = request(&text);
    let proposal = guest
        .call(api::Input::Event {
            panel: None,
            event: api::Notification::LanguageStructure(request.clone()),
        })
        .unwrap()
        .language_structure
        .unwrap();
    assert!(proposal.valid_for(&request));
    assert_eq!(proposal.nodes[0].range.end, text.len());
    let request = self::request(&"# 标题\n".repeat(4097));
    let error = guest
        .call(api::Input::Event {
            panel: None,
            event: api::Notification::LanguageStructure(request),
        })
        .unwrap_err();
    assert_eq!(error.code, api::ErrorCode::LimitExceeded);
}

/// Dense short lines near the source cap must parse within the actual worker fuel, independently of preview.
#[test]
fn structure_dense_near_cap_source_fits_production_fuel() {
    let mut guest = Guest::new("");
    let mut costs = Vec::new();
    for body in ["正文内容\n", "正文内容 "] {
        let tail = "\n\n## `末尾标题`\n";
        let mut source = "# 预算上限\n\n".to_string();
        while source.len() + body.len() + tail.len() <= 1024 * 1024 {
            source.push_str(body);
        }
        source.push_str(tail);
        let request = request(&source);
        let (output, spent) = guest.metered(
            api::Input::Event {
                panel: None,
                event: api::Notification::LanguageStructure(request.clone()),
            },
            1_000_000_000,
        );
        eprintln!(
            "outline fuel: {} bytes, short_lines={}, {spent} instructions",
            source.len(),
            body.ends_with('\n')
        );
        let proposal = output.unwrap().language_structure.unwrap();
        assert!(proposal.valid_for(&request));
        assert_eq!(proposal.nodes[0].name, "预算上限");
        let child = &proposal.nodes[0].children[0];
        assert_eq!(
            &source[child.definition.start..child.definition.end],
            "末尾标题"
        );
        costs.push(spent);
    }
    assert!(
        costs.iter().all(|spent| *spent <= 100_000_000),
        "outline exceeds the unchanged production fuel"
    );
}

/// A later Setext underline reinterprets all earlier lines, including Unicode whitespace, on the large-file path.
#[test]
fn structure_large_source_keeps_complete_multiline_setext_titles() {
    let mut guest = Guest::new("");
    for middle in [
        "",
        "\u{a0}\n",
        "\u{3000}\n",
        "[链接](next.md)\n",
        "`代码`\n",
        "<i>文字</i>\n",
    ] {
        let source = format!(
            "第一行\n第二行\n{middle}**末尾**\n===\n\n{}\n\n# 尾\n",
            "普通正文\n".repeat(24_000)
        );
        assert!(source.len() > 256 * 1024);
        let request = request(&source);
        let proposal = guest
            .call(api::Input::Event {
                panel: None,
                event: api::Notification::LanguageStructure(request.clone()),
            })
            .unwrap()
            .language_structure
            .unwrap();
        assert!(proposal.valid_for(&request));
        assert!(proposal.nodes[0].name.starts_with("第一行 第二行 "));
        assert!(proposal.nodes[0].name.ends_with("末尾"));
        assert_eq!(proposal.nodes[1].name, "尾");
    }
}

/// The actual public outline callback must supply levels and exact UTF-8 title destinations without preview IO.
#[test]
fn structure_headings_form_a_native_hierarchy_with_precise_chinese_definitions() {
    let mut guest = Guest::new("");
    let source = "# 中文根\n\n正文\n\n### **子标题**\n\n子内容\n\n另一个标题\n==========\n\n```md\n# 假标题\n```\n";
    let request = plugin_protocol::structure::Request {
        request: 7,
        provider: "structure".into(),
        source: plugin_protocol::language::SourceSnapshot {
            document: document("outline", "大纲.md", 9),
            text: source.into(),
        },
        settings: Default::default(),
    };
    let output = guest
        .call(api::Input::Event {
            panel: None,
            event: api::Notification::LanguageStructure(request.clone()),
        })
        .unwrap();
    let proposal = output
        .language_structure
        .expect("Markdown must provide a pure native outline");
    assert!(output.views.is_empty() && output.view_patches.is_empty());
    assert!(proposal.valid_for(&request));
    assert_eq!(proposal.nodes.len(), 2);
    assert_eq!(proposal.nodes[0].name, "中文根");
    assert_eq!(proposal.nodes[0].children[0].name, "子标题");
    assert_eq!(proposal.nodes[1].name, "另一个标题");
    let child = &proposal.nodes[0].children[0];
    assert_eq!(
        &source[child.definition.start..child.definition.end],
        "子标题"
    );
    assert!(proposal.folds.len() >= 2);
    assert!(
        guest.store.data().started.is_empty(),
        "outline parsing must not allocate editor or navigation requests"
    );
}
