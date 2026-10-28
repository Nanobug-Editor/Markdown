//! The same wheel trace measures real request coalescing, target geometry and deadlines before/after A2/A3 evaluation.
use super::*;

/// Native source positions are fed exactly as the host emits them; no private scrolling queue is inspected.
fn position(guest: &mut Guest, offset: usize) {
    guest
        .input(api::Input::Event {
            panel: Some("preview".into()),
            event: api::Notification::SourceViewport(api::SourceViewport {
                document: document("notes", "notes.md", 1),
                ui_revision: guest.view.revision,
                offset,
                line_fraction: 0.,
                origin: None,
                layout: false,
            }),
        })
        .unwrap();
}

/// Twenty manual positions coalesce into one in-flight locate plus its final deferred target, without per-frame cancellation.
#[test]
fn scrolling_trace_keeps_two_requests_and_the_last_semantic_block() {
    let source = (0..24)
        .map(|n| format!("段落{n}：中文正文。\n\n"))
        .collect::<String>();
    let last = source.find("段落20").unwrap();
    let mut guest = Guest::new(&source);
    position(&mut guest, 0);
    for n in 1..=20 {
        position(&mut guest, source.find(&format!("段落{n}：")).unwrap());
    }
    assert_eq!(guest.store.data().started.len(), 1);
    let first = guest.store.data().started[0].0.clone();
    guest.receipt(first, Ok(api::EditorValue::Unit));
    assert_eq!(guest.store.data().started.len(), 2);
    let (handle, operation, deadline) = guest.store.data().started[1].clone();
    assert_eq!(deadline, 2_000);
    assert!(
        matches!(operation, api::EditorOperation::LocateViewport { origin: 2, target: api::ViewportTarget::Preview { node, fraction }, .. } if node == format!("b-{last}-paragraph") && fraction == 0.)
    );
    position(&mut guest, last);
    guest.receipt(handle, Ok(api::EditorValue::Unit));
    assert_eq!(guest.store.data().started.len(), 2);
    assert!(guest.store.data().owned.is_empty());
    eprintln!(
        "scroll trace: 21 source positions -> 2 requests, final block b-{last}-paragraph, origin 2, deadline 2000ms"
    );
}

/// A genuine preview translation carries the visible paragraph's original source geometry.
fn preview_position(guest: &mut Guest, source: &str, paragraph: &str) {
    let start = source.find(paragraph).unwrap();
    let end = start + source[start..].find('\n').unwrap();
    guest
        .input(api::Input::Event {
            panel: Some("preview".into()),
            event: api::Notification::Ui(ui::UiEvent {
                revision: guest.view.revision,
                node: "preview-scroll".into(),
                action: ui::Action::Viewport(api::PreviewViewport {
                    block: format!("b-{start}-paragraph"),
                    source_range: ui::SourceRange { start, end },
                    fraction: 0.,
                    origin: None,
                    layout: false,
                }),
            }),
        })
        .unwrap();
}

/// Reversed wheels, stale completions and explicit heading navigation retain the same drivers and accepted target sequence.
#[test]
fn scrolling_trace_keeps_bidirectional_takeover_and_rejects_stale_receipts() {
    let source = "[跳转](#末尾)\n\n第一段\n\n第二段\n\n# 末尾\n\n末段\n";
    let mut guest = Guest::new(source);
    position(&mut guest, source.find("第一段").unwrap());
    let abandoned_source = guest.store.data().started[0].0.clone();
    preview_position(&mut guest, source, "第二段");
    assert_eq!(guest.store.data().started.len(), 2);
    let abandoned_preview = guest.store.data().started[1].0.clone();
    assert!(
        matches!(&guest.store.data().started[1].1, api::EditorOperation::LocateViewport { target: api::ViewportTarget::Source { offset, .. }, .. } if *offset == source.find("第二段").unwrap())
    );
    guest.receipt(abandoned_source, Ok(api::EditorValue::Unit));
    assert_eq!(guest.store.data().started.len(), 2);
    guest.link("b-0-paragraph", "#%E6%9C%AB%E5%B0%BE");
    assert_eq!(guest.store.data().started.len(), 3);
    let navigation = guest.store.data().started[2].0.clone();
    assert!(
        matches!(&guest.store.data().started[2].1, api::EditorOperation::NavigateDocument { target: api::NavigationTarget::PreviewNode { node, .. }, .. } if node == &format!("b-{}-heading", source.find("# 末尾").unwrap()))
    );
    guest.receipt(abandoned_preview, Ok(api::EditorValue::Unit));
    guest.receipt(navigation, Ok(api::EditorValue::Unit));
    assert_eq!(guest.store.data().started.len(), 3);
    position(&mut guest, source.find("第一段").unwrap());
    assert_eq!(guest.store.data().started.len(), 4);
    let final_request = guest.store.data().started[3].0.clone();
    assert!(
        matches!(&guest.store.data().started[3].1, api::EditorOperation::LocateViewport { target: api::ViewportTarget::Preview { node, .. }, origin: 3, .. } if node == &format!("b-{}-paragraph", source.find("第一段").unwrap()))
    );
    guest.receipt(final_request, Ok(api::EditorValue::Unit));
    assert!(guest.store.data().owned.is_empty());
    eprintln!(
        "takeover trace: source -> preview -> heading -> source, 4 requests; stale receipts add 0 requests; final source paragraph retained"
    );
}
