//! Feature: spec/features/checkpoints-view-click-to-select.feature
//!
//! Moved out of tests.rs (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;
use crate::components::Action;
use crossterm::event::KeyCode;

/// Scenario: Clicking a checkpoint name row selects it and loads its files
#[test]
fn clicking_a_checkpoint_name_row_selects_it_and_loads_its_files() {
    // @step Given a Checkpoints view with two checkpoints and the first selected
    let mut view = view_with_two_checkpoints();
    assert_eq!(view.selected_checkpoint(), 0);
    let rect = view
        .last_checkpoints_rect
        .expect("checkpoints rect cached after render");

    // @step When the user left-clicks the second checkpoint's row
    let outcome = left_click(&mut view, rect.x + 1, rect.y + 1);

    // @step Then the selected checkpoint index becomes 1
    assert_eq!(view.selected_checkpoint(), 1);

    // @step And the view emits Action::LoadCheckpointFiles for the second checkpoint
    match outcome {
        CheckpointsEvent::Emit(action) => {
            let Action::LoadCheckpointFiles { work_unit_id, name } = *action else {
                panic!("expected LoadCheckpointFiles(AUTH-002/second), got {action:?}");
            };
            assert_eq!(work_unit_id, "AUTH-002");
            assert_eq!(name, "second");
        }
        other => panic!("expected LoadCheckpointFiles(AUTH-002/second), got {other:?}"),
    }

    // @step And the focused pane is the Checkpoints pane
    assert_eq!(view.focused_pane(), Pane::Checkpoints);
}

/// Scenario: Clicking a file row selects it and loads its diff
#[test]
fn clicking_a_file_row_selects_it_and_loads_its_diff() {
    // @step Given a Checkpoints view whose selected checkpoint lists files a.txt then b.txt with a.txt selected
    let mut view = view_with_two_checkpoints();
    assert_eq!(view.selected_file(), 0);
    let rect = view
        .last_files_rect
        .expect("files rect cached after render");

    // @step When the user left-clicks the file row for b.txt
    // b.txt is file index 1; with file_scroll 0 it sits at content-row rect.y + 1.
    let outcome = left_click(&mut view, rect.x + 1, rect.y + 1);

    // @step Then the selected file index becomes 1
    assert_eq!(view.selected_file(), 1);

    // @step And the view emits Action::LoadCheckpointFileDiff for b.txt
    match outcome {
        CheckpointsEvent::Emit(action) => {
            let Action::LoadCheckpointFileDiff { path, .. } = *action else {
                panic!("expected LoadCheckpointFileDiff(b.txt), got {action:?}");
            };
            assert_eq!(path, "b.txt");
        }
        other => panic!("expected LoadCheckpointFileDiff(b.txt), got {other:?}"),
    }

    // @step And the focused pane is the Files pane
    assert_eq!(view.focused_pane(), Pane::Files);
}

/// Scenario: Clicking inside the diff pane focuses it without changing any
/// selection
#[test]
fn clicking_inside_the_diff_pane_focuses_it_without_changing_any_selection() {
    // @step Given a Checkpoints view with two checkpoints and the first selected
    let mut view = view_with_two_checkpoints();
    assert_eq!(view.selected_checkpoint(), 0);
    let diff_rect = view.last_diff_rect.expect("diff rect cached after render");

    // @step When the user left-clicks inside the Diff pane
    let outcome = left_click(&mut view, diff_rect.x + 1, diff_rect.y + 1);

    // @step Then the focused pane is the Diff pane
    assert_eq!(view.focused_pane(), Pane::Diff);

    // @step And the selected checkpoint index is still 0
    assert_eq!(view.selected_checkpoint(), 0);

    // @step And the view does not emit a checkpoint files or diff reload
    assert!(
        !matches!(outcome, CheckpointsEvent::Emit(_)),
        "expected no Emit, got {outcome:?}"
    );
}

/// Scenario: Clicking empty space below the last checkpoint row changes
/// nothing
#[test]
fn clicking_empty_space_below_the_last_checkpoint_row_changes_nothing() {
    // @step Given a Checkpoints view with two checkpoints and the first selected
    let mut view = view_with_two_checkpoints();
    assert_eq!(view.selected_checkpoint(), 0);
    let rect = view
        .last_checkpoints_rect
        .expect("checkpoints rect cached after render");

    // @step When the user left-clicks the empty area below the last checkpoint row
    // Two checkpoints occupy content rows rect.y and rect.y + 1; clicking
    // rect.y + 2 lands one row past the last populated entry so
    // `row_target` returns None instead of clamping to the last index.
    let empty_row = rect.y + view.selected_checkpoint() as u16 + 2;
    let outcome = left_click(&mut view, rect.x + 1, empty_row);

    // @step Then the selected checkpoint index is still 0
    assert_eq!(view.selected_checkpoint(), 0);

    // @step And the view does not emit a checkpoint files or diff reload
    assert!(
        !matches!(outcome, CheckpointsEvent::Emit(_)),
        "expected no Emit, got {outcome:?}"
    );
}

/// Scenario: A click is swallowed while the restore dialog is open
#[test]
fn a_click_is_swallowed_while_the_restore_dialog_is_open() {
    // @step Given a Checkpoints view with two checkpoints and the first selected and the restore dialog open
    let mut view = view_with_two_checkpoints();
    // Focus the Files pane then open a single-file restore confirmation.
    let _ = view.handle_event(&key(KeyCode::Tab));
    assert_eq!(view.focused_pane(), Pane::Files);
    let _ = view.handle_event(&key(KeyCode::Char('r')));
    assert!(view.dialog().is_some(), "restore dialog must be open");
    assert_eq!(view.selected_checkpoint(), 0);
    let rect = view
        .last_checkpoints_rect
        .expect("checkpoints rect cached after render");

    // @step When the user left-clicks the second checkpoint's row
    let outcome = left_click(&mut view, rect.x + 1, rect.y + 1);

    // @step Then the selected checkpoint index is still 0
    assert_eq!(view.selected_checkpoint(), 0);

    // @step And the view does not emit a checkpoint files or diff reload
    assert!(
        !matches!(outcome, CheckpointsEvent::Emit(_)),
        "expected no Emit while dialog open, got {outcome:?}"
    );
}

/// Scenario: Clicking the already-selected checkpoint row changes nothing
#[test]
fn clicking_the_already_selected_checkpoint_row_changes_nothing() {
    // @step Given a Checkpoints view with two checkpoints and the first selected
    let mut view = view_with_two_checkpoints();
    assert_eq!(view.selected_checkpoint(), 0);
    let rect = view
        .last_checkpoints_rect
        .expect("checkpoints rect cached after render");

    // @step When the user left-clicks the first checkpoint's row
    let outcome = left_click(&mut view, rect.x + 1, rect.y);

    // @step Then the selected checkpoint index is still 0
    assert_eq!(view.selected_checkpoint(), 0);

    // @step And the view does not emit a checkpoint files or diff reload
    assert!(
        !matches!(outcome, CheckpointsEvent::Emit(_)),
        "expected no Emit, got {outcome:?}"
    );
}
