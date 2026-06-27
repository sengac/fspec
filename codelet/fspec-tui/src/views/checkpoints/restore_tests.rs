//! RPC-365 — `CheckpointsView` restore actions + dialog tests.
//!
//! Feature: spec/features/checkpoint-restore.feature
//!
//! Integration-style tests over the real view + dialog sub-state (no
//! mocks): the `r`/`t` keys, confirm/cancel transitions, the emitted
//! `Action::RestoreCheckpoint*`, and the result-driven status dialog.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::dialog::DialogPhase;
use super::*;
use crate::components::Action;
use codelet_rpc_types::{ChangedFile, CheckpointInfo};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

fn ci(work_unit_id: &str, name: &str) -> CheckpointInfo {
    CheckpointInfo {
        work_unit_id: work_unit_id.to_string(),
        name: name.to_string(),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        is_automatic: false,
    }
}

fn cf(path: &str) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        change_type: "M".to_string(),
        staged: false,
    }
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    })
}

/// A view with one checkpoint, files loaded, and focus moved to the
/// Files pane.
fn view_with_files() -> CheckpointsView {
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![ci("AUTH-001", "baseline")]);
    view.set_files("AUTH-001", "baseline", vec![cf("a.txt"), cf("b.txt")]);
    // Cycle focus Checkpoints -> Files.
    let _ = view.handle_event(&key(KeyCode::Tab));
    assert_eq!(view.focused_pane(), Pane::Files);
    view
}

#[test]
fn pressing_r_with_files_pane_focused_opens_single_file_confirmation() {
    // @step Given a Checkpoints view with the Files pane focused and a.txt selected
    let mut view = view_with_files();

    // @step When the user presses the r key
    let outcome = view.handle_event(&key(KeyCode::Char('r')));

    // @step Then a confirmation dialog is open naming a.txt
    assert!(view.dialog().is_some(), "expected a dialog to be open");
    let body = view.dialog().expect("dialog").body_lines().join(" ");
    assert!(body.contains("a.txt"), "dialog body: {body}");
    assert_eq!(view.dialog().expect("dialog").phase, DialogPhase::Confirm);

    // @step And no restore action has been emitted
    assert!(matches!(outcome, CheckpointsEvent::Consumed));
}

#[test]
fn pressing_r_with_checkpoints_pane_focused_is_a_no_op() {
    // @step Given a Checkpoints view with the Checkpoints pane focused and the selected checkpoint has files
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![ci("AUTH-001", "baseline")]);
    view.set_files("AUTH-001", "baseline", vec![cf("a.txt")]);
    assert_eq!(view.focused_pane(), Pane::Checkpoints);

    // @step When the user presses the r key
    let outcome = view.handle_event(&key(KeyCode::Char('r')));

    // @step Then no confirmation dialog is open
    assert!(view.dialog().is_none(), "no dialog should open");

    // @step And no restore action has been emitted
    assert!(matches!(
        outcome,
        CheckpointsEvent::Consumed | CheckpointsEvent::Ignored
    ));
}

#[test]
fn pressing_t_opens_restore_all_confirmation_naming_the_file_count() {
    // @step Given a Checkpoints view whose selected checkpoint changed two files
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![ci("AUTH-001", "baseline")]);
    view.set_files("AUTH-001", "baseline", vec![cf("a.txt"), cf("b.txt")]);

    // @step When the user presses the t key
    let _ = view.handle_event(&key(KeyCode::Char('t')));

    // @step Then a confirmation dialog is open naming 2 files
    let body = view.dialog().expect("dialog").body_lines().join(" ");
    assert!(body.contains('2'), "dialog body: {body}");

    // @step And no restore action has been emitted
    assert_eq!(view.dialog().expect("dialog").phase, DialogPhase::Confirm);
}

#[test]
fn confirming_a_single_file_restore_emits_the_restore_action() {
    // @step Given a confirmation dialog open for restoring the single file a.txt
    let mut view = view_with_files();
    let _ = view.handle_event(&key(KeyCode::Char('r')));
    assert_eq!(view.dialog().expect("dialog").phase, DialogPhase::Confirm);

    // @step When the user presses the y key
    let outcome = view.handle_event(&key(KeyCode::Char('y')));

    // @step Then the view emits Action::RestoreCheckpointFile for a.txt
    match outcome {
        CheckpointsEvent::Emit(Action::RestoreCheckpointFile { path, .. }) => {
            assert_eq!(path, "a.txt");
        }
        other => panic!("expected RestoreCheckpointFile, got {other:?}"),
    }

    // @step And the dialog shows a restoring status
    assert_eq!(view.dialog().expect("dialog").phase, DialogPhase::Restoring);
}

#[test]
fn cancelling_the_restore_confirmation_closes_the_dialog_and_emits_nothing() {
    // @step Given a confirmation dialog open for restoring the single file a.txt
    let mut view = view_with_files();
    let _ = view.handle_event(&key(KeyCode::Char('r')));
    assert!(view.dialog().is_some());

    // @step When the user presses the n key
    let outcome = view.handle_event(&key(KeyCode::Char('n')));

    // @step Then no confirmation dialog is open
    assert!(view.dialog().is_none(), "dialog should be closed");

    // @step And no restore action has been emitted
    assert!(matches!(outcome, CheckpointsEvent::Consumed));
}

#[test]
fn a_restore_result_drives_status_to_complete_then_refreshes_diff_and_counts() {
    // @step Given a Checkpoints view that has dispatched a single-file restore of a.txt
    let mut view = view_with_files();
    let _ = view.handle_event(&key(KeyCode::Char('r')));
    let _ = view.handle_event(&key(KeyCode::Char('y')));
    assert_eq!(view.dialog().expect("dialog").phase, DialogPhase::Restoring);

    // @step When the restore completes successfully
    let follow_ups = view.on_restore_result("AUTH-001", "baseline", Some("a.txt"), None);

    // @step Then the dialog shows a complete status
    assert_eq!(view.dialog().expect("dialog").phase, DialogPhase::Complete);

    // @step And the view emits Action::LoadCheckpointFileDiff for a.txt
    assert!(
        follow_ups.iter().any(|a| matches!(
            a,
            Action::LoadCheckpointFileDiff { path, .. } if path == "a.txt"
        )),
        "expected LoadCheckpointFileDiff for a.txt, got {follow_ups:?}"
    );

    // @step And the view emits Action::RefreshCheckpointCounts
    assert!(
        follow_ups
            .iter()
            .any(|a| matches!(a, Action::RefreshCheckpointCounts)),
        "expected RefreshCheckpointCounts, got {follow_ups:?}"
    );
}

#[test]
fn a_failed_restore_drives_status_to_error_with_the_message() {
    // @step Given a Checkpoints view that has dispatched a single-file restore of a.txt
    let mut view = view_with_files();
    let _ = view.handle_event(&key(KeyCode::Char('r')));
    let _ = view.handle_event(&key(KeyCode::Char('y')));

    // @step When the restore fails with the message disk full
    let follow_ups =
        view.on_restore_result("AUTH-001", "baseline", Some("a.txt"), Some("disk full"));

    // @step Then the dialog shows an error status containing disk full
    match &view.dialog().expect("dialog").phase {
        DialogPhase::Error(msg) => assert!(msg.contains("disk full"), "msg: {msg}"),
        other => panic!("expected Error phase, got {other:?}"),
    }

    // @step And no diff reload action has been emitted
    assert!(
        !follow_ups
            .iter()
            .any(|a| matches!(a, Action::LoadCheckpointFileDiff { .. })),
        "no diff reload expected on error, got {follow_ups:?}"
    );
}
