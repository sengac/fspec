//! Feature: spec/features/changed-files-view-f-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-changed-files-empty-state.feature
//!
//! Moved out of tests.rs (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;
use crate::components::load_state::LoadTracker;
use crate::components::Action;
use crossterm::event::KeyCode;
use ratatui::style::Color;

// ───────────────────────── TUI-108: staged loading dialog ───────────────
// Feature: spec/features/changed-files-view-f-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-changed-files-empty-state.feature

/// Feature: spec/features/changed-files-view-f-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-changed-files-empty-state.feature
///
/// Scenario: Opening the Changed Files view before the scan returns shows the loading dialog instead of the empty message
#[test]
fn pre_load_render_shows_loading_dialog_not_empty_message() {
    // @step Given the Changed Files view is opened
    let mut view = ChangedFilesView::new();

    // @step When the changed files scan has not yet returned
    assert!(view.is_loading(), "fresh view must report loading");

    // @step Then the body shows the animated loading dialog with the label "Loading changed files…"
    let (joined, _cells) = render_grid(&mut view, 100, 20);
    assert!(
        joined.contains("Loading changed files…"),
        "expected the list-stage label, got:\n{joined}"
    );
    assert!(
        joined.contains("Loading changed files"),
        "expected the dialog title, got:\n{joined}"
    );

    // @step And the body does not show "No changed files"
    assert!(
        !joined.contains("No changed files"),
        "the fake empty state must not paint while loading, got:\n{joined}"
    );
}

/// Feature: spec/features/changed-files-view-f-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-changed-files-empty-state.feature
///
/// Scenario: A completed scan with zero files shows the real empty message
#[test]
fn loaded_but_empty_shows_real_empty_message() {
    // @step Given the Changed Files view is opened
    let mut view = ChangedFilesView::new();

    // @step When the changed files scan completes with zero files
    view.set_files(Vec::new());
    view.load.mark_list_flushed();
    view.sync_loading_label();
    assert!(!view.is_loading(), "flushed empty scan must not be loading");

    // @step Then the view shows "No changed files"
    let (joined, _cells) = render_grid(&mut view, 100, 20);
    assert!(joined.contains("No changed files"), "joined:\n{joined}");

    // @step And no loading dialog is shown
    assert!(
        !joined.contains("Loading changed files…"),
        "no dialog after flush, got:\n{joined}"
    );
}

/// Feature: spec/features/changed-files-view-f-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-changed-files-empty-state.feature
///
/// Scenario: Selecting a file after the list loads shows the diff stage label until it folds in
#[test]
fn diff_stage_shows_its_label_until_the_diff_folds_in() {
    // @step Given the changed files list is loaded with at least one file
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("src/foo.rs", "M", false)]);
    view.load.mark_list_flushed();

    // @step When a file diff load is in flight
    view.load.begin_stage(
        &LoadTracker::diff_stage_key_path("src/foo.rs"),
        "Loading diff for src/foo.rs…",
    );
    view.sync_loading_label();

    // @step Then the loading dialog shows the label "Loading diff for <file path>…"
    let (joined, _cells) = render_grid(&mut view, 100, 20);
    assert!(
        joined.contains("Loading diff for src/foo.rs…"),
        "expected the diff-stage label with the actual path, got:\n{joined}"
    );

    // @step When the diff result folds in
    view.set_diff("src/foo.rs", Some("+new".to_string()));
    assert!(
        view.load
            .complete_stage(&LoadTracker::diff_stage_key_path("src/foo.rs")),
        "matching-key stage must complete"
    );
    view.sync_loading_label();
    assert!(!view.is_loading(), "settled cascade must not be loading");

    // @step Then the loading dialog disappears
    let (joined, _cells) = render_grid(&mut view, 100, 20);
    assert!(
        !joined.contains("Loading diff for src/foo.rs…"),
        "dialog must vanish after flush, got:\n{joined}"
    );
    assert!(
        !joined.contains("Loading changed files"),
        "no dialog title after flush, got:\n{joined}"
    );
}

/// Feature: spec/features/changed-files-view-f-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-changed-files-empty-state.feature
///
/// Scenario: A stale diff result for a de-selected path does not clear the current stage
#[test]
fn stale_diff_result_does_not_clear_the_current_stage() {
    // @step Given the diff stage is in flight for the selected path
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false), cf("b.txt", "A", false)]);
    view.load.mark_list_flushed();
    view.load.begin_stage(
        &LoadTracker::diff_stage_key_path("a.txt"),
        "Loading diff for a.txt…",
    );
    view.sync_loading_label();
    assert!(view.is_loading());

    // @step When a diff result arrives for a path that is no longer selected
    // The dispatcher folds the stale result: set_diff is a no-op on
    // selection mismatch and complete_stage is a no-op on key mismatch.
    view.set_diff("b.txt", Some("+stale".to_string()));
    let completed = view
        .load
        .complete_stage(&LoadTracker::diff_stage_key_path("b.txt"));
    assert!(!completed, "stale key must not complete the stage");

    // @step Then the current stage's loading state is unchanged
    assert!(view.is_loading(), "stale result must not clear the stage");
    assert_eq!(
        view.load.active_stage_key(),
        Some(LoadTracker::diff_stage_key_path("a.txt").as_str()),
        "the in-flight stage must still be the selected file's diff"
    );
}

/// Feature: spec/features/changed-files-view-f-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-changed-files-empty-state.feature
///
/// Scenario: ESC is ignored while the loading dialog is active and closes the view after it flushes
#[test]
fn esc_ignored_while_loading_and_closes_after_flush() {
    // @step Given the loading dialog is active
    let mut view = ChangedFilesView::new();
    assert!(view.is_loading());

    // @step When the user presses ESC
    let outcome = view.handle_event(&key(KeyCode::Esc));

    // @step Then the view stays open
    assert!(
        matches!(outcome, ChangedFilesEvent::Ignored),
        "ESC while loading must be Ignored, got {outcome:?}"
    );

    // @step When the loading has flushed
    view.set_files(Vec::new());
    view.load.mark_list_flushed();
    view.sync_loading_label();
    assert!(!view.is_loading());

    // @step When the user presses ESC
    let outcome = view.handle_event(&key(KeyCode::Esc));

    // @step Then the view emits CloseChangedFilesView
    assert!(
        matches!(outcome, ChangedFilesEvent::Close),
        "ESC after flush must close the view, got {outcome:?}"
    );
}

/// Feature: spec/features/changed-files-view-f-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-changed-files-empty-state.feature
///
/// Scenario: The loading dialog renders through the canonical dialog theme
#[test]
fn loading_dialog_renders_through_the_canonical_dialog_theme() {
    use crate::components::loading_dialog::render_loading_dialog;
    use ratatui::layout::Rect;

    // @step Given the loading dialog is active
    let mut view = ChangedFilesView::new();

    // @step When the view is rendered
    let (joined, cells) = render_grid(&mut view, 100, 20);
    assert!(
        joined.contains("Loading changed files"),
        "dialog title must be painted, got:\n{joined}"
    );

    // @step Then the dialog shows a rounded border in the cyan accent
    let corner = cells
        .iter()
        .find(|(sym, _)| matches!(sym.as_str(), "╭" | "╮" | "╰" | "╯"));
    assert!(corner.is_some(), "rounded corner glyphs must be painted");
    let (_, fg) = corner.expect("corner present");
    assert_eq!(*fg, Color::Cyan, "border must use the cyan accent");

    // @step And the dialog title is "Loading changed files"
    assert!(
        joined.contains("Loading changed files"),
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

/// Feature: spec/features/changed-files-view-f-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-changed-files-empty-state.feature
///
/// Scenario: Arrowing while a diff is in flight is swallowed so the selection stays put
#[test]
fn arrowing_while_a_diff_is_in_flight_is_swallowed_so_the_selection_stays_put() {
    // @step Given the changed files list is loaded with three files and the first selected
    let mut view = ChangedFilesView::new();
    view.set_files(vec![
        cf("a.txt", "M", false),
        cf("b.txt", "A", false),
        cf("c.txt", "D", false),
    ]);
    assert_eq!(view.selected_index(), 0);

    // @step And the diff for the first file has folded in
    // (the dispatcher begins the diff stage for the first file, then
    // its result folds in and completes the stage)
    view.load.begin_stage(
        &LoadTracker::diff_stage_key_path("a.txt"),
        "Loading diff for a.txt…",
    );
    view.sync_loading_label();
    view.set_diff("a.txt", Some("+a".to_string()));
    assert!(view
        .load
        .complete_stage(&LoadTracker::diff_stage_key_path("a.txt")));
    view.sync_loading_label();
    assert!(
        !view.is_loading(),
        "settled before the user starts arrowing"
    );

    // @step When the user presses Down
    let outcome = view.handle_event(&key(KeyCode::Down));
    match outcome {
        ChangedFilesEvent::Emit(boxed) => match *boxed {
            Action::LoadFileDiff(path) => assert_eq!(path, "b.txt"),
            other => panic!("expected LoadFileDiff(b.txt), got {other:?}"),
        },
        other => panic!("expected Emit(LoadFileDiff(b.txt)), got {other:?}"),
    }
    // The dispatcher folds the emit by beginning the diff stage.
    view.load.begin_stage(
        &LoadTracker::diff_stage_key_path("b.txt"),
        "Loading diff for b.txt…",
    );
    view.sync_loading_label();

    // @step Then the second file is selected and its diff load is in flight
    assert_eq!(view.selected_index(), 1);
    assert!(view.is_loading(), "the b.txt diff stage must be in flight");
    assert_eq!(
        view.load.active_stage_key(),
        Some(LoadTracker::diff_stage_key_path("b.txt").as_str()),
        "the in-flight stage must be b.txt's diff"
    );

    // @step When the user presses Down again
    let outcome = view.handle_event(&key(KeyCode::Down));

    // @step Then the key is swallowed and the selection stays on the second file
    assert!(
        matches!(outcome, ChangedFilesEvent::Consumed),
        "Down while a diff is in flight must be swallowed, got {outcome:?}"
    );
    assert_eq!(view.selected_index(), 1, "the selection must stay on b.txt");
    assert!(
        view.is_loading(),
        "the b.txt diff stage must still be in flight"
    );

    // @step When the diff result for the second file arrives
    view.set_diff("b.txt", Some("+b".to_string()));
    assert!(view
        .load
        .complete_stage(&LoadTracker::diff_stage_key_path("b.txt")));
    view.sync_loading_label();

    // @step Then the loading dialog disappears
    assert!(!view.is_loading(), "settled cascade must not be loading");
    let (joined, _cells) = render_grid(&mut view, 100, 20);
    assert!(
        !joined.contains("Loading diff for b.txt…"),
        "dialog must vanish after flush, got:\n{joined}"
    );

    // @step And the view shows the diff for the second file
    assert!(
        joined.contains("+b"),
        "the b.txt diff must be painted, got:\n{joined}"
    );
}
