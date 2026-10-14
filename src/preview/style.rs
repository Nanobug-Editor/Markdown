//! GitHub Primer's README metrics are guest policy, projected through reusable native UI styles.

use plugin_protocol::{FontStyle, ui};

/// Publish bounded role defaults without changing the user's editor theme or typography settings.
pub(super) fn defaults(document: &mut ui::Document) {
    let body = ui::ContentStyle {
        font: FontStyle {
            family: Some("Segoe UI".into()),
            size_px: Some(16.),
            bold: Some(false),
        },
        line_height: Some(1.5),
        paragraph_gap: Some(1.),
        headings: [2., 1.5, 1.25, 1., 0.875, 0.85]
            .into_iter()
            .enumerate()
            .map(|(index, scale)| ui::HeadingStyle {
                scale,
                weight: 600,
                line_height: 1.25,
                padding_bottom: if index < 2 { 0.3 } else { 0. },
                border_bottom: if index < 2 { 1. } else { 0. },
                muted: index == 5,
            })
            .collect(),
        code: Some(ui::CodeStyle {
            scale: 0.85,
            line_height: 1.45,
            padding: 16.,
            radius: 6.,
            border: 0.,
        }),
        table_cell_padding: Some(ui::Edges::symmetric(6., 13.)),
        ..Default::default()
    };
    for role in [
        "github",
        "github-muted",
        "github-quote",
        "github-quote-text",
        "github-list",
        "github-rule",
        "github-table",
        "github-table-cell",
        "github-table-header",
        "github-table-alt",
    ] {
        let mut style = body.clone();
        match role {
            "github-quote" => {
                style.padding = Some(ui::Edges::symmetric(0., 16.));
                style.border = Some(ui::Edges {
                    left: 4.,
                    ..Default::default()
                });
            }
            "github-list" => {
                style.padding = Some(ui::Edges {
                    left: 16.,
                    ..Default::default()
                })
            }
            "github-table" => {
                style.border = Some(ui::Edges {
                    top: 1.,
                    left: 1.,
                    ..Default::default()
                })
            }
            "github-table-cell" | "github-table-header" | "github-table-alt" => {
                style.padding = Some(ui::Edges::symmetric(6., 13.));
                style.border = Some(ui::Edges {
                    right: 1.,
                    bottom: 1.,
                    ..Default::default()
                });
                style.paragraph_gap = Some(0.);
            }
            _ => {}
        }
        document.content_styles.insert(role.into(), style);
    }
}

/// Sibling spacing follows README blocks instead of one gap applied to every nested structure.
pub(super) fn blocks(nodes: &mut [ui::Node]) {
    let last = nodes.len().saturating_sub(1);
    let mut previous_bottom: f32 = 0.;
    for (index, node) in nodes.iter_mut().enumerate() {
        node.layout.margin.bottom = if index == last { 0. } else { 16. };
        let heading = node.id.contains("-heading")
            || matches!(&node.kind,
            ui::Kind::RichText { html } if (1..=6).any(|level| html.trim_start().starts_with(&format!("<h{level}>"))));
        let rule = node.id.ends_with("-rule");
        let requested_top: f32 = if index > 0 && (heading || rule) {
            24.
        } else {
            0.
        };
        // CSS margins collapse between siblings; flex margins add, so publish only the remainder.
        node.layout.margin.top = (requested_top - previous_bottom).max(0.);
        if rule {
            node.layout.margin.bottom = if index == last { 0. } else { 24. };
        }
        previous_bottom = node.layout.margin.bottom;
    }
}
