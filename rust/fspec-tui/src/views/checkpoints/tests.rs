//! RPC-364 — `CheckpointsView` three-pane component tests.
//!
//! Feature: spec/features/rust-checkpoints-view.feature
//!
//! Integration-style ratatui `TestBackend` buffer assertions over the
//! real view (no mocks): board C-key wiring, Navigator open/close flip,
//! auto/manual checkpoint label rendering, pane-focus cycling with
//! heading highlight, lazy diff coloring, pane-aware arrow keys, and the
//! empty state + Esc close.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::components::{Action, EventResult};
use crate::store::BoardStore;
use crate::theme::Theme;
use crate::views::board::BoardView;
use crate::views::navigator::{Navigator, ViewMode};
use codelet_rpc_types::{ChangedFile, CheckpointInfo};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;

fn ci(work_unit_id: &str, name: &str, is_automatic: bool) -> CheckpointInfo {
    CheckpointInfo {
        work_unit_id: work_unit_id.to_string(),
        name: name.to_string(),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        is_automatic,
    }
}

fn cf(path: &str, ct: &str) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        change_type: ct.to_string(),
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

/// Render the view and return `(joined symbols, per-cell (symbol, fg))`.
fn render_grid(view: &mut CheckpointsView, w: u16, h: u16) -> (String, Vec<(String, Color)>) {
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

/// Per-row `(text, leading-cell fg)` for diff-color assertions.
fn diff_rows(view: &mut CheckpointsView, w: u16, h: u16) -> Vec<(String, Color)> {
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

// ───────────────────────── board + navigator wiring ─────────────────────

// ───────────────────────── RPC-367: pane borders ────────────────────────
// Feature: spec/features/rust-tui-pane-borders-checkpoints.feature

/// Scenario: Checkpoints view shows a vertical divider between the
/// Checkpoints and Files panes
#[test]
fn checkpoints_view_shows_vertical_divider_between_checkpoints_and_files_panes() {
    // @step Given the Checkpoints view has at least one checkpoint to display
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![ci("AUTH-001", "baseline", false)]);
    // Load files for the selected checkpoint so the top Files pane renders too.
    view.set_files("AUTH-001", "baseline", vec![cf("a.txt", "M")]);

    // @step When the view is rendered to the terminal buffer
    let mut term = Terminal::new(TestBackend::new(80, 24)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let cp_rect = view
        .last_checkpoints_rect
        .expect("checkpoints rect cached after render");

    // @step Then a vertical divider glyph is drawn in the column between the Checkpoints pane and the Files pane
    // The divider lives in the gutter column directly to the right of the
    // Checkpoints pane content (~40% of the top-row width).
    let divider_col = cp_rect.x + cp_rect.width;
    let divider_cell = (0..buf.area.height)
        .map(|y| &buf[(divider_col, y)])
        .find(|cell| cell.symbol() == "│");
    assert!(
        divider_cell.is_some(),
        "expected a '│' divider in column {divider_col} between the Checkpoints and Files panes"
    );

    // @step And the divider uses the default terminal colour with no explicit colour set
    let divider_fg = divider_cell
        .map(|cell| cell.fg)
        .expect("divider cell present");
    assert_eq!(
        divider_fg,
        Color::Reset,
        "divider should use the default terminal colour (Color::Reset)"
    );
}

#[test]
fn pressing_c_on_the_board_emits_open_checkpoints_view() {
    // @step Given the Kanban board is focused
    let (tx, mut rx) = unbounded_channel();
    let board = BoardView::new(Arc::new(Theme::default()), tx);
    let store = BoardStore::default();

    // @step When the user presses the C key
    let result = board.handle_event(&key(KeyCode::Char('C')), &store);

    // @step Then the board emits Action::OpenCheckpointsView
    let mut saw_open = false;
    while let Ok(action) = rx.try_recv() {
        if matches!(action, Action::OpenCheckpointsView) {
            saw_open = true;
        }
    }
    assert!(saw_open, "expected Action::OpenCheckpointsView on the bus");

    // @step And the key event is consumed
    assert!(matches!(result, EventResult::Consumed(_)));
}

#[test]
fn open_flips_navigator_to_checkpoints_and_close_returns_to_board() {
    // @step Given a Navigator whose active view is the Board
    let (tx, _rx) = unbounded_channel();
    let mut nav = Navigator::new(Arc::new(Theme::default()), tx);
    assert_eq!(nav.active_view, ViewMode::Board);

    // @step When Action::OpenCheckpointsView is applied
    nav.apply_action(&Action::OpenCheckpointsView);

    // @step Then the Navigator active view is Checkpoints
    assert_eq!(nav.active_view, ViewMode::Checkpoints);

    // @step When Action::CloseCheckpointsView is applied
    nav.apply_action(&Action::CloseCheckpointsView);

    // @step Then the Navigator active view is Board
    assert_eq!(nav.active_view, ViewMode::Board);
}

// ───────────────────────── rendering: labels ────────────────────────────

#[test]
fn auto_and_manual_checkpoints_render_their_labels() {
    // @step Given a Checkpoints view listing an automatic checkpoint AUTH-001-auto-testing and a manual checkpoint baseline
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![
        ci("AUTH-001", "AUTH-001-auto-testing", true),
        ci("AUTH-001", "baseline", false),
    ]);

    // @step When the view is rendered
    let (joined, _cells) = render_grid(&mut view, 100, 20);

    // @step Then the checkpoints pane shows the row AUTH-001: Testing
    assert!(joined.contains("AUTH-001: Testing"), "joined:\n{joined}");

    // @step And the checkpoints pane shows the row baseline
    assert!(joined.contains("baseline"), "joined:\n{joined}");
}

// ───────────────────────── focus cycling ────────────────────────────────

#[test]
fn tab_moves_focus_from_checkpoints_to_files_and_highlights_heading() {
    // @step Given a Checkpoints view focused on the Checkpoints pane
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![ci("AUTH-001", "baseline", false)]);
    let _ = render_grid(&mut view, 100, 20);
    assert_eq!(view.focused_pane(), Pane::Checkpoints);

    // @step When the user presses the Tab key
    let _ = view.handle_event(&key(KeyCode::Tab));

    // @step Then the focused pane is the Files pane
    assert_eq!(view.focused_pane(), Pane::Files);

    // @step And the Files pane heading is highlighted
    // Render once into a grid carrying per-cell (symbol, fg, bg) and assert the
    // green background appears on the row that renders the "Files" heading
    // specifically (so the test cannot pass on an unrelated green cell).
    let mut term = Terminal::new(TestBackend::new(100, 20)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut files_row_has_green_bg = false;
    for y in 0..buf.area.height {
        let row: String = (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect();
        if row.contains("Files") && (0..buf.area.width).any(|x| buf[(x, y)].bg == Color::Green) {
            files_row_has_green_bg = true;
        }
    }
    assert!(
        files_row_has_green_bg,
        "the Files heading row should use a green background when focused"
    );
}

// ───────────────────────── lazy diff flow ───────────────────────────────

#[test]
fn selecting_checkpoint_then_file_shows_colored_diff() {
    // @step Given a Checkpoints view whose selected checkpoint changed a.txt
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![ci("AUTH-001", "baseline", false)]);

    // @step When the checkpoint files for a.txt are loaded
    view.set_files("AUTH-001", "baseline", vec![cf("a.txt", "M")]);

    // @step And the unified diff for a.txt is loaded
    view.set_diff(
        "AUTH-001",
        "baseline",
        "a.txt",
        Some("@@ -1 +1 @@\n-old\n+new".to_string()),
    );

    // @step When the view is rendered
    let rows = diff_rows(&mut view, 100, 24);

    // @step Then the diff pane shows the added line in green
    assert_eq!(row_color_for(&rows, "new"), Some(Color::Green));
}

// ───────────────────────── pane-aware arrow keys ────────────────────────

#[test]
fn arrow_keys_act_on_the_focused_pane() {
    // @step Given a Checkpoints view with the Diff pane focused and a long diff
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![ci("AUTH-001", "baseline", false)]);
    view.set_files("AUTH-001", "baseline", vec![cf("a.txt", "M")]);
    let long_diff = (0..200)
        .map(|i| format!(" context line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    view.set_diff("AUTH-001", "baseline", "a.txt", Some(long_diff));
    let _ = render_grid(&mut view, 100, 24);
    let _ = view.handle_event(&key(KeyCode::Tab)); // -> Files
    let _ = view.handle_event(&key(KeyCode::Tab)); // -> Diff
    assert_eq!(view.focused_pane(), Pane::Diff);
    let before_scroll = view.diff_scroll();
    let before_sel = view.selected_checkpoint();

    // @step When the user presses the Down key
    let _ = view.handle_event(&key(KeyCode::Down));

    // @step Then the diff pane scroll offset increases and the checkpoint selection is unchanged
    assert!(view.diff_scroll() > before_scroll);
    assert_eq!(view.selected_checkpoint(), before_sel);

    // @step Given a Checkpoints view with the Checkpoints pane focused and two checkpoints
    let mut view2 = CheckpointsView::new();
    view2.set_checkpoints(vec![
        ci("AUTH-001", "baseline", false),
        ci("AUTH-002", "second", false),
    ]);
    let _ = render_grid(&mut view2, 100, 24);
    assert_eq!(view2.focused_pane(), Pane::Checkpoints);

    // @step When the user presses the Down key
    let _ = view2.handle_event(&key(KeyCode::Down));

    // @step Then the selected checkpoint index becomes 1
    assert_eq!(view2.selected_checkpoint(), 1);
}

// ───────────────────────── empty state + esc ────────────────────────────

#[test]
fn empty_repo_shows_message_and_esc_closes() {
    // @step Given a Checkpoints view with no checkpoints
    let mut view = CheckpointsView::new();
    view.set_checkpoints(Vec::new());

    // @step When the view is rendered
    let (joined, _cells) = render_grid(&mut view, 100, 20);

    // @step Then the view shows the No checkpoints available message
    assert!(
        joined.contains("No checkpoints available"),
        "joined:\n{joined}"
    );

    // @step When the user presses the Esc key
    let outcome = view.handle_event(&key(KeyCode::Esc));

    // @step Then the view emits Action::CloseCheckpointsView
    assert!(matches!(outcome, CheckpointsEvent::Close));
}

// ───────────────────────── RPC-369: click to select ─────────────────────
// Feature: spec/features/checkpoints-view-click-to-select.feature

/// Dispatch a synthetic left mouse Down event at the given screen cell.
fn left_click(view: &mut CheckpointsView, column: u16, row: u16) -> CheckpointsEvent {
    view.handle_event(&Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }))
}

/// A view with two checkpoints (first selected) and files loaded for it,
/// rendered once so the pane content rects are cached.
fn view_with_two_checkpoints() -> CheckpointsView {
    let mut view = CheckpointsView::new();
    view.set_checkpoints(vec![
        ci("AUTH-001", "baseline", false),
        ci("AUTH-002", "second", false),
    ]);
    view.set_files(
        "AUTH-001",
        "baseline",
        vec![cf("a.txt", "M"), cf("b.txt", "A")],
    );
    let _ = render_grid(&mut view, 100, 24);
    view
}

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
        CheckpointsEvent::Emit(Action::LoadCheckpointFiles { work_unit_id, name }) => {
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
        CheckpointsEvent::Emit(Action::LoadCheckpointFileDiff { path, .. }) => {
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
