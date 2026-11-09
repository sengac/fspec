//! Feature: spec/features/changed-files-view-click-to-select.feature
//!
//! Moved out of tests.rs (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;
use crate::components::Action;
use crossterm::event::KeyCode;

/// Scenario: Clicking an unselected file row selects it and reloads its diff
#[test]
fn clicking_an_unselected_file_row_selects_it_and_reloads_its_diff() {
    // @step Given a Changed Files view listing a.txt then b.txt with a.txt selected
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false), cf("b.txt", "A", false)]);
    let _ = render_grid(&mut view, 80, 20);
    assert_eq!(view.selected_index(), 0);
    let rect = view
        .last_files_rect
        .expect("files rect cached after render");

    // @step When the user left-clicks the file row for b.txt
    // b.txt is index 1; with file_scroll 0 it sits at content-row rect.y + 1.
    let outcome = left_click(&mut view, rect.x + 1, rect.y + 1);

    // @step Then the selected index becomes 1
    assert_eq!(view.selected_index(), 1);

    // @step And the view requests a diff reload for b.txt
    match outcome {
        ChangedFilesEvent::Emit(boxed) => match *boxed {
            Action::LoadFileDiff(path) => assert_eq!(path, "b.txt"),
            other => panic!("expected LoadFileDiff(b.txt), got {other:?}"),
        },
        other => panic!("expected Emit(LoadFileDiff(b.txt)), got {other:?}"),
    }

    // @step And the focused pane is the file list pane
    assert_eq!(view.focused_pane(), Pane::Files);
}

/// Scenario: Clicking the top visible row selects the file at the scroll offset
#[test]
fn clicking_the_top_visible_row_selects_the_file_at_the_scroll_offset() {
    // @step Given a Changed Files view whose file list is scrolled so the first visible row is index 3
    let mut view = ChangedFilesView::new();
    let files: Vec<ChangedFile> = (0..20)
        .map(|i| cf(&format!("file{i}.txt"), "M", false))
        .collect();
    view.set_files(files);
    // A short terminal keeps the file pane small so paging the selection past
    // the bottom forces file_scroll > 0 (the first visible row is no longer 0).
    let _ = render_grid(&mut view, 80, 8);
    for _ in 0..15 {
        let _ = view.handle_event(&key(KeyCode::Down));
    }
    let _ = render_grid(&mut view, 80, 8);
    let rect = view
        .last_files_rect
        .expect("files rect cached after render");
    let scroll = view.file_scroll();
    assert!(
        scroll > 0,
        "file list must be scrolled (file_scroll > 0), got {scroll}"
    );

    // @step When the user left-clicks the top visible file row
    let outcome = left_click(&mut view, rect.x + 1, rect.y);

    // @step Then the selected index becomes 3
    assert_eq!(
        view.selected_index(),
        scroll,
        "top visible row maps to file_scroll"
    );

    // @step And the view requests a diff reload for the file at index 3
    let expected = format!("file{scroll}.txt");
    match outcome {
        ChangedFilesEvent::Emit(boxed) => match *boxed {
            Action::LoadFileDiff(path) => assert_eq!(path, expected),
            other => panic!("expected LoadFileDiff({expected}), got {other:?}"),
        },
        other => panic!("expected Emit(LoadFileDiff), got {other:?}"),
    }
}

/// Scenario: Clicking the already-selected file row changes nothing
#[test]
fn clicking_the_already_selected_file_row_changes_nothing() {
    // @step Given a Changed Files view listing a.txt then b.txt with a.txt selected
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false), cf("b.txt", "A", false)]);
    let _ = render_grid(&mut view, 80, 20);
    assert_eq!(view.selected_index(), 0);
    let rect = view
        .last_files_rect
        .expect("files rect cached after render");

    // @step When the user left-clicks the file row for a.txt
    let outcome = left_click(&mut view, rect.x + 1, rect.y);

    // @step Then the selected index is still 0
    assert_eq!(view.selected_index(), 0);

    // @step And the view does not request a diff reload
    assert!(
        !matches!(outcome, ChangedFilesEvent::Emit(_)),
        "expected no Emit, got {outcome:?}"
    );
}

/// Scenario: Clicking inside the diff pane focuses it without changing the
/// selection
#[test]
fn clicking_inside_the_diff_pane_focuses_it_without_changing_the_selection() {
    // @step Given a Changed Files view listing a.txt then b.txt with a.txt selected
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false), cf("b.txt", "A", false)]);
    let _ = render_grid(&mut view, 80, 20);
    assert_eq!(view.selected_index(), 0);
    assert_eq!(view.focused_pane(), Pane::Files);
    let diff_rect = view.last_diff_rect.expect("diff rect cached after render");

    // @step When the user left-clicks inside the diff pane
    let outcome = left_click(&mut view, diff_rect.x + 1, diff_rect.y + 1);

    // @step Then the focused pane is the diff pane
    assert_eq!(view.focused_pane(), Pane::Diff);

    // @step And the selected index is still 0
    assert_eq!(view.selected_index(), 0);

    // @step And the view does not request a diff reload
    assert!(
        !matches!(outcome, ChangedFilesEvent::Emit(_)),
        "expected no Emit, got {outcome:?}"
    );
}

/// Scenario: Clicking empty space below the last file changes nothing
#[test]
fn clicking_empty_space_below_the_last_file_changes_nothing() {
    // @step Given a Changed Files view listing a.txt then b.txt with a.txt selected
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false), cf("b.txt", "A", false)]);
    let _ = render_grid(&mut view, 80, 20);
    assert_eq!(view.selected_index(), 0);
    let rect = view
        .last_files_rect
        .expect("files rect cached after render");

    // @step When the user left-clicks the empty area below the last file row
    // Two files occupy rows rect.y and rect.y+1; row rect.y+5 is empty space
    // still inside the pane rect.
    let outcome = left_click(&mut view, rect.x + 1, rect.y + 5);

    // @step Then the selected index is still 0
    assert_eq!(view.selected_index(), 0);

    // @step And the view does not request a diff reload
    assert!(
        !matches!(outcome, ChangedFilesEvent::Emit(_)),
        "expected no Emit, got {outcome:?}"
    );
}
