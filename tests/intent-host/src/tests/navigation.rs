//! Actual packaged navigation behavior crosses public WIT instead of fabricated SDK ownership.
use super::*;

/// Formatting cleanup is independent from navigation, and both owners inspect each terminal notification.
#[test]
fn navigation_replacement_releases_only_its_own_request() {
    let mut guest = Guest::new("[甲](a.md) [乙](b.md)\n");
    guest.click("format-bold");
    guest.link("b-0-paragraph", "a.md");
    guest.link("b-0-paragraph", "b.md");
    assert_eq!(guest.store.data().started.len(), 3);
    assert_eq!(
        guest.store.data().owned.len(),
        2,
        "one formatting and one navigation request must remain"
    );
}

/// Navigation keeps failed cleanup and independent formatting receipts; retries allocate exactly one new open.
#[test]
fn navigation_cleanup_failure_does_not_short_circuit_formatting_delivery() {
    let mut guest = Guest::new("[甲](a.md) [乙](b.md)\n");
    guest.click("format-bold");
    let read = guest.store.data().started[0].0.clone();
    guest.link("b-0-paragraph", "a.md");
    guest.store.data_mut().close_failures = 1;
    guest.link("b-0-paragraph", "b.md");
    assert_eq!(guest.store.data().started.len(), 2);
    assert!(guest.feedback().contains("上次导航请求尚未释放"));
    guest.receipt(
        read,
        Ok(api::EditorValue::DocumentSelection {
            document: document("notes", "notes.md", 1),
            range: api::TextRange { start: 0, end: 0 },
            text: "".into(),
        }),
    );
    assert_eq!(
        guest.store.data().started.len(),
        3,
        "formatting still receives its result and starts its write"
    );
    guest.link("b-0-paragraph", "b.md");
    assert_eq!(guest.store.data().started.len(), 4);
    assert_eq!(guest.store.data().owned.len(), 2);
    assert!(!guest.feedback().contains("上次导航请求尚未释放"));
}

/// A Chinese path keeps its parsed identity for the controlled host resolver; heading identity comes from Preview.
#[test]
fn navigation_handles_both_arrival_orders_and_chinese_targets() {
    for preview_first in [false, true] {
        let mut guest = Guest::new("[打开](中文.md#目标)\n");
        guest.link("b-0-paragraph", "%E4%B8%AD%E6%96%87.md#%E7%9B%AE%E6%A0%87");
        let (handle, operation, deadline) = guest.store.data().started[0].clone();
        assert_eq!(deadline, 30_000);
        assert!(
            matches!(operation, api::EditorOperation::NavigateDocument { target: api::NavigationTarget::RelativeDocument { path }, .. } if path == "中文.md")
        );
        let target = document("target", "中文.md", 1);
        if preview_first {
            guest.preview(Some(target.clone()), "# 目标\n");
        }
        guest.receipt(
            handle,
            Ok(api::EditorValue::Opened {
                document: target.clone(),
            }),
        );
        assert_eq!(
            guest.store.data().started.len(),
            if preview_first { 2 } else { 1 }
        );
        if !preview_first {
            guest.preview(Some(target.clone()), "# 目标\n");
        }
        assert_eq!(guest.store.data().started.len(), 2);
        let (reveal, operation, _) = guest.store.data().started[1].clone();
        assert!(
            matches!(operation, api::EditorOperation::NavigateDocument { document, target: api::NavigationTarget::PreviewNode { node, .. } } if document == target && node == "b-0-heading")
        );
        guest.receipt(reveal, Ok(api::EditorValue::Unit));
        assert!(guest.store.data().owned.is_empty());
        assert!(!guest.feedback().contains("preview-navigation-feedback"));
    }
}

/// A new link replaces Waiting even after the old document was already opened; its later Preview starts no reveal.
#[test]
fn navigation_waiting_for_preview_cannot_reveal_after_replacement() {
    let mut guest = Guest::new("[甲](a.md#目标) [乙](https://example.com)\n");
    guest.link("b-0-paragraph", "a.md#%E7%9B%AE%E6%A0%87");
    let old = guest.store.data().started[0].0.clone();
    guest.receipt(
        old.clone(),
        Ok(api::EditorValue::Opened {
            document: document("target", "a.md", 1),
        }),
    );
    guest.link("b-0-paragraph", "https://example.com");
    guest.receipt(
        old,
        Err(api::Failure::new(api::ErrorCode::TimedOut, "late failure")),
    );
    guest.preview(Some(document("target", "a.md", 1)), "# 目标\n");
    assert_eq!(guest.store.data().started.len(), 2);
    assert!(guest.store.data().owned.is_empty());
    assert!(!guest.feedback().contains("preview-navigation-feedback"));
}

/// Exact real accepted handles filter unrelated and duplicate receipts without altering source or feedback.
#[test]
fn navigation_completion_ignores_unrelated_and_post_terminal_receipts() {
    let mut guest = Guest::new("[网页](https://example.com)\n");
    guest.link("b-0-paragraph", "https://example.com");
    let handle = guest.store.data().started[0].0.clone();
    guest.receipt(
        api::ResourceHandle {
            resource: 99,
            ..handle.clone()
        },
        Err(api::Failure::new(api::ErrorCode::TimedOut, "unrelated")),
    );
    assert_eq!(guest.store.data().owned.len(), 1);
    assert!(!guest.feedback().contains("preview-navigation-feedback"));
    guest.receipt(handle.clone(), Ok(api::EditorValue::Unit));
    guest.receipt(
        handle,
        Err(api::Failure::new(api::ErrorCode::TimedOut, "duplicate")),
    );
    assert!(guest.store.data().owned.is_empty());
    assert!(!guest.feedback().contains("preview-navigation-feedback"));
}

/// Timeout releases the preview driver; bilingual feedback stays bound to the exact source document.
#[test]
fn navigation_terminal_timeout_feedback_is_localized_and_source_bound() {
    let mut guest = Guest::new("[标题](#目标)\n\n# 目标\n");
    guest.link("b-0-paragraph", "#%E7%9B%AE%E6%A0%87");
    let handle = guest.store.data().started[0].0.clone();
    guest.store.data_mut().owned.remove(&handle.resource);
    guest
        .input(api::Input::Event {
            panel: None,
            event: api::Notification::Request {
                handle,
                update: api::RequestUpdate::Cancelled {
                    reason: api::ErrorCode::TimedOut,
                    effect: api::CancellationEffect::NotExecuted,
                },
            },
        })
        .unwrap();
    assert!(guest.feedback().contains("链接导航超时"));
    guest
        .input(api::Input::Event {
            panel: None,
            event: api::Notification::Theme(plugin_protocol::Environment {
                locale: "en-US".into(),
                ..Default::default()
            }),
        })
        .unwrap();
    assert!(guest.feedback().contains("Link navigation timed out"));
    guest.preview(Some(document("other", "other.md", 1)), "其他文档");
    assert!(!guest.feedback().contains("preview-navigation-feedback"));
}
