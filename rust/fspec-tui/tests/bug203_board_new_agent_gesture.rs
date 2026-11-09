//! Feature: spec/features/board-new-agent-gesture-prompts-create-session-dialog.feature
//!
//! Moved out of menu005_menu_bar_help.rs (COV-075: one test file per feature).

use crate::common::TaskOutcome;
use codelet_fspec_tui::{
    App, FspecBackend, MuxConfig, MuxLayout, MuxPaneKind, ViewMode, CREATE_SESSION_DIALOG_ID,
};
use codelet_rpc_types::{SessionId, WorkUnitInfo};
use codelet_test_helpers::{OrFail, TestResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use std::sync::Arc;
mod common;
use common::MockBackend;

/// The stable compositor id for the menu-bar help dialog (BOARD-023 id,
/// kept stable this release — MENU-005 R2).
const MENU_BAR_HELP_DIALOG_ID: &str = "board-actions-dialog";

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

fn render_app_at(
    app: &mut App,
    width: u16,
    height: u16,
) -> Result<Buffer, codelet_test_helpers::TestError> {
    let mut term = Terminal::new(TestBackend::new(width, height)).or_fail("Terminal::new")?;
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .or_fail("draw")?;
    Ok(term.backend().buffer().clone())
}

fn row_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol())
        .collect::<String>()
}

/// Seed `n` open sessions (s-1..s-n, Idle by default) on the App.
async fn seed_sessions(app: &mut App, n: usize) -> Result<(), codelet_test_helpers::TestError> {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(SessionId::new(
            format!("s-{i}"),
        )));
    }
    drain_pending(app).await?;
    Ok(())
}

fn make_unit(id: &str, status: &str, attachments: Vec<&str>) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: status.to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: attachments.into_iter().map(String::from).collect(),
        last_state_change_at: None,
    }
}

/// Seed a small board so the board surface has content.
async fn seed_units(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    app.dispatch(codelet_fspec_tui::Action::WorkUnitsLoaded(vec![
        make_unit("AUTH-001", "backlog", vec![]),
        make_unit("AUTH-002", "testing", vec![]),
        make_unit("AUTH-003", "blocked", vec![]),
    ]));
    drain_pending(app).await?;
    Ok(())
}

fn sid(id: &str) -> SessionId {
    SessionId::new(id)
}

/// The cell x of the first occurrence of `needle` on row `y`.
fn find_x(buf: &Buffer, y: u16, needle: &str) -> Option<u16> {
    let row = row_text(buf, y);
    let byte_idx = row.find(needle)?;
    Some(row[..byte_idx].chars().count() as u16)
}

/// Enter mux mode with the given pane kinds (the `menu004_mux_surface.rs`
/// `enable_mux` helper — MuxConfig + `enable_with_config`, lazy-pane
/// initial loads for Files/Checkpoints only when rendered).
fn enable_mux(app: &mut App, panes: Vec<MuxPaneKind>) {
    let Some(layout) = MuxLayout::columns(&panes) else {
        return;
    };
    let config = MuxConfig {
        layout,
        focused_pane: 0,
        enabled: true,
    };
    let nav = app.navigator_mut();
    nav.mux.enable_with_config(config, ViewMode::Board);
    nav.active_view = ViewMode::Mux;
    // BUG-182 R7 parity: initial loads for rendered lazy panes (none for
    // Board | Agent grids).
}

// ─────────────────────────────────────────────────────────────────────────
// BUG-203 — the board 'New Agent' gesture always prompts the
// CreateSessionDialog (board-new-agent-gesture-prompts-create-session-dialog.feature)
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn scenario_bug203_board_period_key_with_attached_session_mounts_dialog_not_jump(
) -> TestResult {
    // @step Given the board has one open session "s-1" attached to work unit "AUTH-001"
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    app.board_store_mut()
        .attach_session("AUTH-001", SessionId::new("s-1"));
    // @step And "AUTH-001" is selected in the focused column
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);

    // @step When the user presses the '.' key
    let _ = app.handle_event(&key(KeyCode::Char('.'), KeyModifiers::NONE));
    drain_pending(&mut app).await?;

    // @step Then the CreateSessionDialog mounts over the board exactly once
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "'.' must mount the CreateSessionDialog (BUG-203); layers={:?}",
        app.compositor().layer_ids()
    );
    // @step And the active view is still the board (the dialog overlays; the view switch is deferred to confirm)
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "the view must stay Board until the dialog is confirmed"
    );
    // @step And no existing session is focused or switched (the navigation target is not set to "s-1")
    assert!(
        app.agent_view_store().navigation_target_session().is_none(),
        "'.' must never set a navigation target (no resume/attach — BUG-203)"
    );
    Ok(())
}

#[tokio::test]
async fn scenario_bug203_board_period_key_with_unattached_open_session_does_not_resume(
) -> TestResult {
    // @step Given the board has one open session "s-1" attached to no work unit
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    // @step And the selected work unit "AUTH-002" has no attached session
    app.board_store_mut().set_focused_column("testing");
    app.board_store_mut().set_selected_index_for("testing", 0);

    // @step When the user presses the '.' key
    let _ = app.handle_event(&key(KeyCode::Char('.'), KeyModifiers::NONE));
    drain_pending(&mut app).await?;

    // @step Then the CreateSessionDialog mounts over the board
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "'.' on an unattached unit must mount the CreateSessionDialog; layers={:?}",
        app.compositor().layer_ids()
    );
    // @step And the first open session "s-1" is NOT resumed (the active view is still the board)
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "no silent resume (BUG-203)"
    );
    Ok(())
}

#[tokio::test]
async fn scenario_bug203_board_zone_c_button_with_running_agent_mounts_dialog() -> TestResult {
    // @step Given an agent is running on session "s-1"
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    app.board_store_mut().attach_session("AUTH-001", sid("s-1"));
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);
    // @step And the board bar's '[ New Agent ]' Zone C button is painted
    let buf = render_app_at(&mut app, 120, 24)?;
    let y = (0..buf.area.height)
        .find(|&yy| row_text(&buf, yy).contains("[ New Agent ]"))
        .or_fail("the '[ New Agent ]' Zone C button on the bar row")?;
    let x = find_x(&buf, y, "[ New Agent ]").or_fail("New Agent x")?;

    // @step When I left-click the '[ New Agent ]' button
    app.handle_event(&Event::Mouse(crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    }));
    drain_pending(&mut app).await?;

    // @step Then the CreateSessionDialog mounts over the board instead of switching into the running agent "s-1"
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "the button must mount the CreateSessionDialog (BUG-199 parity); layers={:?}",
        app.compositor().layer_ids()
    );
    // @step And the current session remains "s-1" (untouched)
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("s-1")),
        "the running session must stay current"
    );
    // @step And the board bar's ring focus clears
    assert!(
        app.board_store().menu_focus().is_none(),
        "executing the button must clear the bar's ring focus"
    );
    Ok(())
}

#[tokio::test]
async fn scenario_bug203_confirm_yes_from_board_flips_to_agent_without_binding_unit() -> TestResult
{
    // @step Given the board has one open session "s-1" attached to no work unit
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    // @step And the selected work unit "AUTH-002" has no attached session
    app.board_store_mut().set_focused_column("testing");
    app.board_store_mut().set_selected_index_for("testing", 0);

    // @step When the user presses the '.' key
    let _ = app.handle_event(&key(KeyCode::Char('.'), KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    assert!(app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    // @step And the user presses Enter on the "Start" option
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await?;

    // @step Then a fresh session is created (the backend create_session fires exactly once)
    // (the MockBackend defaults create_session to "s-mock-default" — the
    // fresh session lands in the store after the drain)
    assert!(
        app.agent_view_store()
            .current_session()
            .is_some_and(|s| s != &SessionId::new("s-1")),
        "a fresh session must be current after confirm; got {:?}",
        app.agent_view_store().current_session()
    );
    // @step And the active view is the Agent view on the fresh session
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "confirming Yes from the board must flip to the Agent view"
    );
    // @step And the fresh session is NOT bound to work unit "AUTH-002" (no attachment recorded; the store's current work unit is untouched)
    assert!(
        app.board_store().session_for("AUTH-002").is_none(),
        "the fresh session must NOT bind to the selected work unit (R3)"
    );
    Ok(())
}

#[tokio::test]
async fn scenario_bug203_cancel_from_board_prompt_keeps_board_and_sessions() -> TestResult {
    // @step Given the board has one open session "s-1"
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    // @step And the selected work unit "AUTH-001" has no attached session
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);

    // @step When the user presses the '.' key
    let _ = app.handle_event(&key(KeyCode::Char('.'), KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    assert!(app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    // @step And the user presses Esc on the dialog
    let _ = app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await?;

    // @step Then the CreateSessionDialog is dismissed
    assert!(!app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    // @step And the active view is still the board
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step And the current session remains "s-1"
    assert_eq!(
        app.current_session(),
        Some(SessionId::new("s-1")),
        "Esc must leave every open session untouched (RPC-097 reopen #1)"
    );
    Ok(())
}

#[tokio::test]
async fn scenario_bug203_shift_right_from_board_still_resumes_first_open_session() -> TestResult {
    // @step Given the board has one open session "s-1" attached to no work unit
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    // @step And the selected work unit "AUTH-002" has no attached session
    app.board_store_mut().set_focused_column("testing");
    app.board_store_mut().set_selected_index_for("testing", 0);

    // @step When the user presses Shift+Right
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::SHIFT));
    drain_pending(&mut app).await?;

    // @step Then the active view is the Agent view on session "s-1" (resumed)
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "Shift+Right is the CYCLE gesture — it must resume the open session (R2)"
    );
    assert!(
        app.agent_view_store()
            .current_session()
            .is_some_and(|s| s == &SessionId::new("s-1")),
        "the resumed session must be s-1; got {:?}",
        app.agent_view_store().current_session()
    );
    // @step And the CreateSessionDialog does NOT mount
    assert!(
        !app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "the cycle gesture must NOT mount the CreateSessionDialog; layers={:?}",
        app.compositor().layer_ids()
    );
    Ok(())
}

#[tokio::test]
async fn scenario_bug203_shift_right_from_board_with_attached_session_still_jumps_into_it(
) -> TestResult {
    // @step Given the board has one open session "s-1" attached to work unit "AUTH-001"
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    app.board_store_mut()
        .attach_session("AUTH-001", SessionId::new("s-1"));
    // @step And "AUTH-001" is selected in the focused column
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);

    // @step When the user presses Shift+Right
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::SHIFT));
    drain_pending(&mut app).await?;

    // @step Then the active view is the Agent view on session "s-1"
    assert_eq!(app.active_view(), ViewMode::Agent);
    assert!(
        app.agent_view_store()
            .current_session()
            .is_some_and(|s| s == &SessionId::new("s-1")),
        "the attached-session fast path must jump into s-1; got {:?}",
        app.agent_view_store().current_session()
    );
    // @step And the CreateSessionDialog does NOT mount
    assert!(!app.compositor().contains(CREATE_SESSION_DIALOG_ID));
    Ok(())
}

#[tokio::test]
async fn scenario_bug203_kanban_dropdown_new_agent_row_mounts_dialog() -> TestResult {
    // @step Given an agent is running on session "s-1"
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    app.board_store_mut().attach_session("AUTH-001", sid("s-1"));
    app.board_store_mut().set_focused_column("blocked");
    app.board_store_mut().set_selected_index_for("backlog", 0);
    // @step And the board's Kanban dropdown is open with the cursor on the 'New Agent' row
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE)); // → Item(0) = Kanban
    drain_pending(&mut app).await?;
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE)); // open the Kanban dropdown
    drain_pending(&mut app).await?;
    assert_eq!(
        app.board_store().open_menu(),
        Some((0, 0)),
        "the Kanban dropdown must be open at row 0"
    );

    // @step When I click the 'New Agent' row (row 0 of the open dropdown)
    let buf = render_app_at(&mut app, 120, 24)?;
    let bar_y = (0..buf.area.height)
        .find(|&yy| row_text(&buf, yy).contains("[ Kanban ]"))
        .or_fail("the bar row")?;
    let y = (bar_y + 1..buf.area.height)
        .find(|&yy| row_text(&buf, yy).contains("New Agent"))
        .or_fail("the open Kanban dropdown row for 'New Agent'")?;
    let x = find_x(&buf, y, "New Agent").or_fail("New Agent row x")?;
    app.handle_event(&Event::Mouse(crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    }));
    drain_pending(&mut app).await?;

    // @step Then the CreateSessionDialog mounts over the board instead of switching into the running agent "s-1"
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "the dropdown row must mount the CreateSessionDialog (BUG-203); layers={:?}",
        app.compositor().layer_ids()
    );
    assert_eq!(app.active_view(), ViewMode::Board);
    // @step And the dropdown closes and the bar de-selects exactly once (the execute path)
    assert!(app.board_store().open_menu().is_none());
    assert!(app.board_store().menu_focus().is_none());
    Ok(())
}

#[tokio::test]
async fn scenario_bug203_u_help_dialog_new_agent_row_mounts_dialog() -> TestResult {
    // @step Given an agent is running on session "s-1"
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    app.board_store_mut().attach_session("AUTH-001", sid("s-1"));
    app.board_store_mut().set_focused_column("backlog");
    app.board_store_mut().set_selected_index_for("backlog", 0);
    // @step And the 'u' menu-bar-help dialog is open with '. New Agent' (the first row) highlighted
    let _ = app.handle_event(&key(KeyCode::Char('u'), KeyModifiers::NONE));
    drain_pending(&mut app).await?;
    assert!(app.compositor().contains(MENU_BAR_HELP_DIALOG_ID));

    // @step When I press Enter on the '. New Agent' row
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await?;

    // @step Then the menu-bar-help dialog is closed
    assert!(!app.compositor().contains(MENU_BAR_HELP_DIALOG_ID));
    // @step And the CreateSessionDialog mounts over the board instead of switching into the running agent "s-1"
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "the 'u' dialog's row must mount the CreateSessionDialog (BUG-203); layers={:?}",
        app.compositor().layer_ids()
    );
    assert_eq!(app.active_view(), ViewMode::Board);
    Ok(())
}

#[tokio::test]
async fn scenario_bug203_mux_top_bar_kanban_new_agent_row_mounts_dialog() -> TestResult {
    // @step Given mux mode is active with the board's top bar painted
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await?;
    seed_units(&mut app).await?;
    enable_mux(&mut app, vec![MuxPaneKind::Board, MuxPaneKind::Agent]);
    render_app_at(&mut app, 120, 24)?;
    // @step And an agent is running on session "s-1"
    // (seeded above)
    // @step And the mux top bar's Kanban dropdown is open with the cursor on the 'New Agent' row
    // (the mux bar's ring entry: Right off the board's last column, then
    // Enter opens the Kanban dropdown at row 0 — the menu004 pattern)
    app.board_store_mut().set_focused_column("blocked");
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE)); // → Item(0) = Kanban
    drain_pending(&mut app).await?;
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE)); // open the Kanban dropdown
    drain_pending(&mut app).await?;

    // @step When I press Enter on the 'New Agent' row (row 0)
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await?;

    // @step Then the CreateSessionDialog mounts over the mux grid
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "the mux Kanban row must mount the CreateSessionDialog (BUG-203); layers={:?}",
        app.compositor().layer_ids()
    );
    // @step And the active view remains the mux (confirming the dialog later is what flips, not the row pick)
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "the row pick must not flip the view"
    );
    Ok(())
}
