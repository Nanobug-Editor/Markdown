//! Pure navigation policy tests retain no fabricated SDK owner; real receipt correlation lives in tests/intent-host.
use super::*;
use crate::State;

/// Identity fixtures distinguish actual Preview targets from a guessed path or a reopened document.
fn document(id: &str, path: &str, revision: u64) -> api::DocumentVersion {
    api::DocumentVersion {
        id: id.into(),
        path: path.into(),
        revision,
    }
}

/// Build only the navigation domain stage; no private SDK field or synthetic accepted handle is needed.
fn revealing(version: &api::DocumentVersion) -> Pending {
    Pending {
        document: version.clone(),
        phase: Phase::Revealing,
    }
}

/// Unit acknowledges a queued effect, so later source layout/scene refresh must not restore a Source driver.
#[test]
fn queued_reveal_unit_retains_preview_priority_until_real_source_input() {
    let version = document("target", "next.md", 8);
    let mut state = State {
        source: Some(Source {
            version: version.clone(),
            text: "# 目标\n".into(),
        }),
        navigation: Navigation {
            viewport: Some(version.clone()),
            ..Default::default()
        },
        ..Default::default()
    };
    state.refresh();
    assert!(
        state
            .navigation
            .complete(
                revealing(&version),
                api::EditorValue::Unit,
                state.source.as_ref(),
                &Index::default(),
                &[],
                4
            )
            .is_ok()
    );
    assert!(state.navigation.controls_viewport(Some(&version)));
    assert!(state.viewport_event(
        Some("preview"),
        &api::Notification::SourceViewport(api::SourceViewport {
            document: version.clone(),
            ui_revision: state.revision,
            offset: 0,
            line_fraction: 0.,
            origin: None,
            layout: true,
        })
    ));
    state.refresh();
    state.navigation.preview_drives();
    assert!(state.navigation.controls_viewport(Some(&version)));
    state.navigation.source_drives();
    assert!(!state.navigation.controls_viewport(Some(&version)));
}

/// Receipt-first ownership moves only after its exact target Preview; preview-first ownership awaits its receipt.
#[test]
fn relative_fragment_priority_follows_both_actual_identity_arrival_orders() {
    let origin = document("origin", "notes.md", 3);
    let target = document("target", "next.md", 8);
    for preview_first in [false, true] {
        let mut navigation = Navigation {
            viewport: Some(origin.clone()),
            ..Default::default()
        };
        let pending = Pending {
            document: origin.clone(),
            phase: Phase::Opening {
                fragment: Some("目标".into()),
                transition: Opening::default(),
            },
        };
        if preview_first {
            navigation.pending = Some(pending);
            navigation.source_changed(Some(&target));
            assert!(!navigation.controls_viewport(Some(&origin)));
            assert!(navigation.controls_viewport(Some(&target)));
            let Phase::Opening { transition, .. } = &navigation.pending.as_ref().unwrap().phase
            else {
                panic!("opening stage")
            };
            assert!(matches!(
                transition.complete(&origin, &target, Some(&target)),
                Ok(Arrival::Ready)
            ));
        } else {
            let source = Source {
                version: origin.clone(),
                text: "[目标](next.md#目标)\n".into(),
            };
            assert!(
                navigation
                    .complete(
                        pending,
                        api::EditorValue::Opened {
                            document: target.clone()
                        },
                        Some(&source),
                        &Index::default(),
                        &[],
                        4
                    )
                    .is_ok()
            );
            assert!(navigation.waiting.is_some());
            assert!(!navigation.controls_viewport(Some(&target)));
            navigation.source_changed(Some(&target));
            assert!(navigation.controls_viewport(Some(&target)));
        }
    }
}

/// Even after Unit, source edits, closure, unrelated documents and reopened identities revoke preview priority.
#[test]
fn queued_reveal_priority_is_revoked_by_edit_close_switch_or_reopen() {
    let target = document("target", "next.md", 8);
    for next in [
        None,
        Some(document("target", "next.md", 9)),
        Some(document("other", "other.md", 1)),
        Some(document("reopened", "next.md", 8)),
    ] {
        let mut navigation = Navigation {
            viewport: Some(target.clone()),
            ..Default::default()
        };
        let source = Source {
            version: target.clone(),
            text: "# 目标\n".into(),
        };
        assert!(
            navigation
                .complete(
                    revealing(&target),
                    api::EditorValue::Unit,
                    Some(&source),
                    &Index::default(),
                    &[],
                    4
                )
                .is_ok()
        );
        navigation.source_changed(next.as_ref());
        assert!(!navigation.controls_viewport(Some(&target)));
        assert!(!navigation.controls_viewport(next.as_ref()));
    }
}
