//! Actual packaged formatting behavior crosses public WIT instead of fabricated SDK ownership.
use super::*;

/// A rapid replacement must close its old identity immediately, including before terminal delivery.
#[test]
fn formatting_replacement_releases_old_request_before_accepting_the_next() {
    let mut guest = Guest::new("中文");
    guest.click("format-bold");
    guest.click("format-italic");
    assert_eq!(guest.store.data().started.len(), 2);
    assert_eq!(
        guest.store.data().owned.len(),
        1,
        "replaced requests must not accumulate until the next host poll"
    );
}

/// Failed release blocks B, keeps A owned, ignores A's late result and permits an explicit bounded retry.
#[test]
fn formatting_cleanup_failure_retains_the_owner_and_blocks_replacement_until_retry() {
    let mut guest = Guest::new("中文");
    guest.click("format-bold");
    let old = guest.store.data().started[0].0.clone();
    guest.store.data_mut().close_failures = 1;
    guest.click("format-italic");
    assert_eq!(
        guest.store.data().started.len(),
        1,
        "cleanup failure must block B"
    );
    assert!(guest.feedback().contains("上次编辑请求尚未释放"));
    assert!(guest.store.data().owned.contains_key(&old.resource));
    guest.receipt(
        old,
        Ok(api::EditorValue::DocumentSelection {
            document: document("notes", "notes.md", 1),
            range: api::TextRange { start: 0, end: 6 },
            text: "中文".into(),
        }),
    );
    assert_eq!(
        guest.store.data().started.len(),
        1,
        "a retired read cannot start a write"
    );
    guest.click("format-italic");
    assert_eq!(guest.store.data().started.len(), 2);
    assert_eq!(guest.store.data().owned.len(), 1);
    assert!(!guest.feedback().contains("上次编辑请求尚未释放"));
}

/// Already completed but undelivered A is a terminal race; B alone may initiate a write.
#[test]
fn formatting_completed_before_delivery_cannot_overwrite_the_replacement() {
    let mut guest = Guest::new("中文");
    guest.click("format-bold");
    let old = guest.store.data().started[0].0.clone();
    guest.store.data_mut().owned.remove(&old.resource);
    guest.click("format-italic");
    let current = guest.store.data().started[1].0.clone();
    guest.receipt(
        old,
        Ok(api::EditorValue::DocumentSelection {
            document: document("notes", "notes.md", 1),
            range: api::TextRange { start: 0, end: 6 },
            text: "中文".into(),
        }),
    );
    assert_eq!(guest.store.data().started.len(), 2);
    guest.receipt(
        current,
        Ok(api::EditorValue::DocumentSelection {
            document: document("notes", "notes.md", 1),
            range: api::TextRange { start: 0, end: 6 },
            text: "中文".into(),
        }),
    );
    let (_, operation, deadline) = &guest.store.data().started[2];
    assert_eq!(*deadline, 30_000);
    let api::EditorOperation::ReplaceDocumentRange {
        text,
        expected_selection,
        selection,
        ..
    } = operation
    else {
        panic!("one write expected")
    };
    assert_eq!(text, "*中文*");
    assert_eq!(
        *expected_selection,
        Some(api::TextRange { start: 0, end: 6 })
    );
    assert_eq!(*selection, api::TextRange { start: 1, end: 7 });
}

/// Source identity, revision and UTF-8 boundaries remain independent of SDK request matching.
#[test]
fn formatting_rejects_invalid_or_changed_source_receipts_without_a_write() {
    for (receipt_document, range, text) in [
        (
            document("other", "other.md", 1),
            api::TextRange { start: 0, end: 6 },
            "中文",
        ),
        (
            document("notes", "notes.md", 2),
            api::TextRange { start: 0, end: 6 },
            "中文",
        ),
        (
            document("notes", "notes.md", 1),
            api::TextRange { start: 1, end: 6 },
            "中文",
        ),
    ] {
        let mut guest = Guest::new("中文");
        guest.click("format-bold");
        let handle = guest.store.data().started[0].0.clone();
        guest.receipt(
            handle,
            Ok(api::EditorValue::DocumentSelection {
                document: receipt_document,
                range,
                text: text.into(),
            }),
        );
        assert_eq!(guest.store.data().started.len(), 1);
        assert_eq!(guest.store.data().owned.len(), 0);
    }
}

/// A committed edit survives cancellation; the late receipt may neither restore old feedback nor enqueue work.
#[test]
fn formatting_source_invalidation_retires_a_write_without_claiming_rollback() {
    for next in [
        None,
        Some(document("notes", "notes.md", 2)),
        Some(document("other", "other.md", 1)),
    ] {
        let mut guest = Guest::new("中文");
        guest.click("format-bold");
        let read = guest.store.data().started[0].0.clone();
        guest.receipt(
            read,
            Ok(api::EditorValue::DocumentSelection {
                document: document("notes", "notes.md", 1),
                range: api::TextRange { start: 0, end: 6 },
                text: "中文".into(),
            }),
        );
        let write = guest.store.data().started[1].0.clone();
        guest.preview(next, "**中文**");
        guest.receipt(
            write,
            Ok(api::EditorValue::Edited {
                document: document("notes", "notes.md", 2),
                selection: api::TextRange { start: 2, end: 8 },
            }),
        );
        assert_eq!(guest.store.data().started.len(), 2);
        assert!(guest.store.data().owned.is_empty());
        assert!(!guest.feedback().contains("format-error"));
    }
}

/// Prepare tries every owner and rejects resetting State while either request is still retained.
#[test]
fn preparation_preserves_all_failed_owners_until_explicit_retry() {
    let mut guest = Guest::new("[打开](next.md)\n");
    guest.click("format-bold");
    guest.link("b-0-paragraph", "next.md");
    guest.store.data_mut().close_failures = 2;
    assert!(guest.prepare().is_err());
    assert_eq!(
        guest.store.data().owned.len(),
        2,
        "both cleanup attempts failed without losing either owner"
    );
    guest.prepare().unwrap();
    assert!(guest.store.data().owned.is_empty());
}
