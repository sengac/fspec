//! Feature: spec/features/rust-tui-pane-borders-changed-files.feature
//!
//! Moved out of tests.rs (COV-075: one test file per feature).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::test_support::*;
use super::*;
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;

/// Scenario: Changed Files view shows a vertical divider between the Files
/// and Diff panes
#[test]
fn changed_files_view_shows_vertical_divider_between_files_and_diff_panes() {
    // @step Given the Changed Files view has at least one changed file to display
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false)]);

    // @step When the view is rendered to the terminal buffer
    let mut term = Terminal::new(TestBackend::new(80, 24)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let files_rect = view
        .last_files_rect
        .expect("files rect cached after render");

    // @step Then a vertical divider glyph is drawn in the column between the Files pane and the Diff pane
    // The divider lives in the gutter column directly to the right of the
    // Files pane content (~40% of the body width).
    let divider_col = files_rect.x + files_rect.width;
    assert!(
        has_vertical_divider_in_column(&buf, divider_col),
        "expected a '│' divider in column {divider_col} between the Files and Diff panes"
    );

    // @step And the divider uses the default terminal colour with no explicit colour set
    let divider_fg = (0..buf.area.height)
        .map(|y| &buf[(divider_col, y)])
        .find(|cell| cell.symbol() == "│")
        .map(|cell| cell.fg)
        .expect("divider cell present");
    assert_eq!(
        divider_fg,
        Color::Reset,
        "divider should use the default terminal colour (Color::Reset)"
    );
}

/// Scenario: Each pane shows a horizontal underline rule beneath its heading
#[test]
fn each_pane_shows_horizontal_underline_rule_beneath_its_heading() {
    // @step Given the Changed Files view has at least one changed file to display
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false)]);

    // @step When the view is rendered to the terminal buffer
    let mut term = Terminal::new(TestBackend::new(80, 24)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let files_rect = view
        .last_files_rect
        .expect("files rect cached after render");

    // @step Then a horizontal underline rule is drawn on the row directly below each pane heading
    // The pane_header reserves [heading(1), underline(1), content(min)], so the
    // '─' rule sits on the row directly below the heading — one row ABOVE the
    // cached content rect (`files_rect.y - 1`). The file rows render below it.
    let rule_row = files_rect.y - 1;
    let has_rule =
        (files_rect.x..files_rect.x + files_rect.width).any(|x| buf[(x, rule_row)].symbol() == "─");
    assert!(
        has_rule,
        "expected a '─' underline rule on row {rule_row} directly below the Files heading"
    );
}

/// Scenario: Empty Changed Files view still shows its empty-state message
#[test]
fn empty_changed_files_view_still_shows_its_empty_state_message() {
    // @step Given the Changed Files view has no changed files
    let mut view = ChangedFilesView::new();
    view.set_files(Vec::new());

    // @step When the view is rendered to the terminal buffer
    let mut term = Terminal::new(TestBackend::new(80, 24)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }

    // @step Then the empty-state message that there are no changed files is shown
    assert!(
        joined.contains("No changed files"),
        "expected the empty-state message, got:\n{joined}"
    );

    // @step And no pane divider is drawn over the empty-state message
    // Find the row carrying the empty-state message and assert it has no '│'.
    let message_row = (0..buf.area.height)
        .find(|&y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
                .contains("No changed files")
        })
        .expect("empty-state message row present");
    let has_divider_over_message =
        (0..buf.area.width).any(|x| buf[(x, message_row)].symbol() == "│");
    assert!(
        !has_divider_over_message,
        "no '│' divider should be painted over the empty-state message row {message_row}"
    );
}
