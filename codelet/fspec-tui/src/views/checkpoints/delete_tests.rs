//! RPC-366 — `CheckpointsView` delete actions + dialog tests.
//!
//! Feature: spec/features/checkpoint-delete.feature
//!
//! Integration-style tests over the real view + delete dialog sub-state
//! (no mocks): the `d`/`a` keys, the yes/no single confirmation, the
//! typed `DELETE ALL` confirmation gating, the emitted
//! `Action::DeleteCheckpoint*`, and the result-driven row removal /
//! selection clamp / close-on-empty.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::delete_dialog::DeletePhase;
use super::*;
use crate::components::Action;
use codelet_rpc_types::CheckpointInfo;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

fn ci(work_unit_id: &str, name: &str) -> CheckpointInfo {
    CheckpointInfo {
        work_unit_id: work_unit_id.to_string(),
        name: name.to_string(),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        is_automatic: false,
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

fn type_str(view: &mut CheckpointsView, s: &str) {
    for c in s.chars() {
        let _ = view.handle_event(&key(KeyCode::Char(c)));
    }
}

/// A view with two manual checkpoints, selection on the first.
fn view_with_two() -> CheckpointsView {
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![ci("AUTH-001", "baseline"), ci("AUTH-002", "wip")]);
    view
}

#[test]
fn pressing_d_with_checkpoints_opens_a_single_delete_confirmation_naming_the_checkpoint() {
    // @step Given a Checkpoints view with two checkpoints
    let mut view = view_with_two();

    // @step When the user presses the d key
    let outcome = view.handle_event(&key(KeyCode::Char('d')));

    // @step Then a confirmation dialog is open naming the selected checkpoint
    let dialog = view.delete_dialog().expect("delete dialog");
    assert_eq!(dialog.phase, DeletePhase::ConfirmSingle);
    let body = dialog.body_lines().join(" ");
    assert!(body.contains("baseline"), "dialog body: {body}");

    // @step And no delete action has been emitted
    assert!(matches!(outcome, CheckpointsEvent::Consumed));
}

#[test]
fn confirming_a_single_delete_emits_the_delete_checkpoint_action() {
    // @step Given a single-delete confirmation dialog open for the selected checkpoint
    let mut view = view_with_two();
    let _ = view.handle_event(&key(KeyCode::Char('d')));
    assert_eq!(
        view.delete_dialog().expect("dialog").phase,
        DeletePhase::ConfirmSingle
    );

    // @step When the user presses the y key
    let outcome = view.handle_event(&key(KeyCode::Char('y')));

    // @step Then the view emits Action::DeleteCheckpoint for the selected checkpoint
    match outcome {
        CheckpointsEvent::Emit(Action::DeleteCheckpoint { work_unit_id, name }) => {
            assert_eq!(work_unit_id, "AUTH-001");
            assert_eq!(name, "baseline");
        }
        other => panic!("expected DeleteCheckpoint, got {other:?}"),
    }

    // @step And the dialog shows a deleting status
    assert_eq!(
        view.delete_dialog().expect("dialog").phase,
        DeletePhase::Deleting
    );
}

#[test]
fn cancelling_the_single_delete_confirmation_closes_the_dialog_and_emits_nothing() {
    // @step Given a single-delete confirmation dialog open for the selected checkpoint
    let mut view = view_with_two();
    let _ = view.handle_event(&key(KeyCode::Char('d')));
    assert!(view.delete_dialog().is_some());

    // @step When the user presses the n key
    let outcome = view.handle_event(&key(KeyCode::Char('n')));

    // @step Then no delete dialog is open
    assert!(view.delete_dialog().is_none(), "dialog should be closed");

    // @step And no delete action has been emitted
    assert!(matches!(outcome, CheckpointsEvent::Consumed));
}

#[test]
fn a_single_delete_result_removes_the_row_clamps_the_selection_and_reloads_the_new_selection() {
    // @step Given a Checkpoints view that has dispatched a single delete of the last of two checkpoints
    let mut view = view_with_two();
    // Move selection to the second (last) checkpoint, then confirm delete.
    let _ = view.handle_event(&key(KeyCode::Down));
    assert_eq!(view.selected_checkpoint(), 1);
    let _ = view.handle_event(&key(KeyCode::Char('d')));
    let _ = view.handle_event(&key(KeyCode::Char('y')));
    assert_eq!(
        view.delete_dialog().expect("dialog").phase,
        DeletePhase::Deleting
    );

    // @step When the delete completes successfully
    let follow_ups = view.on_delete_result("AUTH-002", "wip", false, None);

    // @step Then the deleted checkpoint is removed leaving one checkpoint
    assert_eq!(view.checkpoint_count_total(), 1);
    assert_eq!(
        view.selected_checkpoint_info().map(|c| c.work_unit_id.clone()),
        Some("AUTH-001".to_string())
    );

    // @step And the selection is clamped to the remaining checkpoint
    assert_eq!(view.selected_checkpoint(), 0);

    // @step And the view emits Action::LoadCheckpointFiles for the remaining checkpoint
    assert!(
        follow_ups.iter().any(|a| matches!(
            a,
            Action::LoadCheckpointFiles { work_unit_id, .. } if work_unit_id == "AUTH-001"
        )),
        "expected LoadCheckpointFiles for AUTH-001, got {follow_ups:?}"
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
fn deleting_the_only_remaining_checkpoint_returns_the_view_to_the_board() {
    // @step Given a Checkpoints view that has dispatched a single delete of its only checkpoint
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![ci("AUTH-001", "baseline")]);
    let _ = view.handle_event(&key(KeyCode::Char('d')));
    let _ = view.handle_event(&key(KeyCode::Char('y')));

    // @step When the delete completes successfully
    let follow_ups = view.on_delete_result("AUTH-001", "baseline", false, None);

    // @step Then the checkpoint list is empty
    assert!(view.is_empty(), "list should be empty");

    // @step And the view emits Action::CloseCheckpointsView
    assert!(
        follow_ups
            .iter()
            .any(|a| matches!(a, Action::CloseCheckpointsView)),
        "expected CloseCheckpointsView, got {follow_ups:?}"
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
fn pressing_a_opens_a_typed_confirmation_that_stays_disabled_until_delete_all_is_typed_exactly() {
    // @step Given a Checkpoints view with two checkpoints
    let mut view = view_with_two();

    // @step When the user presses the a key
    let outcome = view.handle_event(&key(KeyCode::Char('a')));

    // @step Then a typed-confirmation dialog is open requiring the phrase DELETE ALL
    let dialog = view.delete_dialog().expect("delete dialog");
    assert!(matches!(dialog.phase, DeletePhase::ConfirmAll { .. }));
    let body = dialog.body_lines().join(" ");
    assert!(body.contains("DELETE ALL"), "dialog body: {body}");

    // @step And the confirmation is disabled until DELETE ALL is typed exactly
    assert!(
        !dialog.all_confirm_ready(),
        "confirm must be disabled with empty input"
    );

    // @step And no delete action has been emitted
    assert!(matches!(outcome, CheckpointsEvent::Consumed));
}

#[test]
fn pressing_enter_before_delete_all_is_typed_exactly_does_not_dispatch() {
    // @step Given a typed-confirmation delete-all dialog with the partial text DELETE typed
    let mut view = view_with_two();
    let _ = view.handle_event(&key(KeyCode::Char('a')));
    type_str(&mut view, "DELETE");
    assert!(!view.delete_dialog().expect("dialog").all_confirm_ready());

    // @step When the user presses the Enter key
    let outcome = view.handle_event(&key(KeyCode::Enter));

    // @step Then no delete action has been emitted
    assert!(matches!(outcome, CheckpointsEvent::Consumed));

    // @step And the typed-confirmation dialog is still open
    assert!(matches!(
        view.delete_dialog().expect("dialog").phase,
        DeletePhase::ConfirmAll { .. }
    ));
}

#[test]
fn confirming_the_typed_delete_all_dispatches_delete_all_checkpoints() {
    // @step Given a typed-confirmation delete-all dialog with DELETE ALL typed exactly
    let mut view = view_with_two();
    let _ = view.handle_event(&key(KeyCode::Char('a')));
    type_str(&mut view, "DELETE ALL");
    assert!(view.delete_dialog().expect("dialog").all_confirm_ready());

    // @step When the user presses the Enter key
    let outcome = view.handle_event(&key(KeyCode::Enter));

    // @step Then the view emits Action::DeleteAllCheckpoints
    assert!(
        matches!(outcome, CheckpointsEvent::Emit(Action::DeleteAllCheckpoints)),
        "expected DeleteAllCheckpoints, got {outcome:?}"
    );

    // @step And the dialog shows a deleting status
    assert_eq!(
        view.delete_dialog().expect("dialog").phase,
        DeletePhase::Deleting
    );
}

#[test]
fn cancelling_the_delete_all_dialog_makes_no_transport_call_and_leaves_the_list_unchanged() {
    // @step Given a typed-confirmation delete-all dialog is open with two checkpoints present
    let mut view = view_with_two();
    let _ = view.handle_event(&key(KeyCode::Char('a')));
    assert!(view.delete_dialog().is_some());

    // @step When the user presses the Esc key
    let outcome = view.handle_event(&key(KeyCode::Esc));

    // @step Then no delete dialog is open
    assert!(view.delete_dialog().is_none(), "dialog should be closed");

    // @step And no delete action has been emitted
    assert!(matches!(outcome, CheckpointsEvent::Consumed));

    // @step And the checkpoint list still has two checkpoints
    assert_eq!(view.checkpoint_count_total(), 2);
}
