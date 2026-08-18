//! RPC-356 — `ChangedFilesView` component tests.
//!
//! Feature: spec/features/rust-changed-files-view.feature
//!
//! Integration-style ratatui `TestBackend` buffer assertions over the
//! real view (no mocks): row rendering with status colors + cursor,
//! diff coloring, pane-focus toggle, selection navigation reloading the
//! diff, empty state, Esc → close, and mouse-wheel diff scroll.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::components::{Action, EventResult};
use crate::store::BoardStore;
use crate::theme::Theme;
use crate::views::board::BoardView;
use crate::views::navigator::{Navigator, ViewMode};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;

fn cf(path: &str, ct: &str, staged: bool) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        change_type: ct.to_string(),
        staged,
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

/// Render the view to a `(joined symbols, per-cell fg colors)` grid.
fn render_grid(view: &mut ChangedFilesView, w: u16, h: u16) -> (String, Vec<(String, Color)>) {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    let mut cells = Vec::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            joined.push_str(cell.symbol());
            cells.push((cell.symbol().to_string(), cell.fg));
        }
        joined.push('\n');
    }
    (joined, cells)
}

fn color_of(cells: &[(String, Color)], symbol: &str) -> Option<Color> {
    cells.iter().find(|(s, _)| s == symbol).map(|(_, c)| *c)
}

#[test]
fn file_list_renders_colored_status_letters_and_cursor() {
    // @step Given a Changed Files view with a modified a.txt and an added b.txt
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false), cf("b.txt", "A", false)]);

    // @step When the view is rendered
    let (joined, cells) = render_grid(&mut view, 80, 20);

    // @step Then the row for a.txt shows a yellow M status letter
    assert!(joined.contains("a.txt"));
    assert_eq!(color_of(&cells, "M"), Some(Color::Yellow));

    // @step And the row for b.txt shows a green A status letter
    assert!(joined.contains("b.txt"));
    assert_eq!(color_of(&cells, "A"), Some(Color::Green));

    // @step And the selected row shows a > cursor while other rows show a space
    assert!(joined.contains('>'));
    assert_eq!(view.selected_index(), 0);
}

/// Render the view and return a per-row `(text, leading-cell fg)` list
/// for the diff pane content. Lets tests assert the colour of the first
/// glyph of a specific diff line (e.g. the `+` of `+newline`) without
/// being fooled by an identical glyph elsewhere on the screen.
fn diff_rows(view: &mut ChangedFilesView, w: u16, h: u16) -> Vec<(String, Color)> {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut rows = Vec::new();
    for y in 0..buf.area.height {
        let mut text = String::new();
        let mut leading: Option<Color> = None;
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            let sym = cell.symbol();
            // Skip structural chrome (pane divider '│' / heading rule '─')
            // so the leading colour reflects the diff line's own first glyph
            // (RPC-367 added an inter-pane divider to the left of this pane).
            if leading.is_none() && sym.trim() != "" && sym != "│" && sym != "─" {
                leading = Some(cell.fg);
            }
            text.push_str(sym);
        }
        rows.push((text.trim_end().to_string(), leading.unwrap_or(Color::Reset)));
    }
    rows
}

fn row_color_for(rows: &[(String, Color)], needle: &str) -> Option<Color> {
    rows.iter()
        .find(|(t, _)| t.contains(needle))
        .map(|(_, c)| *c)
}

#[test]
fn diff_pane_renders_colored_add_remove_and_hunk_lines() {
    // @step Given a Changed Files view whose selected file diff has an added, a removed and a hunk-header line
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false)]);
    view.set_diff("a.txt", Some("@@ -1 +1 @@\n-oldline\n+newline".to_string()));

    // @step When the view is rendered
    let rows = diff_rows(&mut view, 80, 20);

    // @step Then the diff pane shows the added line in green
    assert_eq!(row_color_for(&rows, "newline"), Some(Color::Green));

    // @step And the diff pane shows the removed line in red
    assert_eq!(row_color_for(&rows, "oldline"), Some(Color::Red));

    // @step And the diff pane shows the hunk-header line dimmed
    assert_eq!(row_color_for(&rows, "@@ -1 +1 @@"), Some(Color::Cyan));
}

#[test]
fn moving_selection_down_reloads_diff_for_new_file() {
    // @step Given a Changed Files view listing a.txt then b.txt with a.txt selected
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false), cf("b.txt", "A", false)]);
    assert_eq!(view.selected_index(), 0);

    // @step When the user presses the Down key
    let outcome = view.handle_event(&key(KeyCode::Down));

    // @step Then the selected index becomes 1
    assert_eq!(view.selected_index(), 1);

    // @step And the view requests a diff reload for b.txt
    match outcome {
        ChangedFilesEvent::Emit(Action::LoadFileDiff(path)) => assert_eq!(path, "b.txt"),
        other => panic!("expected LoadFileDiff(b.txt), got {other:?}"),
    }
}

#[test]
fn tab_moves_focus_to_diff_then_pgdn_scrolls_diff_not_list() {
    // @step Given a Changed Files view focused on the file list pane
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false)]);
    let long_diff = (0..200)
        .map(|i| format!(" context line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    view.set_diff("a.txt", Some(long_diff));
    // Render once so the pane rects (and page step) are known.
    let _ = render_grid(&mut view, 80, 20);
    assert_eq!(view.focused_pane(), Pane::Files);

    // @step When the user presses the Tab key
    let _ = view.handle_event(&key(KeyCode::Tab));

    // @step Then the focused pane is the diff pane
    assert_eq!(view.focused_pane(), Pane::Diff);

    let before_selection = view.selected_index();
    let before_diff = view.diff_scroll();

    // @step When the user presses the PgDn key
    let _ = view.handle_event(&key(KeyCode::PageDown));

    // @step Then the diff pane scroll offset increases and the file selection is unchanged
    assert!(view.diff_scroll() > before_diff);
    assert_eq!(view.selected_index(), before_selection);
}

#[test]
fn empty_repo_shows_message_and_esc_closes() {
    // @step Given a Changed Files view with no changed files
    let mut view = ChangedFilesView::new();
    view.set_files(Vec::new());

    // @step When the view is rendered
    let (joined, _cells) = render_grid(&mut view, 80, 20);

    // @step Then the view shows the No changed files message
    assert!(joined.contains("No changed files"));

    // @step When the user presses the Esc key
    let outcome = view.handle_event(&key(KeyCode::Esc));

    // @step Then the view emits Action::CloseChangedFilesView
    assert!(matches!(outcome, ChangedFilesEvent::Close));
}

#[test]
fn mouse_wheel_scrolls_focused_diff_pane_by_wheel_velocity_step() {
    // @step Given a Changed Files view focused on the diff pane with a long diff
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false)]);
    let long_diff = (0..200)
        .map(|i| format!(" context line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    view.set_diff("a.txt", Some(long_diff));
    let _ = render_grid(&mut view, 80, 20);
    let _ = view.handle_event(&key(KeyCode::Tab));
    assert_eq!(view.focused_pane(), Pane::Diff);
    let before = view.diff_scroll();
    let diff_rect = view.last_diff_rect.expect("diff rect cached after render");

    // @step When a mouse wheel ScrollDown event arrives over the diff pane
    let mouse = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: diff_rect.x + 1,
        row: diff_rect.y + 1,
        modifiers: KeyModifiers::NONE,
    });
    let _ = view.handle_event(&mouse);

    // @step Then the diff pane scroll offset advances by the WheelVelocity step
    assert!(view.diff_scroll() > before);
}

/// Scenario: Diff pane scroll stops at the last full page
#[test]
fn diff_pane_scroll_stops_at_the_last_full_page() {
    // @step Given a Changed Files view focused on the diff pane with a long diff and a known pane height
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false)]);
    let line_count = 200usize;
    let long_diff = (0..line_count)
        .map(|i| format!(" context line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    view.set_diff("a.txt", Some(long_diff));
    let _ = render_grid(&mut view, 80, 20);
    let _ = view.handle_event(&key(KeyCode::Tab));
    assert_eq!(view.focused_pane(), Pane::Diff);
    let diff_rect = view.last_diff_rect.expect("diff rect cached after render");
    let viewport_height = diff_rect.height as usize;
    assert!(
        viewport_height > 0,
        "viewport height must be known after render"
    );

    // @step When the user pages down far past the end of the diff
    for _ in 0..50 {
        let _ = view.handle_event(&key(KeyCode::PageDown));
    }

    // @step Then the diff pane scroll offset never exceeds the diff line count minus the pane height
    let max_scroll = line_count.saturating_sub(viewport_height);
    assert!(
        view.diff_scroll() <= max_scroll,
        "diff_scroll {} overshot last full page (max {})",
        view.diff_scroll(),
        max_scroll
    );
    assert_eq!(
        view.diff_scroll(),
        max_scroll,
        "should rest at the last full page"
    );
}

// ───────────────────────── board + navigator wiring ─────────────────────

/// Scenario: Mouse wheel selection over the file list reloads the diff for
/// the newly selected file
#[test]
fn mouse_wheel_over_file_list_reloads_diff_for_new_file() {
    // @step Given a Changed Files view listing a.txt then b.txt with a.txt selected
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false), cf("b.txt", "A", false)]);
    let _ = render_grid(&mut view, 80, 20);
    assert_eq!(view.selected_index(), 0);
    let files_rect = view
        .last_files_rect
        .expect("files rect cached after render");

    // @step When a mouse wheel ScrollDown event arrives over the file list pane
    let mouse = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: files_rect.x + 1,
        row: files_rect.y + 1,
        modifiers: KeyModifiers::NONE,
    });
    let outcome = view.handle_event(&mouse);

    // @step Then the selected index becomes 1
    assert_eq!(view.selected_index(), 1);

    // @step And the view requests a diff reload for b.txt
    match outcome {
        ChangedFilesEvent::Emit(Action::LoadFileDiff(path)) => assert_eq!(path, "b.txt"),
        other => panic!("expected LoadFileDiff(b.txt), got {other:?}"),
    }
}

/// Scenario: Mouse wheel over the diff pane scrolls the diff and leaves the
/// file selection unchanged
#[test]
fn mouse_wheel_over_diff_pane_scrolls_diff_without_changing_selection() {
    // @step Given a Changed Files view listing a.txt then b.txt with a.txt selected focused on the diff pane with a long diff
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false), cf("b.txt", "A", false)]);
    let long_diff = (0..200)
        .map(|i| format!(" context line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    view.set_diff("a.txt", Some(long_diff));
    let _ = render_grid(&mut view, 80, 20);
    let _ = view.handle_event(&key(KeyCode::Tab));
    assert_eq!(view.focused_pane(), Pane::Diff);
    assert_eq!(view.selected_index(), 0);
    let before = view.diff_scroll();
    let diff_rect = view.last_diff_rect.expect("diff rect cached after render");

    // @step When a mouse wheel ScrollDown event arrives over the diff pane
    let mouse = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: diff_rect.x + 1,
        row: diff_rect.y + 1,
        modifiers: KeyModifiers::NONE,
    });
    let _ = view.handle_event(&mouse);

    // @step Then the diff pane scroll offset advances by the WheelVelocity step
    assert!(view.diff_scroll() > before);

    // @step And the selected index is still 0
    assert_eq!(view.selected_index(), 0);
}

/// Build a Changed Files view with a long diff, render it (caching rects)
/// and move focus to the diff pane via Tab.
fn diff_focused_view() -> ChangedFilesView {
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false), cf("b.txt", "A", false)]);
    let long_diff = (0..200)
        .map(|i| format!(" context line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    view.set_diff("a.txt", Some(long_diff));
    let _ = render_grid(&mut view, 80, 20);
    let _ = view.handle_event(&key(KeyCode::Tab));
    view
}

/// Scenario: With the diff pane focused the Down key scrolls the diff down
/// one line
#[test]
fn diff_focused_down_scrolls_diff_one_line() {
    // @step Given a Changed Files view focused on the diff pane with a long diff
    let mut view = diff_focused_view();
    assert_eq!(view.focused_pane(), Pane::Diff);
    assert_eq!(view.diff_scroll(), 0);

    // @step When the user presses the Down key
    let _ = view.handle_event(&key(KeyCode::Down));

    // @step Then the diff pane scroll offset increases from 0 to 1
    assert_eq!(view.diff_scroll(), 1);
}

/// Scenario: With the diff pane focused the Up key at the top keeps the diff
/// scroll clamped at zero
#[test]
fn diff_focused_up_at_top_clamps_scroll_at_zero() {
    // @step Given a Changed Files view focused on the diff pane with a long diff at diff scroll 0
    let mut view = diff_focused_view();
    assert_eq!(view.focused_pane(), Pane::Diff);
    assert_eq!(view.diff_scroll(), 0);

    // @step When the user presses the Up key
    let _ = view.handle_event(&key(KeyCode::Up));

    // @step Then the diff pane scroll offset stays at 0
    assert_eq!(view.diff_scroll(), 0);
}

/// Scenario: With the diff pane focused the Down key does not change the file
/// selection
#[test]
fn diff_focused_down_leaves_selection_unchanged() {
    // @step Given a Changed Files view listing a.txt then b.txt with a.txt selected focused on the diff pane with a long diff
    let mut view = diff_focused_view();
    assert_eq!(view.focused_pane(), Pane::Diff);
    assert_eq!(view.selected_index(), 0);

    // @step When the user presses the Down key
    let _ = view.handle_event(&key(KeyCode::Down));

    // @step Then the selected index is still 0
    assert_eq!(view.selected_index(), 0);
}

/// Scenario: With the file list pane focused the Down key still moves the
/// selection and reloads the diff
#[test]
fn files_focused_down_moves_selection_and_reloads_diff() {
    // @step Given a Changed Files view listing a.txt then b.txt with a.txt selected focused on the file list pane
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false), cf("b.txt", "A", false)]);
    let _ = render_grid(&mut view, 80, 20);
    assert_eq!(view.focused_pane(), Pane::Files);
    assert_eq!(view.selected_index(), 0);

    // @step When the user presses the Down key
    let outcome = view.handle_event(&key(KeyCode::Down));

    // @step Then the selected index becomes 1
    assert_eq!(view.selected_index(), 1);

    // @step And the view requests a diff reload for b.txt
    match outcome {
        ChangedFilesEvent::Emit(Action::LoadFileDiff(path)) => assert_eq!(path, "b.txt"),
        other => panic!("expected LoadFileDiff(b.txt), got {other:?}"),
    }
}

// ───────────────────────── RPC-368: click to select ─────────────────────
// Feature: spec/features/changed-files-view-click-to-select.feature

/// Dispatch a synthetic left mouse Down event at the given screen cell.
fn left_click(view: &mut ChangedFilesView, column: u16, row: u16) -> ChangedFilesEvent {
    view.handle_event(&Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }))
}

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
        ChangedFilesEvent::Emit(Action::LoadFileDiff(path)) => assert_eq!(path, "b.txt"),
        other => panic!("expected LoadFileDiff(b.txt), got {other:?}"),
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
        ChangedFilesEvent::Emit(Action::LoadFileDiff(path)) => assert_eq!(path, expected),
        other => panic!("expected LoadFileDiff({expected}), got {other:?}"),
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

/// Render the view and return the raw `TestBackend` buffer so tests can
/// inspect individual cells (used for scrollbar glyph assertions).
fn render_buffer(view: &mut ChangedFilesView, w: u16, h: u16) -> ratatui::buffer::Buffer {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

/// Count scrollbar glyphs (`■` thumb / `│` track painted by
/// `render_list_scrollbar`) in the single column `col` over the rows of
/// `rect`.
fn scrollbar_glyphs_in_column(buf: &ratatui::buffer::Buffer, col: u16, rect: Rect) -> usize {
    let mut count = 0;
    for y in rect.y..rect.y.saturating_add(rect.height) {
        let sym = buf[(col, y)].symbol();
        if sym == "■" || sym == "│" {
            count += 1;
        }
    }
    count
}

/// Scenario: A file list taller than the pane renders a scrollbar in the
/// file list pane
#[test]
fn file_list_taller_than_pane_renders_scrollbar() {
    // @step Given a Changed Files view with 50 files rendered in a 10-row pane
    let mut view = ChangedFilesView::new();
    let files: Vec<ChangedFile> = (0..50)
        .map(|i| cf(&format!("file{i}.txt"), "M", false))
        .collect();
    view.set_files(files);

    // @step When the view is rendered
    let buf = render_buffer(&mut view, 80, 14);
    let rect = view
        .last_files_rect
        .expect("files rect cached after render");
    let scrollbar_col = rect.x + rect.width - 1;

    // @step Then a vertical scrollbar is painted in the rightmost column of the file list pane
    assert!(
        scrollbar_glyphs_in_column(&buf, scrollbar_col, rect) > 0,
        "expected scrollbar glyphs in files pane column {scrollbar_col}"
    );
}

/// Scenario: A diff longer than the pane renders a scrollbar in the diff pane
#[test]
fn diff_longer_than_pane_renders_scrollbar() {
    // @step Given a Changed Files view with a 200-line diff rendered in a small pane
    let mut view = ChangedFilesView::new();
    view.set_files(vec![cf("a.txt", "M", false)]);
    let long_diff = (0..200)
        .map(|i| format!(" context line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    view.set_diff("a.txt", Some(long_diff));

    // @step When the view is rendered
    let buf = render_buffer(&mut view, 80, 14);
    let rect = view.last_diff_rect.expect("diff rect cached after render");
    let scrollbar_col = rect.x + rect.width - 1;

    // @step Then a vertical scrollbar is painted in the rightmost column of the diff pane
    assert!(
        scrollbar_glyphs_in_column(&buf, scrollbar_col, rect) > 0,
        "expected scrollbar glyphs in diff pane column {scrollbar_col}"
    );
}

/// Scenario: A file list that fits the pane renders no scrollbar
#[test]
fn file_list_that_fits_renders_no_scrollbar() {
    // @step Given a Changed Files view with 3 files rendered in a 10-row pane
    let mut view = ChangedFilesView::new();
    // First path is intentionally long so that, when the full pane width is
    // used (no gutter reserved), its truncation ellipsis lands in the
    // rightmost content column.
    let long_path = "a".repeat(200);
    view.set_files(vec![
        cf(&long_path, "M", false),
        cf("b.txt", "A", false),
        cf("c.txt", "D", false),
    ]);

    // @step When the view is rendered
    let buf = render_buffer(&mut view, 80, 14);
    let rect = view
        .last_files_rect
        .expect("files rect cached after render");
    let scrollbar_col = rect.x + rect.width - 1;

    // @step Then no scrollbar is painted in the file list pane
    assert_eq!(
        scrollbar_glyphs_in_column(&buf, scrollbar_col, rect),
        0,
        "expected NO scrollbar glyphs in files pane column {scrollbar_col}"
    );

    // @step And the file list content occupies the full pane width
    // With the gutter reclaimed, the long first-row path is truncated to the
    // FULL content width, so its trailing ellipsis sits in the rightmost
    // content column. A reserved gutter would shift it one column left and
    // leave this cell blank.
    let last_cell = buf[(scrollbar_col, rect.y)].symbol().to_string();
    assert_eq!(
        last_cell, "…",
        "expected the file row to fill the full pane width (ellipsis in rightmost column), got {last_cell:?}"
    );
}

// ───────────────────────── RPC-367: pane borders ────────────────────────
// Feature: spec/features/rust-tui-pane-borders-changed-files.feature

/// True if a vertical divider glyph `│` appears anywhere in column `col`
/// of the rendered buffer.
fn has_vertical_divider_in_column(buf: &ratatui::buffer::Buffer, col: u16) -> bool {
    (0..buf.area.height).any(|y| buf[(col, y)].symbol() == "│")
}

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

#[test]
fn pressing_f_on_the_board_emits_open_changed_files_view() {
    // @step Given the BoardView is the active view
    let (tx, mut rx) = unbounded_channel();
    let board = BoardView::new(Arc::new(Theme::default()), tx);
    let store = BoardStore::default();

    // @step When the user presses the F key
    let result = board.handle_event(&key(KeyCode::Char('F')), &store);

    // @step Then the BoardView emits Action::OpenChangedFilesView
    let mut saw_open = false;
    while let Ok(action) = rx.try_recv() {
        if matches!(action, Action::OpenChangedFilesView) {
            saw_open = true;
        }
    }
    assert!(saw_open, "expected Action::OpenChangedFilesView on the bus");

    // @step And the key event is consumed
    assert!(matches!(result, EventResult::Consumed(_)));
}

// ───────────────────────── TUI-108: staged loading dialog ───────────────
// Feature: spec/features/changed-files-view-f-shows-staged-animated-loading-dialog-via-shared-base-instead-of-fake-no-changed-files-empty-state.feature

use crate::components::load_state::LoadTracker;

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
    assert!(
        joined.contains("No changed files"),
        "joined:\n{joined}"
    );

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
    view.load
        .begin_stage(&LoadTracker::diff_stage_key_path("src/foo.rs"), "Loading diff for src/foo.rs…");
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
    view.load
        .begin_stage(&LoadTracker::diff_stage_key_path("a.txt"), "Loading diff for a.txt…");
    view.sync_loading_label();
    assert!(view.is_loading());

    // @step When a diff result arrives for a path that is no longer selected
    // The dispatcher folds the stale result: set_diff is a no-op on
    // selection mismatch and complete_stage is a no-op on key mismatch.
    view.set_diff("b.txt", Some("+stale".to_string()));
    let completed = view.load
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
    let out0: String = buf0.content.iter().map(|c| c.symbol().to_string()).collect();
    let mut buf80 = ratatui::buffer::Buffer::empty(area);
    render_loading_dialog(area, &mut buf80, &view.loading, 80);
    let out80: String = buf80.content.iter().map(|c| c.symbol().to_string()).collect();
    assert!(out0.contains("⠋") && !out0.contains("⠙"), "t=0 → first glyph only");
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
    view.load
        .begin_stage(&LoadTracker::diff_stage_key_path("a.txt"), "Loading diff for a.txt…");
    view.sync_loading_label();
    view.set_diff("a.txt", Some("+a".to_string()));
    assert!(view.load.complete_stage(&LoadTracker::diff_stage_key_path("a.txt")));
    view.sync_loading_label();
    assert!(!view.is_loading(), "settled before the user starts arrowing");

    // @step When the user presses Down
    let outcome = view.handle_event(&key(KeyCode::Down));
    match outcome {
        ChangedFilesEvent::Emit(Action::LoadFileDiff(path)) => assert_eq!(path, "b.txt"),
        other => panic!("expected LoadFileDiff(b.txt), got {other:?}"),
    }
    // The dispatcher folds the emit by beginning the diff stage.
    view.load
        .begin_stage(&LoadTracker::diff_stage_key_path("b.txt"), "Loading diff for b.txt…");
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
    assert_eq!(
        view.selected_index(),
        1,
        "the selection must stay on b.txt"
    );
    assert!(
        view.is_loading(),
        "the b.txt diff stage must still be in flight"
    );

    // @step When the diff result for the second file arrives
    view.set_diff("b.txt", Some("+b".to_string()));
    assert!(view.load.complete_stage(&LoadTracker::diff_stage_key_path("b.txt")));
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

/// Scenario: Opening flips the Navigator to the Changed Files view and
/// closing returns to the board.
#[test]
fn open_flips_navigator_to_changed_files_and_close_returns_to_board() {
    // @step Given the Navigator is showing the Board view
    let (tx, _rx) = unbounded_channel();
    let mut nav = Navigator::new(Arc::new(Theme::default()), tx);
    assert_eq!(nav.active_view, ViewMode::Board);

    // @step When Action::OpenChangedFilesView is applied to the Navigator
    nav.apply_action(&Action::OpenChangedFilesView);

    // @step Then the Navigator active view is ViewMode::ChangedFiles
    assert_eq!(nav.active_view, ViewMode::ChangedFiles);

    // @step When Action::CloseChangedFilesView is applied to the Navigator
    nav.apply_action(&Action::CloseChangedFilesView);

    // @step Then the Navigator active view is ViewMode::Board
    assert_eq!(nav.active_view, ViewMode::Board);
}
