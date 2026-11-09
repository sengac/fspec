//! Feature: spec/features/menu-dropdown-cycles-with-the-board-ring-walk.feature
//!
//! Moved out of menu002_board_surface.rs (COV-075: one test file per feature).

use crate::common::TaskOutcome;
use codelet_fspec_tui::components::menu_bar::MenuFocus;
use codelet_fspec_tui::{App, FspecBackend};
use codelet_rpc_types::{SessionId, WorkUnitInfo};
use codelet_test_helpers::{OrFail, TestResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;
use ratatui::Terminal;
use std::sync::Arc;
mod common;
use common::MockBackend;

/// The Kanban item's Zone A rect x (header right column, bar row).
const KANBAN_X: u16 = 15;

// ─────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

fn mouse(kind: MouseEventKind, col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// Render the full App (navigator + compositor) into a 120x24 buffer.
fn render_app(app: &mut App) -> Result<Buffer, codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(120, 24)).or_fail("Terminal::new")?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    Ok(term.backend().buffer().clone())
}

fn row_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// The header's row 3 — the 2-zone menu bar's row (3 rows below the
/// checkpoint-status row, whose leading text is `Checkpoints:`).
fn bar_row(buf: &Buffer) -> Result<u16, codelet_test_helpers::TestError> {
    let status_y = (0..buf.area.height)
        .find(|&y| row_text(buf, y).contains("Checkpoints:"))
        .or_fail("the checkpoint status row must render")?;
    Ok(status_y + 3)
}

/// True when the cell carries the inverse-video highlight (bg Cyan).
fn is_inverse(buf: &Buffer, x: u16, y: u16) -> bool {
    buf[(x, y)].bg == Color::Cyan
}

fn make_unit(id: &str, status: &str) -> WorkUnitInfo {
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

/// Seed `n` open sessions (s-1..s-n, Idle by default).
async fn seed_sessions(app: &mut App, n: usize) -> Result<(), codelet_test_helpers::TestError> {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
            format!("s-{i}"),
        )));
    }
    drain_pending(app).await?;
    Ok(())
}

/// Seed a small board so the focused column has a selectable card
/// (AUTH-001 in backlog, one in testing, one in blocked).
async fn seed_units(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    app.dispatch(codelet_fspec_tui::Action::WorkUnitsLoaded(vec![
        make_unit("AUTH-001", "backlog"),
        make_unit("AUTH-002", "testing"),
        make_unit("AUTH-003", "blocked"),
    ]));
    drain_pending(app).await?;
    Ok(())
}

/// Focus the named column via the existing action bus.
fn focus_column(app: &mut App, name: &str) {
    app.board_store_mut().set_focused_column(name);
}

/// Arrange: the 2-zone bar is focused on the FIRST menu item (the
/// dropdown closed): focus the last column and step Right once.
async fn focus_first_item(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    focus_column(app, "blocked");
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(app).await?;
    Ok(())
}

/// Arrange: open the `item_index`-th menu item's dropdown with the
/// cursor on `cursor_row` (0 = Kanban, 1 = Tools, 2 = Settings,
/// 3 = Help — MENU-008 four Zone A items).
async fn open_category_dropdown_at(
    app: &mut App,
    item_index: usize,
    cursor_row: usize,
) -> Result<(), codelet_test_helpers::TestError> {
    focus_first_item(app).await?;
    for _ in 1..=item_index {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    drain_pending(app).await?;
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(app).await?;
    for _ in 0..cursor_row {
        app.handle_event(&key(KeyCode::Down, KeyModifiers::NONE));
    }
    drain_pending(app).await?;
    Ok(())
}

/// Arrange: open the Kanban dropdown (item 0) with the cursor on
/// `cursor_row`.
async fn open_kanban_dropdown_at(
    app: &mut App,
    cursor_row: usize,
) -> Result<(), codelet_test_helpers::TestError> {
    open_category_dropdown_at(app, 0, cursor_row).await?;
    Ok(())
}

/// Arrange: open the Help dropdown (item 3) with the cursor on
/// `cursor_row`.
async fn open_help_dropdown_at(
    app: &mut App,
    cursor_row: usize,
) -> Result<(), codelet_test_helpers::TestError> {
    open_category_dropdown_at(app, 3, cursor_row).await?;
    Ok(())
}

/// The y of the dropdown row whose text contains `needle` (the panel
/// anchors at bar_row + 1; rows sit just below the bar row).
fn dropdown_row_y(buf: &Buffer, needle: &str) -> Result<u16, codelet_test_helpers::TestError> {
    let bar = bar_row(buf)?;
    Ok(
        // Start at bar + 2: the panel's border row (bar + 1) and the bar's
        // OWN row (which can contain the item label, e.g. "Help") never
        // carry entry rows — the first entry row sits at bar + 2.
        (bar + 2..buf.area.height)
            .find(|&y| row_text(buf, y).contains(needle))
            .or_fail(&format!(
                "dropdown row '{needle}' not found below row {bar}"
            ))?,
    )
}

// ─────────────────────────────────────────────────────────────────────────
// BUG-198 — the open dropdown must follow the Left/Right ring walk
// (feature: spec/features/menu-dropdown-cycles-with-the-board-ring-walk.feature)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: While a dropdown is open Left and Right walk the ring and re-anchor
#[tokio::test]
async fn scenario_while_a_dropdown_is_open_left_and_right_walk_the_ring_and_reanchor() -> TestResult
{
    // @step Given the Kanban dropdown is open with the cursor on row 1 (Search)
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    open_kanban_dropdown_at(&mut app, 1).await?;
    assert_eq!(
        app.board_store().open_menu(),
        Some((0, 1)),
        "the Kanban dropdown must be open with the cursor on row 1"
    );
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(0)),
        "the Kanban item must hold the ring focus"
    );

    // @step When I press Right once and then Left once
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await?;

    // @step Then the dropdown re-anchors under the focused item at row 0 each time and the panel paints under the new item
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(0)),
        "Left must walk back onto the Kanban item"
    );
    assert_eq!(
        app.board_store().open_menu(),
        Some((0, 0)),
        "the re-anchored dropdown must sit at row 0 of Kanban"
    );
    let buf = render_app(&mut app)?;
    let search_y = dropdown_row_y(&buf, "New Agent")?;
    assert!(
        row_text(&buf, search_y).contains("New Agent"),
        "the Kanban panel must be painted under the re-anchored item:\n{}",
        row_text(&buf, search_y)
    );
    Ok(())
}

/// Scenario: While a dropdown is open Left walks the ring back onto the previous item
#[tokio::test]
async fn scenario_left_with_an_open_dropdown_moves_the_panel_back_onto_the_previous_item(
) -> TestResult {
    // @step Given the Kanban dropdown is open and the ring has walked onto the Tools item
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    open_kanban_dropdown_at(&mut app, 2).await?;
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    assert_eq!(
        app.board_store().open_menu(),
        Some((1, 0)),
        "Right must have re-anchored the open panel under Tools at row 0"
    );

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await?;

    // @step Then the Tools item loses the focus and the dropdown re-anchors under the Kanban item at row 0
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(0)),
        "Left must walk back onto the Kanban item"
    );
    assert_eq!(
        app.board_store().open_menu(),
        Some((0, 0)),
        "the open panel must follow the walk back under Kanban at row 0"
    );
    let buf = render_app(&mut app)?;
    let new_agent_y = dropdown_row_y(&buf, "New Agent")?;
    assert!(
        is_inverse(&buf, KANBAN_X + 1, new_agent_y),
        "the Kanban panel must be painted with the cursor re-armed at row 0"
    );
    Ok(())
}

/// Scenario: While a dropdown is open Left from the first item walks into the columns and closes it
#[tokio::test]
async fn scenario_left_from_the_first_item_with_an_open_dropdown_closes_the_panel() -> TestResult {
    // @step Given the Kanban dropdown is open with the ring on the first item
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    open_kanban_dropdown_at(&mut app, 0).await?;
    let buf = render_app(&mut app)?;
    assert!(
        row_text(&buf, bar_row(&buf)? + 2).contains("New Agent"),
        "the Kanban panel must be open before the walk"
    );

    // @step When I press Left once
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(&mut app).await?;

    // @step Then the focused column is re-focused, the bar highlight clears and the dropdown closes
    assert!(
        app.board_store().menu_focus().is_none(),
        "a column landing must drop the bar focus"
    );
    assert_eq!(
        app.board_store().focused_column_index(),
        6,
        "Left from the first item must land on the last column (blocked)"
    );
    assert_eq!(
        app.board_store().open_menu(),
        None,
        "walking off the items must close the open dropdown"
    );
    let buf = render_app(&mut app)?;
    assert!(
        !row_text(&buf, bar_row(&buf)? + 2).contains("New Agent"),
        "the panel must no longer be painted"
    );
    Ok(())
}

/// Scenario: While a dropdown is open Right from the last item walks onto a chip and closes it
#[tokio::test]
async fn scenario_right_from_the_last_item_with_an_open_dropdown_closes_the_panel() -> TestResult {
    // @step Given the Help dropdown is open and the board has 1 open session
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    open_help_dropdown_at(&mut app, 0).await?;
    assert_eq!(
        app.board_store().open_menu(),
        Some((3, 0)),
        "the Help dropdown must be open at row 0"
    );

    // @step When I press Right once
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    drain_pending(&mut app).await?;

    // @step Then the first chip has the focus and the dropdown closes
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::ZoneB(0)),
        "Right from the last item must land on chip #1"
    );
    assert_eq!(
        app.board_store().open_menu(),
        None,
        "a Zone B landing must close the open dropdown"
    );
    Ok(())
}

/// Scenario: Wheel left and right over the bar with an open dropdown re-anchors the panel
#[tokio::test]
async fn scenario_wheel_over_the_bar_with_an_open_dropdown_reanchors_the_panel() -> TestResult {
    // @step Given the Kanban dropdown is open and the board has 1 open session
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    open_kanban_dropdown_at(&mut app, 1).await?;
    assert_eq!(app.board_store().open_menu(), Some((0, 1)));
    let bar = bar_row(&render_app(&mut app)?)?;

    // @step When I scroll the wheel right once over the bar row
    app.handle_event(&mouse(MouseEventKind::ScrollRight, 20, bar));
    drain_pending(&mut app).await?;

    // @step Then the Tools item has the focus and the open dropdown re-anchors under it at row 0
    assert_eq!(
        app.board_store().menu_focus(),
        Some(MenuFocus::Item(1)),
        "wheel right must walk the ring onto Tools"
    );
    assert_eq!(
        app.board_store().open_menu(),
        Some((1, 0)),
        "the open panel must follow the wheel walk, re-anchored at row 0"
    );
    Ok(())
}
