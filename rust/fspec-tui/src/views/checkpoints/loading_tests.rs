//! Feature: spec/features/checkpoints-view-c-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-checkpoints-empty-state.feature
//!
//! Moved out of tests.rs (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;
use crate::components::load_state::LoadTracker;
use crossterm::event::KeyCode;
use ratatui::style::Color;

// ───────────────────────── TUI-107: staged loading dialog ───────────────
// Feature: spec/features/checkpoints-view-c-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-checkpoints-empty-state.feature

/// Feature: spec/features/checkpoints-view-c-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-checkpoints-empty-state.feature
///
/// Scenario: Opening the Checkpoints view before the list returns shows the loading dialog instead of the empty message
#[test]
fn pre_load_render_shows_loading_dialog_not_empty_message() {
    // @step Given the Checkpoints view is opened
    let mut view = CheckpointsView::new();

    // @step When the checkpoint list request has not yet returned
    assert!(view.is_loading(), "fresh view must report loading");

    // @step Then the body shows the animated loading dialog with the label "Loading checkpoint list…"
    let (joined, _cells) = render_grid(&mut view, 100, 20);
    assert!(
        joined.contains("Loading checkpoint list…"),
        "expected the list-stage label, got:\n{joined}"
    );
    assert!(
        joined.contains("Loading checkpoints"),
        "expected the dialog title, got:\n{joined}"
    );

    // @step And the body does not show "No checkpoints available"
    assert!(
        !joined.contains("No checkpoints available"),
        "the fake empty state must not paint while loading, got:\n{joined}"
    );
}

/// Feature: spec/features/checkpoints-view-c-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-checkpoints-empty-state.feature
///
/// Scenario: A completed list load with zero checkpoints shows the real empty message
#[test]
fn loaded_but_empty_shows_real_empty_message() {
    // @step Given the Checkpoints view is opened
    let mut view = CheckpointsView::new();

    // @step When the checkpoint list request completes with zero checkpoints
    view.set_checkpoints(Vec::new());
    view.load.mark_list_flushed();
    view.sync_loading_label();
    assert!(!view.is_loading(), "flushed empty list must not be loading");

    // @step Then the view shows "No checkpoints available"
    let (joined, _cells) = render_grid(&mut view, 100, 20);
    assert!(
        joined.contains("No checkpoints available"),
        "joined:\n{joined}"
    );

    // @step And no loading dialog is shown
    assert!(
        !joined.contains("Loading checkpoint list…"),
        "no dialog after flush, got:\n{joined}"
    );
}

/// Feature: spec/features/checkpoints-view-c-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-checkpoints-empty-state.feature
///
/// Scenario: Selecting a checkpoint after the list loads shows the files stage label
#[test]
fn files_stage_shows_its_own_label() {
    // @step Given the checkpoint list is loaded with at least one checkpoint
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![ci("AUTH-001", "baseline", false)]);
    view.load.mark_list_flushed();

    // @step When a checkpoint is selected
    // (the dispatcher begins the files stage with this exact label shape)
    view.load.begin_stage(
        &LoadTracker::files_stage_key("AUTH-001", "baseline"),
        "Loading files for baseline…",
    );
    view.sync_loading_label();

    // @step Then the loading dialog shows the label "Loading files for <checkpoint label>…"
    let (joined, _cells) = render_grid(&mut view, 100, 20);
    assert!(
        joined.contains("Loading files for baseline…"),
        "expected the files-stage label, got:\n{joined}"
    );
    assert!(
        !joined.contains("Loading checkpoint list…"),
        "list label must not linger, got:\n{joined}"
    );
}

/// Feature: spec/features/checkpoints-view-c-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-checkpoints-empty-state.feature
///
/// Scenario: Loading a file diff shows the diff stage label until it folds in
#[test]
fn diff_stage_shows_its_label_until_the_diff_folds_in() {
    // @step Given the checkpoint files are loaded with at least one file
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![ci("AUTH-001", "baseline", false)]);
    view.load.mark_list_flushed();
    view.set_files("AUTH-001", "baseline", vec![cf("a.txt", "M")]);

    // @step When a file diff load is in flight
    view.load.begin_stage(
        &LoadTracker::diff_stage_key("AUTH-001", "baseline", "a.txt"),
        "Loading diff for a.txt…",
    );
    view.sync_loading_label();

    // @step Then the loading dialog shows the label "Loading diff for <file path>…"
    let (joined, _cells) = render_grid(&mut view, 100, 20);
    assert!(
        joined.contains("Loading diff for a.txt…"),
        "expected the diff-stage label, got:\n{joined}"
    );

    // @step When the diff result folds in
    view.set_diff("AUTH-001", "baseline", "a.txt", Some("+new".to_string()));
    assert!(
        view.load.complete_stage(&LoadTracker::diff_stage_key(
            "AUTH-001", "baseline", "a.txt"
        )),
        "matching-key stage must complete"
    );
    view.sync_loading_label();
    assert!(!view.is_loading(), "settled cascade must not be loading");

    // @step Then the loading dialog disappears
    let (joined, _cells) = render_grid(&mut view, 100, 20);
    assert!(
        !joined.contains("Loading diff for a.txt…"),
        "dialog must vanish after flush, got:\n{joined}"
    );
    assert!(
        !joined.contains("Loading checkpoints"),
        "no dialog title after flush, got:\n{joined}"
    );
}

/// Feature: spec/features/checkpoints-view-c-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-checkpoints-empty-state.feature
///
/// Scenario: A stale files result for a de-selected checkpoint does not clear the current stage
#[test]
fn stale_files_result_does_not_clear_the_current_stage() {
    // @step Given the files stage is in flight for the selected checkpoint
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![
        ci("AUTH-001", "baseline", false),
        ci("AUTH-002", "second", false),
    ]);
    view.load.mark_list_flushed();
    view.load.begin_stage(
        &LoadTracker::files_stage_key("AUTH-001", "baseline"),
        "Loading files for baseline…",
    );
    view.sync_loading_label();
    assert!(view.is_loading());

    // @step When a files result arrives for a checkpoint that is no longer selected
    // The dispatcher folds the stale result: set_files is a no-op on
    // selection mismatch and complete_stage is a no-op on key mismatch.
    view.set_files("AUTH-002", "second", vec![cf("stale.txt", "M")]);
    let completed = view
        .load
        .complete_stage(&LoadTracker::files_stage_key("AUTH-002", "second"));
    assert!(!completed, "stale key must not complete the stage");

    // @step Then the current stage's loading state is unchanged
    assert!(view.is_loading(), "stale result must not clear the stage");
    assert_eq!(
        view.load.active_stage_key(),
        Some(LoadTracker::files_stage_key("AUTH-001", "baseline").as_str()),
        "the in-flight stage must still be the selected checkpoint's files"
    );
    assert!(
        view.files.is_empty(),
        "stale files must not be folded into the view"
    );
}

/// Feature: spec/features/checkpoints-view-c-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-checkpoints-empty-state.feature
///
/// Scenario: ESC is ignored while the loading dialog is active and closes the view after it flushes
#[test]
fn esc_ignored_while_loading_and_closes_after_flush() {
    // @step Given the loading dialog is active
    let mut view = CheckpointsView::new();
    assert!(view.is_loading());

    // @step When the user presses ESC
    let outcome = view.handle_event(&key(KeyCode::Esc));

    // @step Then the view stays open
    assert!(
        matches!(outcome, CheckpointsEvent::Ignored),
        "ESC while loading must be Ignored, got {outcome:?}"
    );

    // @step When the loading has flushed
    view.set_checkpoints(Vec::new());
    view.load.mark_list_flushed();
    view.sync_loading_label();
    assert!(!view.is_loading());

    // @step When the user presses ESC
    let outcome = view.handle_event(&key(KeyCode::Esc));

    // @step Then the view emits CloseCheckpointsView
    assert!(
        matches!(outcome, CheckpointsEvent::Close),
        "ESC after flush must close the view, got {outcome:?}"
    );
}

/// Feature: spec/features/checkpoints-view-c-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-checkpoints-empty-state.feature
///
/// Scenario: The loading dialog renders through the canonical dialog theme
#[test]
fn loading_dialog_renders_through_the_canonical_dialog_theme() {
    use crate::components::loading_dialog::render_loading_dialog;
    use ratatui::layout::Rect;

    // @step Given the loading dialog is active
    let mut view = CheckpointsView::new();

    // @step When the view is rendered
    let (joined, cells) = render_grid(&mut view, 100, 20);
    assert!(
        joined.contains("Loading checkpoints"),
        "dialog title must be painted, got:\n{joined}"
    );

    // @step Then the dialog shows a rounded border in the cyan accent
    let corner = cells
        .iter()
        .find(|(sym, _)| matches!(sym.as_str(), "╭" | "╮" | "╰" | "╯"));
    assert!(corner.is_some(), "rounded corner glyphs must be painted");
    let (_, fg) = corner.expect("corner present");
    assert_eq!(*fg, Color::Cyan, "border must use the cyan accent");

    // @step And the dialog title is "Loading checkpoints"
    assert!(
        joined.contains("Loading checkpoints"),
        "title text, got:\n{joined}"
    );

    // @step And the spinner glyph advances between 0 ms and 80 ms
    let area = Rect::new(0, 0, 60, 14);
    let mut buf0 = ratatui::buffer::Buffer::empty(area);
    render_loading_dialog(area, &mut buf0, &view.loading, 0);
    let out0: String = buf0
        .content
        .iter()
        .map(|c| c.symbol().to_string())
        .collect();
    let mut buf80 = ratatui::buffer::Buffer::empty(area);
    render_loading_dialog(area, &mut buf80, &view.loading, 80);
    let out80: String = buf80
        .content
        .iter()
        .map(|c| c.symbol().to_string())
        .collect();
    assert!(
        out0.contains("⠋") && !out0.contains("⠙"),
        "t=0 → first glyph only"
    );
    assert!(out80.contains("⠙"), "t=80 → second glyph");
}
