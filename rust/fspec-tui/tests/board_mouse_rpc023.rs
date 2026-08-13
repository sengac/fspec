//! RPC-023 — Mouse hit-test + wheel + click integration for BoardView.
//!
//! Feature: spec/features/boardview-mouse-handling.feature
//!
//! Drives `BoardView::handle_event` against `Event::Mouse` variants to
//! assert wheel-up/down emit Action::SelectPrev/Next, horizontal wheel
//! emits FocusPrev/NextColumn, header clicks emit SetFocusedColumn, and
//! content-row clicks emit SetFocusedColumn + SelectIndexInFocused with
//! the scroll_offset added in.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, BoardStore, BoardView, Theme, COLUMN_ORDER};
use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

fn wu(id: &str, status: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

fn fresh() -> (BoardView, UnboundedReceiver<Action>) {
    let (tx, rx) = unbounded_channel();
    let view = BoardView::new(Arc::new(Theme::default()), tx);
    (view, rx)
}

fn render(view: &BoardView, store: &BoardStore) {
    let mut term = Terminal::new(TestBackend::new(120, 30)).expect("Terminal::new");
    term.draw(|frame| {
        view.render_with_store(frame.area(), frame.buffer_mut(), store);
    })
    .expect("draw");
}

fn synth_mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// Scenario: Wheel-down inside the BACKLOG content area emits SelectNext
#[tokio::test]
async fn wheel_down_inside_the_backlog_content_area_emits_select_next() {
    // @step Given the BoardStore is seeded with 20 story work units in the BACKLOG column
    let mut store = BoardStore::default();
    let units: Vec<WorkUnitInfo> = (0..20)
        .map(|i| wu(&format!("AUTH-{i:03}"), "backlog"))
        .collect();
    store.replace_work_units(units);
    // @step And the focused column is BACKLOG and selected_index is 0
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 0);
    let (view, mut rx) = fresh();
    // @step And BoardView has been rendered onto a 120x30 TestBackend so last_content_area is populated
    render(&view, &store);

    // @step When an Event::Mouse(ScrollDown) arrives with the cursor inside the column-content area
    let event = synth_mouse(MouseEventKind::ScrollDown, 5, 15);
    let result = view.handle_event(&event, &store);

    // @step Then BoardView::handle_event returns EventResult::Consumed
    assert!(result.is_consumed());
    // @step And Action::SelectNext is emitted onto the action bus
    let action = rx.try_recv().expect("Action::SelectNext expected");
    assert!(matches!(action, Action::SelectNext));
    // @step And dispatching that action through App::dispatch advances BoardStore.selected_index_for("backlog") to 1
    let vh = view.last_viewport_height();
    store.move_selection(1, vh);
    assert_eq!(store.selected_index_for("backlog"), 1);
}

/// Scenario: Wheel-down at the last unit wraps the selection to index 0
#[tokio::test]
async fn wheel_down_at_the_last_unit_wraps_the_selection_to_index_0() {
    // @step Given the BoardStore is seeded with 20 story work units in the BACKLOG column
    let mut store = BoardStore::default();
    let units: Vec<WorkUnitInfo> = (0..20)
        .map(|i| wu(&format!("AUTH-{i:03}"), "backlog"))
        .collect();
    store.replace_work_units(units);
    // @step And the focused column is BACKLOG and selected_index is 19
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 19);
    let (view, mut rx) = fresh();
    // @step And BoardView has been rendered onto a 120x30 TestBackend so last_content_area is populated
    render(&view, &store);

    // @step When an Event::Mouse(ScrollDown) arrives with the cursor inside the column-content area
    let event = synth_mouse(MouseEventKind::ScrollDown, 5, 15);
    let _ = view.handle_event(&event, &store);
    // @step And the resulting Action::SelectNext is dispatched through App::dispatch
    let action = rx.try_recv().expect("Action expected");
    assert!(matches!(action, Action::SelectNext));
    let vh = view.last_viewport_height();
    store.move_selection(1, vh);

    // @step Then BoardStore.selected_index_for("backlog") wraps back to 0
    assert_eq!(store.selected_index_for("backlog"), 0);
}

/// Scenario: Wheel-up inside the BACKLOG content area emits SelectPrev
#[tokio::test]
async fn wheel_up_inside_the_backlog_content_area_emits_select_prev() {
    // @step Given the BoardStore is seeded with 20 story work units in the BACKLOG column
    let mut store = BoardStore::default();
    let units: Vec<WorkUnitInfo> = (0..20)
        .map(|i| wu(&format!("AUTH-{i:03}"), "backlog"))
        .collect();
    store.replace_work_units(units);
    // @step And the focused column is BACKLOG and selected_index is 5
    store.set_focused_column("backlog");
    store.set_selected_index_for("backlog", 5);
    let (view, mut rx) = fresh();
    // @step And BoardView has been rendered onto a 120x30 TestBackend so last_content_area is populated
    render(&view, &store);

    // @step When an Event::Mouse(ScrollUp) arrives with the cursor inside the column-content area
    let event = synth_mouse(MouseEventKind::ScrollUp, 5, 15);
    let result = view.handle_event(&event, &store);

    // @step Then BoardView::handle_event returns EventResult::Consumed
    assert!(result.is_consumed());
    // @step And Action::SelectPrev is emitted onto the action bus
    let action = rx.try_recv().expect("Action::SelectPrev expected");
    assert!(matches!(action, Action::SelectPrev));
}

/// Scenario: Wheel event outside the content area is Ignored
#[tokio::test]
async fn wheel_event_outside_the_content_area_is_ignored() {
    // @step Given the BoardStore is seeded with 20 story work units in the BACKLOG column
    let mut store = BoardStore::default();
    let units: Vec<WorkUnitInfo> = (0..20)
        .map(|i| wu(&format!("AUTH-{i:03}"), "backlog"))
        .collect();
    store.replace_work_units(units);
    let (view, mut rx) = fresh();
    // @step And BoardView has been rendered onto a 120x30 TestBackend so last_content_area is populated
    render(&view, &store);

    // @step When an Event::Mouse(ScrollDown) arrives at row 0 which lies on the top border
    let event = synth_mouse(MouseEventKind::ScrollDown, 5, 0);
    let result = view.handle_event(&event, &store);

    // @step Then BoardView::handle_event returns EventResult::Ignored
    assert!(!result.is_consumed());
    // @step And no Action is emitted onto the action bus
    assert!(rx.try_recv().is_err());
}

/// Scenario: Wheel-right inside the content area emits FocusNextColumn
#[tokio::test]
async fn wheel_right_inside_the_content_area_emits_focus_next_column() {
    // @step Given the BoardStore is seeded with work units across columns
    let mut store = BoardStore::default();
    store.replace_work_units(vec![
        wu("AUTH-001", "backlog"),
        wu("AUTH-002", "specifying"),
    ]);
    // @step And the focused column is BACKLOG
    store.set_focused_column("backlog");
    let (view, mut rx) = fresh();
    // @step And BoardView has been rendered onto a 120x30 TestBackend so last_content_area is populated
    render(&view, &store);

    // @step When an Event::Mouse(ScrollRight) arrives with the cursor inside the column-content area
    let event = synth_mouse(MouseEventKind::ScrollRight, 5, 15);
    let result = view.handle_event(&event, &store);

    // @step Then BoardView::handle_event returns EventResult::Consumed
    assert!(result.is_consumed());
    // @step And Action::FocusNextColumn is emitted onto the action bus
    let action = rx.try_recv().expect("FocusNextColumn expected");
    assert!(matches!(action, Action::FocusNextColumn));
}

/// Scenario: Wheel-left inside the content area emits FocusPrevColumn
#[tokio::test]
async fn wheel_left_inside_the_content_area_emits_focus_prev_column() {
    // @step Given the BoardStore is seeded with work units across columns
    let mut store = BoardStore::default();
    store.replace_work_units(vec![
        wu("AUTH-001", "implementing"),
        wu("AUTH-002", "testing"),
    ]);
    // @step And the focused column is IMPLEMENTING
    store.set_focused_column("implementing");
    let (view, mut rx) = fresh();
    // @step And BoardView has been rendered onto a 120x30 TestBackend so last_content_area is populated
    render(&view, &store);

    // @step When an Event::Mouse(ScrollLeft) arrives with the cursor inside the column-content area
    let event = synth_mouse(MouseEventKind::ScrollLeft, 55, 15);
    let result = view.handle_event(&event, &store);

    // @step Then BoardView::handle_event returns EventResult::Consumed
    assert!(result.is_consumed());
    // @step And Action::FocusPrevColumn is emitted onto the action bus
    let action = rx.try_recv().expect("FocusPrevColumn expected");
    assert!(matches!(action, Action::FocusPrevColumn));
}

/// Scenario: Left-click on a column header emits SetFocusedColumn
#[tokio::test]
async fn left_click_on_a_column_header_emits_set_focused_column() {
    // @step Given the BoardStore is seeded with work units across columns
    let mut store = BoardStore::default();
    store.replace_work_units(vec![
        wu("AUTH-001", "backlog"),
        wu("AUTH-002", "specifying"),
    ]);
    // @step And the focused column is BACKLOG
    store.set_focused_column("backlog");
    let (view, mut rx) = fresh();
    // @step And BoardView has been rendered onto a 120x30 TestBackend so last_column_header_areas is populated
    render(&view, &store);

    // @step When an Event::Mouse(Down(Left)) arrives with the cursor inside the SPECIFYING column header rect
    // 120-wide terminal: column 0 (BACKLOG) covers x [1,17), col 1 (SPECIFYING) covers x [18,34).
    // Column header row sits at y=12 for a 30-row terminal layout (RPC-014/015 grid).
    let event = synth_mouse(MouseEventKind::Down(MouseButton::Left), 25, 12);
    let result = view.handle_event(&event, &store);

    // @step Then BoardView::handle_event returns EventResult::Consumed
    assert!(result.is_consumed());
    // @step And Action::SetFocusedColumn(1) is emitted onto the action bus
    let action = rx.try_recv().expect("SetFocusedColumn expected");
    let idx = match action {
        Action::SetFocusedColumn(i) => i,
        other => panic!("expected SetFocusedColumn, got {other:?}"),
    };
    assert_eq!(idx, 1, "expected column index 1 (specifying)");
    // @step And dispatching that action through App::dispatch sets BoardStore.focused_column_index() to 1
    store.set_focused_column(COLUMN_ORDER[idx]);
    assert_eq!(store.focused_column_index(), 1);
}

/// Scenario: Left-click on a content row emits SetFocusedColumn and SelectIndexInFocused
#[tokio::test]
async fn left_click_on_a_content_row_emits_set_focused_column_and_select_index() {
    // @step Given the BoardStore is seeded with five story work units in the DONE column
    let mut store = BoardStore::default();
    store.replace_work_units(
        (0..5)
            .map(|i| wu(&format!("DONE-{i:03}"), "done"))
            .collect::<Vec<_>>(),
    );
    // @step And the focused column is BACKLOG and DONE has scroll_offset 0
    store.set_focused_column("backlog");
    store.set_scroll_offset_for("done", 0);
    let (view, mut rx) = fresh();
    // @step And BoardView has been rendered onto a 120x30 TestBackend so last_column_content_areas is populated
    render(&view, &store);

    // @step When an Event::Mouse(Down(Left)) arrives with the cursor on visible row 2 of the DONE column content area
    // DONE is col 5 — x band [86, 102); content area starts at y=14 → visible row 2 → y=16.
    let event = synth_mouse(MouseEventKind::Down(MouseButton::Left), 90, 16);
    let result = view.handle_event(&event, &store);

    // @step Then BoardView::handle_event returns EventResult::Consumed
    assert!(result.is_consumed());
    // @step And Action::SetFocusedColumn(5) is emitted onto the action bus
    let a1 = rx.try_recv().expect("SetFocusedColumn expected");
    let col_idx = match a1 {
        Action::SetFocusedColumn(i) => i,
        other => panic!("expected SetFocusedColumn, got {other:?}"),
    };
    assert_eq!(col_idx, 5);
    // @step And Action::SelectIndexInFocused(2) is emitted onto the action bus
    let a2 = rx.try_recv().expect("SelectIndexInFocused expected");
    let row_idx = match a2 {
        Action::SelectIndexInFocused(i) => i,
        other => panic!("expected SelectIndexInFocused, got {other:?}"),
    };
    assert_eq!(row_idx, 2);
    // @step And dispatching those actions through App::dispatch leaves BoardStore.focused_column_index() at 5 and BoardStore.selected_index_for("done") at 2
    store.set_focused_column(COLUMN_ORDER[col_idx]);
    let vh = view.last_viewport_height();
    store.select_index_in_focused(row_idx, vh);
    assert_eq!(store.focused_column_index(), 5);
    assert_eq!(store.selected_index_for("done"), 2);
}

/// Scenario: Click on a content row adds scroll_offset to the clicked row index
#[tokio::test]
async fn click_on_a_content_row_adds_scroll_offset_to_the_clicked_row_index() {
    // @step Given the BoardStore is seeded with 20 story work units in the BACKLOG column
    let mut store = BoardStore::default();
    let units: Vec<WorkUnitInfo> = (0..20)
        .map(|i| wu(&format!("AUTH-{i:03}"), "backlog"))
        .collect();
    store.replace_work_units(units);
    // @step And the BACKLOG scroll_offset is 5
    store.set_scroll_offset_for("backlog", 5);
    let (view, mut rx) = fresh();
    // @step And BoardView has been rendered onto a 120x30 TestBackend so last_column_content_areas is populated
    render(&view, &store);

    // @step When an Event::Mouse(Down(Left)) arrives with the cursor on visible row 1 of the BACKLOG column content area
    // BACKLOG is col 0 — x band [1,17); content starts at y=14 → visible row 1 → y=15.
    let event = synth_mouse(MouseEventKind::Down(MouseButton::Left), 5, 15);
    let _ = view.handle_event(&event, &store);

    // Drain the SetFocusedColumn first.
    let _ = rx.try_recv().expect("SetFocusedColumn expected");
    // @step Then Action::SelectIndexInFocused(6) is emitted onto the action bus
    let a2 = rx.try_recv().expect("SelectIndexInFocused expected");
    let row_idx = match a2 {
        Action::SelectIndexInFocused(i) => i,
        other => panic!("expected SelectIndexInFocused, got {other:?}"),
    };
    assert_eq!(row_idx, 6, "visible row 1 + scroll_offset 5 = 6");
}
