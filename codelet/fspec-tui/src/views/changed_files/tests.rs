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
use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};
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
            if leading.is_none() && sym.trim() != "" {
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
    view.set_diff(
        "a.txt",
        Some("@@ -1 +1 @@\n-oldline\n+newline".to_string()),
    );

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
    assert!(viewport_height > 0, "viewport height must be known after render");

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
    assert_eq!(view.diff_scroll(), max_scroll, "should rest at the last full page");
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
    let files_rect = view.last_files_rect.expect("files rect cached after render");

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
    let files: Vec<ChangedFile> = (0..50).map(|i| cf(&format!("file{i}.txt"), "M", false)).collect();
    view.set_files(files);

    // @step When the view is rendered
    let buf = render_buffer(&mut view, 80, 14);
    let rect = view.last_files_rect.expect("files rect cached after render");
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
    let rect = view.last_files_rect.expect("files rect cached after render");
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
