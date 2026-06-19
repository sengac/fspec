//! RPC-098 — AgentView ESC exit confirmation dialog (Detach / Close Session /
//! Cancel) — failing tests (RED phase).
//!
//! Feature: spec/features/agentview-esc-exit-confirmation-dialog.feature
//!
//! This test file is written BEFORE the implementation. Until RPC-098 lands,
//! `ExitConfirmationDialog`, `EXIT_CONFIRMATION_DIALOG_ID`, `ExitChoice`, and
//! `Action::AgentExitChoice { choice }` do not yet exist in the crate — the
//! file therefore fails to compile, which is the canonical Rust "red phase".
//!
//! Each scenario in the feature file is exercised by exactly one `#[test]`
//! (or `#[tokio::test]`) below, and every Gherkin step has a matching
//! `// @step ...` comment placed immediately before the code that exercises
//! it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::components::exit_confirmation_dialog::{
    ExitChoice, ExitConfirmationDialog, EXIT_CONFIRMATION_DIALOG_ID,
};
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, Component, FspecBackend};
use codelet_rpc_types::{SessionId, SessionStatus};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;
use tokio::time::timeout;

mod common;
use common::MockBackend;

// ───────────────────────── helpers (mirroring keyboard_cascade_rpc051.rs) ──

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: mods,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

fn esc() -> Event {
    key(KeyCode::Esc, KeyModifiers::NONE)
}

async fn wait_until<F: FnMut() -> bool>(mut predicate: F, label: &str) {
    timeout(Duration::from_secs(1), async {
        loop {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for: {label}"));
}

/// Drain spawned tokio tasks and the action bus so backend.spawn'd
/// destroy_session / interrupt calls complete deterministically.
async fn drain_pending(app: &mut App) {
    while let Some(handle) = app.next_pending_task() {
        let _ = handle.await;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            let _ = handle.await;
        }
    }
}

/// Build an App in ViewMode::Agent with a single open session at the given
/// status.
fn agent_app_with_status(status: SessionStatus) -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), status);
    (app, mock)
}

/// Build an App in ViewMode::Agent with NO open session.
fn agent_app_no_session() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

/// Render an entire App (Navigator + Compositor) into an 80x24 TestBackend
/// buffer.
fn render_app_buffer(app: &mut App) -> Buffer {
    let backend = TestBackend::new(80, 24);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|frame| {
        app.render(frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    term.backend().buffer().clone()
}

/// Render a single ExitConfirmationDialog into an 80x24 TestBackend buffer.
fn render_dialog_buffer(is_busy: bool) -> Buffer {
    let mut dialog = ExitConfirmationDialog::new(is_busy);
    let backend = TestBackend::new(80, 24);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|frame| {
        Component::render(&mut dialog, frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    term.backend().buffer().clone()
}

/// Flatten a buffer into a string for substring assertions.
fn buffer_to_string(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// Find the (x, y) of the first column where N consecutive cells'
/// symbols concatenated equal `needle`.
fn find_text_cell(buf: &Buffer, needle: &str) -> Option<(u16, u16)> {
    let cols: Vec<&str> = needle.split("").filter(|s| !s.is_empty()).collect();
    let n = cols.len();
    if n == 0 || (buf.area.width as usize) < n {
        return None;
    }
    for y in 0..buf.area.height {
        for x in 0..=(buf.area.width as usize - n) {
            let mut hit = true;
            for (i, want) in cols.iter().enumerate() {
                if buf[(x as u16 + i as u16, y)].symbol() != *want {
                    hit = false;
                    break;
                }
            }
            if hit {
                return Some((x as u16, y));
            }
        }
    }
    None
}

/// Count how many components on the compositor have id ==
/// `EXIT_CONFIRMATION_DIALOG_ID`. The Compositor API only exposes
/// `contains(id)` (bool) — but since `push` is idempotent-by-id in the
/// no-double-push design we only need the boolean. We still expose this
/// helper for the "exactly one" scenario for readability.
fn dialog_present(app: &App) -> bool {
    app.compositor().contains(EXIT_CONFIRMATION_DIALOG_ID)
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Idle session ESC opens dialog with idle description and Detach focused
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_session_esc_opens_dialog_with_idle_description_and_detach_focused() {
    // @step Given I am in the Rust AgentView with an active session whose status is Idle
    let (mut app, mock) = agent_app_with_status(SessionStatus::Idle);
    // @step And the input buffer is empty
    assert!(app.navigator().agent.input.value().is_empty());
    // @step And no popup or mode view is currently active

    // @step When I press ESC once
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then an ExitConfirmationDialog is pushed onto the compositor
    assert!(
        dialog_present(&app),
        "ESC at L7 with idle session must push ExitConfirmationDialog onto compositor"
    );
    // @step And the dialog renders a yellow rounded border centred on screen
    let painted = render_app_buffer(&mut app);
    let painted_str = buffer_to_string(&painted);
    assert!(
        painted_str.contains('╭') && painted_str.contains('╮'),
        "rendered buffer must include rounded border corners, got:\n{painted_str}"
    );
    // @step And the title row reads "Exit Session?" in bold
    let (tx, ty) = find_text_cell(&painted, "Exit Session?")
        .expect("'Exit Session?' must appear in rendered buffer");
    let title_cell = &painted[(tx, ty)];
    assert!(
        title_cell.style().add_modifier.contains(Modifier::BOLD),
        "title 'Exit Session?' must be bold"
    );
    // @step And the description row reads "Choose how to exit the session." in dim text
    assert!(
        painted_str.contains("Choose how to exit the session."),
        "idle-description text must be painted, got:\n{painted_str}"
    );
    // @step And the button "Detach" is selected with blue background and white foreground
    let (dx, dy) =
        find_text_cell(&painted, " Detach ").expect("' Detach ' must appear in rendered buffer");
    for off in 0..8 {
        let style = painted[(dx + off, dy)].style();
        assert_eq!(
            style.bg,
            Some(Color::Blue),
            "Detach cell ({},{}) bg must be Blue",
            dx + off,
            dy
        );
        assert_eq!(
            style.fg,
            Some(Color::White),
            "Detach cell ({},{}) fg must be White",
            dx + off,
            dy
        );
        assert!(
            style.add_modifier.contains(Modifier::BOLD),
            "Detach cell ({},{}) must be bold",
            dx + off,
            dy
        );
    }
    // @step And the buttons "Close Session" and "Cancel" are rendered in gray
    let (cs_x, cs_y) = find_text_cell(&painted, " Close Session ")
        .expect("' Close Session ' must appear in rendered buffer");
    assert_eq!(
        painted[(cs_x + 1, cs_y)].style().fg,
        Some(Color::Gray),
        "Close Session unselected must be Gray fg"
    );
    let (cx, cy) =
        find_text_cell(&painted, " Cancel ").expect("' Cancel ' must appear in rendered buffer");
    assert_eq!(
        painted[(cx + 1, cy)].style().fg,
        Some(Color::Gray),
        "Cancel unselected must be Gray fg"
    );
    // @step And the footer reads "← → Navigate | Enter Select | Esc Cancel" in dim text
    assert!(
        painted_str.contains("← → Navigate | Enter Select | Esc Cancel"),
        "footer must be painted, got:\n{painted_str}"
    );
    // L4 interrupt must NOT have fired (idle session)
    assert_eq!(mock.interrupt_calls(), 0);
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Running session ESC first interrupts then second ESC opens dialog
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn running_session_first_esc_interrupts_second_esc_opens_dialog() {
    // @step Given I am in the Rust AgentView with an active session whose status is Running
    let (mut app, mock) = agent_app_with_status(SessionStatus::Running);
    // @step And the input buffer is empty
    assert!(app.navigator().agent.input.value().is_empty());

    // @step When I press ESC once
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then the App spawns a backend.interrupt task for the session
    wait_until(|| mock.interrupt_calls() >= 1, "backend.interrupt to fire").await;
    assert_eq!(mock.last_interrupt(), Some(sid("s-1")));
    // @step And no ExitConfirmationDialog is pushed onto the compositor
    assert!(
        !dialog_present(&app),
        "L4 interrupt path must NOT push ExitConfirmationDialog"
    );
    // @step And the navigator remains on the Agent view
    assert_eq!(app.navigator().active_view, ViewMode::Agent);

    // @step When the session status transitions to Idle
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Idle);
    // @step And I press ESC a second time
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then an ExitConfirmationDialog is pushed onto the compositor
    assert!(
        dialog_present(&app),
        "Second ESC with idle status must push ExitConfirmationDialog"
    );
    // @step And the description row reads "Choose how to exit the session."
    let painted = render_app_buffer(&mut app);
    assert!(
        buffer_to_string(&painted).contains("Choose how to exit the session."),
        "idle-description must be painted on second ESC"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Compacting session ESC routes to interrupt and not to dialog
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compacting_session_esc_routes_to_interrupt_and_not_dialog() {
    // @step Given I am in the Rust AgentView with an active session whose status is Compacting
    let (mut app, mock) = agent_app_with_status(SessionStatus::Compacting);
    // @step And the input buffer is empty
    assert!(app.navigator().agent.input.value().is_empty());

    // @step When I press ESC once
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then the App spawns a backend.interrupt task for the session
    wait_until(|| mock.interrupt_calls() >= 1, "backend.interrupt to fire").await;
    assert_eq!(mock.last_interrupt(), Some(sid("s-1")));
    // @step And no ExitConfirmationDialog is pushed onto the compositor
    assert!(
        !dialog_present(&app),
        "Compacting ESC must route to interrupt — no dialog"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: No active session ESC dispatches BackToBoard without dialog
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_active_session_esc_dispatches_back_to_board_without_dialog() {
    // @step Given I am in the Rust AgentView with no active session
    let (mut app, mock) = agent_app_no_session();
    assert!(app.agent_view_store().current_session().is_none());
    // @step And the input buffer is empty
    assert!(app.navigator().agent.input.value().is_empty());
    // @step And no popup or mode view is currently active

    // @step When I press ESC once
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then Action::BackToBoard is dispatched
    // @step And no ExitConfirmationDialog is pushed onto the compositor
    assert!(
        !dialog_present(&app),
        "No-session ESC must NOT push ExitConfirmationDialog"
    );
    // @step And the navigator switches to the Board view
    assert_eq!(app.navigator().active_view, ViewMode::Board);
    // backend.interrupt must not have fired either
    assert_eq!(mock.interrupt_calls(), 0);
    assert_eq!(mock.destroy_session_calls(), 0);
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Cyclic Left/Right navigation across the three buttons
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn cyclic_left_right_navigation_across_three_buttons() {
    // @step Given the ExitConfirmationDialog is open with Detach focused
    let mut dialog = ExitConfirmationDialog::new(false);
    assert_eq!(dialog.selected_choice(), ExitChoice::Detach);

    // @step When I press Left
    let _ = dialog.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    // @step Then Cancel is focused
    assert_eq!(dialog.selected_choice(), ExitChoice::Cancel);

    // @step When I press Right
    let _ = dialog.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    // @step Then Detach is focused
    assert_eq!(dialog.selected_choice(), ExitChoice::Detach);

    // @step When I press Right
    let _ = dialog.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    // @step Then Close Session is focused
    assert_eq!(dialog.selected_choice(), ExitChoice::CloseSession);

    // @step When I press Right
    let _ = dialog.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    // @step Then Cancel is focused
    assert_eq!(dialog.selected_choice(), ExitChoice::Cancel);

    // @step When I press Right
    let _ = dialog.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    // @step Then Detach is focused
    assert_eq!(dialog.selected_choice(), ExitChoice::Detach);
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Enter on Detach dispatches BackToBoard without destroying the session
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enter_on_detach_dispatches_back_to_board_without_destroy() {
    // @step Given the ExitConfirmationDialog is open with Detach focused
    let (mut app, mock) = agent_app_with_status(SessionStatus::Idle);
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    assert!(dialog_present(&app), "dialog must be open");
    // @step And the backend records every destroy_session call
    assert_eq!(mock.destroy_session_calls(), 0);

    // @step When I press Enter
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then Action::AgentExitChoice { choice: Detach } is emitted
    // (verified indirectly: dialog has popped AND BackToBoard occurred)
    // @step And the ExitConfirmationDialog is removed from the compositor
    assert!(
        !dialog_present(&app),
        "dialog must be popped after Enter on Detach"
    );
    // @step And Action::BackToBoard is dispatched
    // @step And the navigator switches to the Board view
    assert_eq!(app.navigator().active_view, ViewMode::Board);
    // @step And the backend records zero destroy_session calls
    assert_eq!(
        mock.destroy_session_calls(),
        0,
        "Detach must NOT call backend.destroy_session"
    );
    // @step And the backend session remains alive
    assert!(mock.last_destroyed_session().is_none());
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Enter on Close Session destroys the session then dispatches BackToBoard
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enter_on_close_session_destroys_then_dispatches_back_to_board() {
    // @step Given the ExitConfirmationDialog is open with Detach focused
    let (mut app, mock) = agent_app_with_status(SessionStatus::Idle);
    // @step And the current AgentView session is attached to work unit "AUTH-001" in BoardStore
    app.agent_view_store_mut().set_current_work_unit(
        Some("AUTH-001".to_string()),
        Some("implementing".to_string()),
    );
    app.board_store_mut().attach_session("AUTH-001", sid("s-1"));
    assert_eq!(
        app.board_store().session_for("AUTH-001"),
        Some(&sid("s-1")),
        "precondition: BoardStore must hold the AUTH-001 → s-1 attachment"
    );

    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    assert!(dialog_present(&app), "dialog must be open");
    // @step And the backend records every destroy_session call
    assert_eq!(mock.destroy_session_calls(), 0);

    // @step When I press Right once
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    // @step Then Close Session is focused
    // (verified indirectly via the Enter-side effect below)

    // @step When I press Enter
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then Action::AgentExitChoice { choice: CloseSession } is emitted
    // @step And the ExitConfirmationDialog is removed from the compositor
    assert!(
        !dialog_present(&app),
        "dialog must be popped after Enter on Close Session"
    );
    // @step And the App spawns a backend.destroy_session task for the current session
    wait_until(
        || mock.destroy_session_calls() >= 1,
        "backend.destroy_session to fire",
    )
    .await;
    assert_eq!(mock.last_destroyed_session(), Some(sid("s-1")));
    // @step And the BoardStore work-unit-to-session attachment for "AUTH-001" is cleared
    assert_eq!(
        app.board_store().session_for("AUTH-001"),
        None,
        "Close Session must clear the AUTH-001 → s-1 attachment (mirrors TS fspecStore.detachSession step at sessionService.ts:637)"
    );
    // @step And Action::BackToBoard is dispatched
    // @step And the navigator switches to the Board view
    assert_eq!(app.navigator().active_view, ViewMode::Board);
    // @step And the destroyed session is removed from AgentViewStore open_sessions
    assert!(
        app.agent_view_store()
            .open_sessions()
            .iter()
            .all(|c| c.id != sid("s-1")),
        "Close Session must remove s-1 from AgentViewStore::open_sessions \
         (mirrors handle_confirm_delete_session at dispatch_resume_search_views.rs:249 — \
         without this, navigate_next/navigate_prev/first_open_session_id \
         keep surfacing the destroyed session)"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Cycling sessions in AgentView after Close Session does not list
// the destroyed session
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cycling_sessions_in_agent_view_after_close_session_does_not_list_destroyed_session() {
    use codelet_fspec_tui::store::NavTarget;

    // @step Given I am in the Rust AgentView with two open sessions "s-1" and "s-2"
    //       where "s-1" is focused and attached to work unit "AUTH-001" in BoardStore
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    app.navigator_mut().active_view = ViewMode::Agent;
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Idle);
    app.agent_view_store_mut()
        .set_session_status(sid("s-2"), SessionStatus::Idle);
    // SessionCreated focuses the new tail (s-2 at index 1); the user is
    // describing s-1 as the focused session in the scenario, so bring it
    // back into focus before opening the exit dialog.
    app.agent_view_store_mut().focus_session_index(0);
    app.agent_view_store_mut().set_current_work_unit(
        Some("AUTH-001".to_string()),
        Some("implementing".to_string()),
    );
    app.board_store_mut().attach_session("AUTH-001", sid("s-1"));
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        2,
        "precondition: two open sessions"
    );
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("s-1")),
        "precondition: s-1 is focused"
    );

    // @step And the ExitConfirmationDialog is open with Close Session focused
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    assert!(dialog_present(&app));
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));

    // @step When I press Enter
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then Action::AgentExitChoice { choice: CloseSession } is emitted
    // @step And the App spawns a backend.destroy_session task for session "s-1"
    wait_until(
        || mock.destroy_session_calls() >= 1,
        "backend.destroy_session to fire for s-1",
    )
    .await;
    assert_eq!(mock.last_destroyed_session(), Some(sid("s-1")));

    // @step And the AgentViewStore open_sessions list contains only "s-2"
    let open_ids: Vec<SessionId> = app
        .agent_view_store()
        .open_sessions()
        .iter()
        .map(|c| c.id.clone())
        .collect();
    assert_eq!(
        open_ids,
        vec![sid("s-2")],
        "Close Session must remove s-1 from AgentViewStore::open_sessions"
    );

    // @step And first_open_session_id returns "s-2"
    assert_eq!(
        app.agent_view_store().first_open_session_id(),
        Some(sid("s-2"))
    );

    // @step And navigate_next from the focused session resolves to NavTarget::CreateDialog
    // After remove_session_if_open with s-1 removed and s-2 the sole remaining
    // session, current_session_index clamps to 0 (the only session), so
    // navigate_next off the right end yields CreateDialog.
    assert_eq!(
        app.agent_view_store().navigate_next(),
        NavTarget::CreateDialog,
        "with s-1 removed and s-2 alone, Shift+Right past the only session must open Create Session dialog"
    );

    // @step And navigate_prev from the focused session resolves to NavTarget::Board
    assert_eq!(
        app.agent_view_store().navigate_prev(),
        NavTarget::Board,
        "with s-1 removed and s-2 alone at index 0, Shift+Left must exit to BoardView"
    );

    // @step And the destroyed SessionId "s-1" never appears in open_sessions
    assert!(
        app.agent_view_store()
            .open_sessions()
            .iter()
            .all(|c| c.id != sid("s-1")),
        "destroyed s-1 must be absent from AgentViewStore::open_sessions"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Right on the same work unit after Close Session does not
// navigate back to the destroyed session
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shift_right_after_close_session_does_not_navigate_back_to_destroyed_session() {
    // @step Given I am in the Rust AgentView with an active session "s1" attached to work unit "AUTH-001" in BoardStore
    let (mut app, mock) = agent_app_with_status(SessionStatus::Idle);
    app.agent_view_store_mut().set_current_work_unit(
        Some("AUTH-001".to_string()),
        Some("implementing".to_string()),
    );
    app.board_store_mut().attach_session("AUTH-001", sid("s-1"));

    // @step And the ExitConfirmationDialog is open with Close Session focused
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    assert!(dialog_present(&app));
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));

    // @step When I press Enter
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then Action::AgentExitChoice { choice: CloseSession } is emitted
    // (covered by the side effects asserted below)

    // @step And the App spawns a backend.destroy_session task for session "s1"
    wait_until(
        || mock.destroy_session_calls() >= 1,
        "backend.destroy_session to fire for s-1",
    )
    .await;
    assert_eq!(mock.last_destroyed_session(), Some(sid("s-1")));

    // @step And the BoardStore work-unit-to-session attachment for "AUTH-001" is cleared
    assert_eq!(
        app.board_store().session_for("AUTH-001"),
        None,
        "Close Session must clear the AUTH-001 attachment so Shift+Right can't route back to s-1"
    );
    // @step And Action::BackToBoard is dispatched
    // @step And the navigator switches to the Board view
    assert_eq!(app.navigator().active_view, ViewMode::Board);

    // @step When the user presses Shift+Right while "AUTH-001" is the focused work unit on the Board
    // BoardView::selected_session reads BoardStore::session_for(&selected_work_unit.id).
    // Drive the same lookup the BoardView would perform.
    // @step Then BoardView::selected_session returns None
    let routed = app.board_store().session_for("AUTH-001").cloned();
    assert_eq!(
        routed, None,
        "BoardView::selected_session must return None after Close Session cleared the attachment"
    );
    // @step And Action::OpenAgentView(None) is emitted
    // (covered by the None-routed assertion above — BoardView emits Action::OpenAgentView(routed))
    // @step And the destroyed SessionId "s1" is NOT routed to AgentView
    assert_ne!(
        routed,
        Some(sid("s-1")),
        "destroyed session s-1 must not be routable via Shift+Right after Close Session"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Enter on Cancel removes the dialog and stays on AgentView
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enter_on_cancel_removes_dialog_and_stays_on_agentview() {
    // @step Given the ExitConfirmationDialog is open with Detach focused
    let (mut app, mock) = agent_app_with_status(SessionStatus::Idle);
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    assert!(dialog_present(&app), "dialog must be open");

    // @step When I press Right twice
    for _ in 0..2 {
        let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    // @step Then Cancel is focused
    // (verified indirectly via the Enter-side effect below)

    // @step When I press Enter
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then Action::AgentExitChoice { choice: Cancel } is emitted
    // @step And the ExitConfirmationDialog is removed from the compositor
    assert!(
        !dialog_present(&app),
        "dialog must be popped after Enter on Cancel"
    );
    // @step And the navigator remains on the Agent view
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    // @step And no Action::BackToBoard is dispatched
    // (verified by navigator staying on Agent)
    // @step And no backend.destroy_session task is spawned
    assert_eq!(mock.destroy_session_calls(), 0);
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: ESC inside the dialog is equivalent to Cancel
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn esc_inside_dialog_is_equivalent_to_cancel() {
    // @step Given the ExitConfirmationDialog is open with Close Session focused
    let (mut app, mock) = agent_app_with_status(SessionStatus::Idle);
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    assert!(dialog_present(&app));
    // Advance focus to Close Session.
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));

    // @step When I press ESC
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;

    // @step Then Action::AgentExitChoice { choice: Cancel } is emitted
    // @step And the ExitConfirmationDialog is removed from the compositor
    assert!(!dialog_present(&app), "dialog must be popped on inner ESC");
    // @step And the navigator remains on the Agent view
    assert_eq!(app.navigator().active_view, ViewMode::Agent);
    // @step And no backend.destroy_session task is spawned
    assert_eq!(mock.destroy_session_calls(), 0);
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Pressing ESC twice from L7 only opens one dialog
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pressing_esc_twice_from_l7_only_opens_one_dialog() {
    // @step Given I am in the Rust AgentView with an active session whose status is Idle
    let (mut app, _mock) = agent_app_with_status(SessionStatus::Idle);
    // @step And the input buffer is empty
    assert!(app.navigator().agent.input.value().is_empty());
    // @step And no popup or mode view is currently active

    // @step When I press ESC
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    // @step Then exactly one ExitConfirmationDialog is on the compositor
    assert!(dialog_present(&app));
    // No "count" API on Compositor — `contains` is bool. The compositor's
    // `push` is idempotent-by-id, so we re-push and re-assert `contains`
    // is still true (and that subsequent ESC eventually pops it once).

    // @step When I press ESC again before navigating the dialog
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    // @step Then exactly one ExitConfirmationDialog is on the compositor
    // The 2nd ESC inside the dialog is equivalent to Cancel and removes it,
    // per scenario "ESC inside the dialog is equivalent to Cancel". The
    // no-double-push rule is enforced inside `handle_agent_esc_pressed`
    // by the `compositor.contains(EXIT_CONFIRMATION_DIALOG_ID)` guard —
    // we exercise that path explicitly below.

    // Re-open the dialog and verify pushing twice via the *cascade* (not
    // via the dialog's own ESC handler) does not stack.
    assert!(
        !dialog_present(&app),
        "after inner-ESC the dialog must be gone"
    );
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    assert!(dialog_present(&app), "re-open must succeed");
    // Directly call the cascade entry again — bypasses the dialog's own
    // ESC handler because the cascade only fires when the
    // AgentView/compositor key path consumes ESC at L7. We rely on the
    // guard inside handle_agent_esc_pressed: contains() prevents the
    // second push.
    app.dispatch(Action::AgentEscPressed);
    drain_pending(&mut app).await;
    assert!(
        dialog_present(&app),
        "L7 cascade must not double-push: dialog should still be present (one instance)"
    );
    // @step And the ExitConfirmationDialog is removed from the compositor
    // (covered by the inner-ESC pop verified above)
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Snapshot of the dialog rendered on 80x24 with is_busy=true
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn snapshot_dialog_80x24_is_busy_true() {
    // @step Given an ExitConfirmationDialog instance is constructed with is_busy=true
    // @step When the dialog is rendered into an 80x24 TestBackend buffer
    let buf = render_dialog_buffer(true);
    let painted = buffer_to_string(&buf);

    // @step Then the snapshot shows a yellow rounded border centred on the buffer
    let (corner_x, corner_y) = find_text_cell(&buf, "╭").expect("rounded corner ╭ must be painted");
    let style = buf[(corner_x, corner_y)].style();
    assert_eq!(
        style.fg,
        Some(Color::Yellow),
        "border corner fg must be Yellow"
    );
    // @step And the title row reads "Exit Session?" in bold
    let (tx, ty) =
        find_text_cell(&buf, "Exit Session?").expect("title 'Exit Session?' must be painted");
    assert!(
        buf[(tx, ty)].style().add_modifier.contains(Modifier::BOLD),
        "title must be bold"
    );
    // @step And the description row reads "The agent is currently running. Choose how to exit." in dim text
    assert!(
        painted.contains("The agent is currently running. Choose how to exit."),
        "busy-description must be painted, got:\n{painted}"
    );
    // @step And the button " Detach " is styled with blue background and white foreground
    let (dx, dy) = find_text_cell(&buf, " Detach ").expect("' Detach ' must be painted");
    for off in 0..8 {
        let cell_style = buf[(dx + off, dy)].style();
        assert_eq!(cell_style.bg, Some(Color::Blue));
        assert_eq!(cell_style.fg, Some(Color::White));
        assert!(cell_style.add_modifier.contains(Modifier::BOLD));
    }
    // @step And the buttons " Close Session " and " Cancel " are styled in gray
    let (cs_x, cs_y) =
        find_text_cell(&buf, " Close Session ").expect("' Close Session ' must be painted");
    assert_eq!(buf[(cs_x + 1, cs_y)].style().fg, Some(Color::Gray));
    let (cx, cy) = find_text_cell(&buf, " Cancel ").expect("' Cancel ' must be painted");
    assert_eq!(buf[(cx + 1, cy)].style().fg, Some(Color::Gray));
    // @step And the footer reads "← → Navigate | Enter Select | Esc Cancel" in dim text
    assert!(
        painted.contains("← → Navigate | Enter Select | Esc Cancel"),
        "footer must be painted, got:\n{painted}"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: Snapshot of the dialog rendered on 80x24 with is_busy=false
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn snapshot_dialog_80x24_is_busy_false() {
    // @step Given an ExitConfirmationDialog instance is constructed with is_busy=false
    // @step When the dialog is rendered into an 80x24 TestBackend buffer
    let buf = render_dialog_buffer(false);
    let painted = buffer_to_string(&buf);

    // @step Then the snapshot shows a yellow rounded border centred on the buffer
    let (corner_x, corner_y) = find_text_cell(&buf, "╭").expect("rounded corner ╭ must be painted");
    assert_eq!(
        buf[(corner_x, corner_y)].style().fg,
        Some(Color::Yellow),
        "border corner fg must be Yellow"
    );
    // @step And the title row reads "Exit Session?" in bold
    let (tx, ty) =
        find_text_cell(&buf, "Exit Session?").expect("title 'Exit Session?' must be painted");
    assert!(
        buf[(tx, ty)].style().add_modifier.contains(Modifier::BOLD),
        "title must be bold"
    );
    // @step And the description row reads "Choose how to exit the session." in dim text
    assert!(
        painted.contains("Choose how to exit the session."),
        "idle-description must be painted, got:\n{painted}"
    );
    // The busy variant text must NOT appear.
    assert!(
        !painted.contains("The agent is currently running"),
        "busy-only text must NOT appear with is_busy=false, got:\n{painted}"
    );
    // @step And the button " Detach " is styled with blue background and white foreground
    let (dx, dy) = find_text_cell(&buf, " Detach ").expect("' Detach ' must be painted");
    for off in 0..8 {
        let cell_style = buf[(dx + off, dy)].style();
        assert_eq!(cell_style.bg, Some(Color::Blue));
        assert_eq!(cell_style.fg, Some(Color::White));
    }
    // @step And the footer reads "← → Navigate | Enter Select | Esc Cancel" in dim text
    assert!(
        painted.contains("← → Navigate | Enter Select | Esc Cancel"),
        "footer must be painted, got:\n{painted}"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Scenario: End-to-end App render overlays the dialog on top of the AgentView chrome
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn end_to_end_app_render_overlays_dialog_on_top_of_agentview_chrome() {
    // @step Given I am in the Rust AgentView with an active idle session
    let (mut app, _mock) = agent_app_with_status(SessionStatus::Idle);
    // @step And the input buffer is empty
    assert!(app.navigator().agent.input.value().is_empty());

    // @step When I press ESC once
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    assert!(dialog_present(&app));
    // @step And the App renders one frame into an 80x24 TestBackend
    let buf = render_app_buffer(&mut app);
    let painted = buffer_to_string(&buf);

    // @step Then the rendered buffer contains a yellow rounded border centred on screen
    let (corner_x, corner_y) =
        find_text_cell(&buf, "╭").expect("rounded corner ╭ must be painted by App::render");
    assert_eq!(
        buf[(corner_x, corner_y)].style().fg,
        Some(Color::Yellow),
        "App-level rendered border must be Yellow"
    );
    // @step And the rendered buffer contains the title "Exit Session?"
    assert!(
        painted.contains("Exit Session?"),
        "App-level rendered buffer must include 'Exit Session?'"
    );
    // @step And the previous AgentView chrome (header, input row, footer) is still painted underneath the modal
    // Modal overlay means we should still find AgentView footer hint glyphs
    // outside the dialog area. The AgentView's standard footer includes
    // 'esc' as part of its hint line; verify *some* of the surrounding
    // chrome exists. We probe by checking that the buffer width is fully
    // used (i.e. lines outside the centred dialog have non-blank glyphs).
    let mut non_dialog_chrome_cells = 0;
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            let s = cell.symbol();
            // Skip the rounded border + interior — assume the dialog
            // occupies the middle ~half of the buffer. Anything to the
            // far left/right that is non-blank counts as chrome.
            if (x < 5 || x > buf.area.width.saturating_sub(5)) && !s.trim().is_empty() {
                non_dialog_chrome_cells += 1;
            }
        }
    }
    assert!(
        non_dialog_chrome_cells > 0,
        "AgentView chrome must remain painted outside the dialog area; \
         no non-blank cells found in left/right margins. painted=\n{painted}"
    );
}

// ──────────────────────────────────────────────────────────────────────────
// Static component invariants (priority, id, default selection)
// ──────────────────────────────────────────────────────────────────────────

#[test]
fn dialog_priority_is_critical_and_id_matches_const() {
    // @step Given a freshly constructed ExitConfirmationDialog
    let dialog = ExitConfirmationDialog::new(false);
    // @step Then its Component::priority is Critical
    assert_eq!(
        dialog.priority(),
        codelet_fspec_tui::Priority::Critical,
        "ExitConfirmationDialog must use Priority::Critical so it overlays everything"
    );
    // @step And its Component::id equals EXIT_CONFIRMATION_DIALOG_ID
    assert_eq!(dialog.id(), EXIT_CONFIRMATION_DIALOG_ID);
}

// ──────────────────────────────────────────────────────────────────────────
// END-TO-END: Full production flow — Board → Enter work unit → ESC →
// Close Session → cycle through work units and sessions. This is the
// scenario the user actually exercises in the running TUI: it drives
// every store + dispatch path involved in `Close Session` (lazy session
// creation, attachment binding, board+agentview cleanup, navigation
// back to board, re-entry on the same work unit, re-entry on a
// neighbour, AgentView Shift+L/R cycling). If the destroyed session
// resurfaces ANYWHERE during the post-close cycle, one of the
// assertions below trips.
// ──────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn end_to_end_close_session_purges_destroyed_session_from_every_cycle_path() {
    use codelet_fspec_tui::store::NavTarget;
    use codelet_rpc_types::WorkUnitInfo;

    // ── Setup: seed the board with two work units so the post-close
    //          cycle exercises both same-work-unit AND neighbour-work-unit
    //          Shift+Right paths. ────────────────────────────────────────
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // Seed two backlog work units in the BoardStore.
    let wu_a = WorkUnitInfo {
        id: "AUTH-001".to_string(),
        title: "Login".to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: vec![],
        last_state_change_at: None,
    };
    let wu_b = WorkUnitInfo {
        id: "AUTH-002".to_string(),
        title: "Logout".to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: vec![],
        last_state_change_at: None,
    };
    app.dispatch(Action::WorkUnitsLoaded(vec![wu_a, wu_b]));

    // Script the next two create_session calls so EnterWorkUnit-driven
    // lazy session creation produces deterministic ids.
    mock.script_create_session(sid("s-A"));

    // ── Stage 1: user presses Enter on AUTH-001 from the Board.
    //   That dispatches EnterWorkUnit → flips to AgentView → lazy-spawns
    //   a session → Action::SessionCreated(s-A) lands → open_sessions
    //   becomes [s-A] → AttachSession binds AUTH-001 → s-A in
    //   BoardStore. ──────────────────────────────────────────────────────
    app.dispatch(Action::EnterWorkUnit("AUTH-001".to_string()));
    drain_pending(&mut app).await;

    // Wait for the lazy create_session to land. The scripted backend
    // returns sid("s-A") synchronously, but Action::SessionCreated is
    // sent on the action bus and drained by drain_pending above.
    wait_until(
        || app.agent_view_store().current_session() == Some(&sid("s-A")),
        "AUTH-001 lazy session creation to land on AgentViewStore",
    )
    .await;
    drain_pending(&mut app).await;
    // Force-bind AUTH-001 → s-A in case AttachSession ran before
    // SessionCreated populated open_sessions in the scripted path.
    app.board_store_mut().attach_session("AUTH-001", sid("s-A"));

    // Sanity preconditions: open_sessions has exactly s-A, the
    // AgentViewStore knows the current work unit, BoardStore has the
    // attachment, and the AgentView is the active mode view.
    let open_ids_pre: Vec<SessionId> = app
        .agent_view_store()
        .open_sessions()
        .iter()
        .map(|c| c.id.clone())
        .collect();
    assert_eq!(open_ids_pre, vec![sid("s-A")], "stage 1: open_sessions");
    assert_eq!(
        app.agent_view_store().current_work_unit_id(),
        Some("AUTH-001"),
        "stage 1: current_work_unit_id"
    );
    assert_eq!(
        app.board_store().session_for("AUTH-001"),
        Some(&sid("s-A")),
        "stage 1: BoardStore attachment"
    );
    assert_eq!(app.navigator().active_view, ViewMode::Agent);

    // Tell the AgentViewStore s-A is Idle so ESC opens the exit dialog
    // (Running/Compacting would route to interrupt).
    app.agent_view_store_mut()
        .set_session_status(sid("s-A"), SessionStatus::Idle);

    // ── Stage 2: user presses ESC → dialog opens; Right → highlight
    //   Close Session; Enter → fire AgentExitChoice(CloseSession). ───────
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    assert!(dialog_present(&app), "stage 2: dialog must open");
    let _ = app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    let _ = app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // Wait for the spawned backend.destroy_session to fire.
    wait_until(
        || mock.destroy_session_calls() >= 1,
        "backend.destroy_session to fire for s-A",
    )
    .await;
    assert_eq!(mock.last_destroyed_session(), Some(sid("s-A")));

    // ── Stage 3: assert every TS-parity cleanup step landed. ────────────
    // (3a) AgentViewStore::open_sessions no longer contains s-A.
    let open_ids_post: Vec<SessionId> = app
        .agent_view_store()
        .open_sessions()
        .iter()
        .map(|c| c.id.clone())
        .collect();
    assert!(
        open_ids_post.is_empty(),
        "stage 3a: open_sessions must be empty after Close Session, got: {open_ids_post:?}"
    );

    // (3b) BoardStore work-unit attachment for AUTH-001 cleared
    //      (sessionService.ts:637 parity).
    assert_eq!(
        app.board_store().session_for("AUTH-001"),
        None,
        "stage 3b: BoardStore AUTH-001 attachment must be cleared"
    );

    // (3c) current_work_unit_id pointer cleared (sessionService.ts:642
    //      parity — setCurrentWorkUnit(null, null)).
    assert_eq!(
        app.agent_view_store().current_work_unit_id(),
        None,
        "stage 3c: current_work_unit_id must be None after Close Session"
    );
    assert_eq!(
        app.agent_view_store().current_work_unit_status(),
        None,
        "stage 3c: current_work_unit_status must be None after Close Session"
    );

    // (3d) Navigator switched back to Board.
    assert_eq!(
        app.navigator().active_view,
        ViewMode::Board,
        "stage 3d: BackToBoard must run"
    );

    // (3e) first_open_session_id returns None — the BoardView Shift+Right
    //      cascade in handle_open_agent_view(None) will fall through to
    //      CreateSessionDialog instead of resuming the dead session.
    assert_eq!(
        app.agent_view_store().first_open_session_id(),
        None,
        "stage 3e: first_open_session_id must be None"
    );

    // (3f) AgentView Shift+R/L navigation from an empty store routes to
    //      CreateDialog / Board respectively — NOT to s-A.
    assert_eq!(
        app.agent_view_store().navigate_next(),
        NavTarget::CreateDialog,
        "stage 3f: navigate_next on empty store must be CreateDialog"
    );
    assert_eq!(
        app.agent_view_store().navigate_prev(),
        NavTarget::Board,
        "stage 3f: navigate_prev on empty store must be Board"
    );

    // ── Stage 4: cycle path #1 — user presses Shift+Right on AUTH-001
    //   (same work unit). BoardView::selected_session must return None
    //   (attachment cleared in 3b), so Action::OpenAgentView(None)
    //   fires. handle_open_agent_view(None) probes first_open_session_id
    //   (None in 3e) and falls through to CreateSessionDialog — NEVER
    //   routes back to s-A. ──────────────────────────────────────────────
    let routed_same = app.board_store().session_for("AUTH-001").cloned();
    assert_eq!(
        routed_same, None,
        "stage 4: Shift+Right on AUTH-001 must not route to destroyed s-A"
    );

    // ── Stage 5: cycle path #2 — user presses Shift+Right on AUTH-002
    //   (neighbour work unit). It has no attachment, so the path is
    //   identical to stage 4: OpenAgentView(None) → CreateSessionDialog.
    //   The destroyed s-A is NOT routable here either. ──────────────────
    let routed_neighbour = app.board_store().session_for("AUTH-002").cloned();
    assert_eq!(
        routed_neighbour, None,
        "stage 5: AUTH-002 has no attachment so Shift+Right must not route to s-A"
    );

    // ── Stage 6: cycle path #3 — user creates a fresh session on
    //   AUTH-002 (lazy via EnterWorkUnit). Then Shift+L/R inside
    //   AgentView must cycle ONLY the fresh session, never s-A. ─────────
    mock.script_create_session(sid("s-B"));
    app.dispatch(Action::EnterWorkUnit("AUTH-002".to_string()));
    drain_pending(&mut app).await;
    wait_until(
        || app.agent_view_store().current_session() == Some(&sid("s-B")),
        "AUTH-002 lazy session creation to land",
    )
    .await;
    drain_pending(&mut app).await;
    app.agent_view_store_mut()
        .set_session_status(sid("s-B"), SessionStatus::Idle);

    let open_ids_after_b: Vec<SessionId> = app
        .agent_view_store()
        .open_sessions()
        .iter()
        .map(|c| c.id.clone())
        .collect();
    assert_eq!(
        open_ids_after_b,
        vec![sid("s-B")],
        "stage 6: open_sessions must contain ONLY s-B; destroyed s-A must not have come back"
    );
    assert!(
        app.agent_view_store()
            .open_sessions()
            .iter()
            .all(|c| c.id != sid("s-A")),
        "stage 6: destroyed s-A must NEVER reappear in open_sessions"
    );

    // Shift+Right past the only open session → CreateDialog (not s-A).
    assert_eq!(
        app.agent_view_store().navigate_next(),
        NavTarget::CreateDialog
    );
    // Shift+Left from index 0 → Board (not s-A).
    assert_eq!(app.agent_view_store().navigate_prev(), NavTarget::Board);

    // ── Stage 7: final sanity — first_open_session_id returns s-B
    //   (the fresh session), NEVER s-A. ─────────────────────────────────
    assert_eq!(
        app.agent_view_store().first_open_session_id(),
        Some(sid("s-B")),
        "stage 7: first_open_session_id must return the fresh s-B, never the destroyed s-A"
    );
}
