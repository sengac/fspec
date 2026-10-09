//! BUG-204 — the agent-bar 'Close Agent' button ALWAYS shows the exit
//! confirmation dialog (the RPC-098 'Exit Session?' modal) — it does NOT
//! run the AgentEscPressed cascade (no interrupt branch, no draft-clear
//! branch, no BackToBoard fallback).
//!
//! Feature: spec/features/bug204-close-agent-button-always-shows-exit-confirmation-dialog.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios (strict
//! arrange-act-assert).
//!
//! Harness: App + MockBackend (the `bug199_agent_bar_zone_c_semantics.rs`
//! pattern) — `fresh_app` + `drain_pending` + full-App render into a
//! TestBackend at 120x24. Observation points:
//! - `app.compositor().contains(EXIT_DIALOG_ID)` (R1: the dialog mounts in
//!   every state);
//! - `mock.interrupt_calls()` (R1: the button NEVER interrupts — the
//!   Esc-only branch);
//! - `app.navigator().agent.input.value()` (R1: the button NEVER clears a
//!   draft — the Esc-only L6 branch);
//! - `mock.destroy_session_calls()` (R2: the dialog gates the teardown);
//! - `app.active_view()` / `app.board_store().session_for("AUTH-001")`
//!   (R2: the Close Session / Cancel commits; BUG-205 removed Detach);
//! - `app.navigator().agent.menu_focus()` (R4: the 'leave the bar'
//!   gesture).
//!
//! Pane geometry (120x24, single agent view): the SessionHeader is row 0,
//! the 2-zone agent bar row 1 (the `bug199` AGENT_BAR_ROW constant).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionId, SessionStatus};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tokio::time::timeout;

mod common;
use common::MockBackend;

/// The agent bar's row in the full-App render (the SessionHeader is row 0,
/// the 2-zone bar row 1 — the `bug199` AGENT_BAR_ROW constant).
const AGENT_BAR_ROW: u16 = 1;

/// The stable compositor id of the RPC-098 'Exit Session?' dialog.
const EXIT_DIALOG_ID: &str = "exit-confirmation-dialog";

/// R1: the dialog's busy-variant description (Running/Compacting session).
const DESCRIPTION_BUSY: &str = "The agent is currently running. Choose how to exit.";

/// R1: the dialog's idle-variant description.
const DESCRIPTION_IDLE: &str = "Choose how to exit the session.";

// ─────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

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

fn key(code: KeyCode, mods: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, mods))
}

fn click(col: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// Render the full App (navigator + compositor) into a 120x24 buffer.
fn render_app(app: &mut App) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(120, 24)).expect("Terminal::new");
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

fn row_text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// The whole 120x24 render flattened — the dialog's description text is
/// painted by the compositor layer, so the R1 variant assertions search
/// the full buffer.
fn buffer_text(buf: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        out.push_str(&row_text(buf, y));
        out.push('\n');
    }
    out
}

/// The cell x of the first occurrence of `needle` on row `y`.
fn find_x(buf: &Buffer, y: u16, needle: &str) -> Option<u16> {
    let row = row_text(buf, y);
    let byte_idx = row.find(needle)?;
    Some(row[..byte_idx].chars().count() as u16)
}

fn sid(id: &str) -> SessionId {
    SessionId::new(id.to_string())
}

/// Seed `n` open sessions (s-1..s-n, Idle by default — `append_session`
/// focuses the LAST, so the current session is s-n).
async fn seed_sessions(app: &mut App, n: usize) {
    for i in 1..=n {
        app.dispatch(codelet_fspec_tui::Action::SessionCreated(sid(&format!(
            "s-{i}"
        ))));
    }
    drain_pending(app).await;
}

/// Enter the Agent view on the given session (the RPC-097
/// jump-into-existing-session path).
async fn enter_agent_view(app: &mut App, session: &str) {
    app.dispatch(codelet_fspec_tui::Action::OpenAgentView(Some(sid(session))));
    drain_pending(app).await;
}

/// Walk the agent ring to Zone C button `button` (0 = New Agent,
/// 1 = Close Agent — MENU-011 paints it bracketed) from the empty
/// input: Left enters the bar at 'Board View' (ring pos 0), then Right
/// walks the chips and the buttons (ring pos of ZoneC(i) = 1 + chip_count
/// + i).
async fn ring_to_zone_c(app: &mut App, chip_count: usize, button: usize) {
    app.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    drain_pending(app).await;
    for _ in 0..1 + chip_count + button {
        app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
        drain_pending(app).await;
    }
}

/// The 'Close Agent' button's x on the bar row (panics with the painted
/// row when missing — the `bug199` pattern).
fn close_agent_x(buf: &Buffer) -> u16 {
    find_x(buf, AGENT_BAR_ROW, "Close Agent").unwrap_or_else(|| {
        panic!(
            "the 'Close Agent' button must paint on the bar row (MENU-011: bracketed):\n{}",
            row_text(buf, AGENT_BAR_ROW)
        )
    })
}

/// Await `predicate` for up to a second (the Close Session commit spawns
/// the backend `destroy_session` task).
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

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: 'Close Agent' on a running session shows the exit confirmation
/// dialog (busy variant) — the BUG-199 R2 interrupt behavior is superseded.
#[tokio::test]
async fn scenario_close_agent_on_a_running_session_shows_the_exit_confirmation_dialog_busy_variant()
{
    // @step Given the agent view shows 1 open session that is RUNNING
    let (mut app, mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    // Deterministic status fold (the `keyboard_cascade_rpc051` pattern —
    // the store seam instead of the broadcast subscriber task).
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);
    // @step And the agent bar is painted
    let buf = render_app(&mut app);
    let x = close_agent_x(&buf);

    // @step When I left-click the 'Close Agent' button
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await;

    // @step Then the 'Exit Session?' confirmation dialog (Close Session / Cancel) is shown
    let buf = render_app(&mut app);
    assert!(
        app.compositor().contains(EXIT_DIALOG_ID),
        "'Close Agent' on a running session must mount the ExitConfirmationDialog (BUG-204 R1)"
    );
    // @step And the dialog shows the running-variant description 'The agent is currently running. Choose how to exit.'
    let text = buffer_text(&buf);
    assert!(
        text.contains(DESCRIPTION_BUSY),
        "the dialog must paint the busy-variant description: {text:?}"
    );
    assert!(
        !text.contains(DESCRIPTION_IDLE),
        "the dialog must NOT paint the idle-variant description: {text:?}"
    );
    // @step And the run is NOT interrupted and the session is not destroyed
    assert_eq!(
        mock.interrupt_calls(),
        0,
        "the button must NOT run the Esc cascade's interrupt branch (BUG-204 R1)"
    );
    assert_eq!(
        mock.destroy_session_calls(),
        0,
        "the button must never destroy the session directly (BUG-204 R2)"
    );
    // @step And the agent bar's ring focus clears
    assert!(
        app.navigator().agent.menu_focus().is_none(),
        "activating the button must clear the agent bar's ring focus (BUG-204 R4)"
    );
}

/// Scenario: 'Close Agent' with a non-empty input shows the exit
/// confirmation dialog (draft preserved) — the BUG-199 R2 draft-clear
/// behavior is superseded.
#[tokio::test]
async fn scenario_close_agent_with_a_non_empty_input_shows_the_exit_confirmation_dialog_draft_preserved(
) {
    // @step Given the agent view shows 1 open idle session
    let (mut app, mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    // @step And the input buffer contains a draft
    app.navigator_mut().agent.input.set_value("hello world");
    // @step And the agent bar is painted
    let buf = render_app(&mut app);
    let x = close_agent_x(&buf);

    // @step When I left-click the 'Close Agent' button
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await;

    // @step Then the 'Exit Session?' confirmation dialog (Close Session / Cancel) is shown
    let buf = render_app(&mut app);
    assert!(
        app.compositor().contains(EXIT_DIALOG_ID),
        "'Close Agent' with a draft must mount the ExitConfirmationDialog (BUG-204 R1)"
    );
    // @step And the dialog shows the idle-variant description 'Choose how to exit the session.'
    let text = buffer_text(&buf);
    assert!(
        text.contains(DESCRIPTION_IDLE),
        "the dialog must paint the idle-variant description: {text:?}"
    );
    // @step And the input buffer still contains the draft (the draft-clear Esc branch did NOT run)
    assert_eq!(
        app.navigator().agent.input.value(),
        "hello world",
        "the button must NOT run the Esc cascade's L6 draft-clear branch (BUG-204 R1)"
    );
    // @step And the session is not destroyed
    assert_eq!(
        mock.destroy_session_calls(),
        0,
        "the button must never destroy the session directly (BUG-204 R2)"
    );
}

/// Scenario: 'Close Agent' on an idle session shows the exit confirmation
/// dialog (unchanged — the BUG-199 R2 idle behavior is kept).
#[tokio::test]
async fn scenario_close_agent_on_an_idle_session_shows_the_exit_confirmation_dialog_unchanged() {
    // @step Given the agent view shows 1 open idle session with an empty input
    let (mut app, mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    // @step And the agent bar is painted
    let buf = render_app(&mut app);
    let x = close_agent_x(&buf);

    // @step When I left-click the 'Close Agent' button
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await;

    // @step Then the 'Exit Session?' confirmation dialog (Close Session / Cancel) is shown
    assert!(
        app.compositor().contains(EXIT_DIALOG_ID),
        "the idle branch must push the ExitConfirmationDialog (BUG-204 R1, RPC-098 L7)"
    );
    // @step And the session is not destroyed until an option is committed
    assert_eq!(
        mock.destroy_session_calls(),
        0,
        "the dialog must gate the teardown — no direct destroy"
    );
    // @step And the agent bar's ring focus clears
    assert!(
        app.navigator().agent.menu_focus().is_none(),
        "activating the button must clear the agent bar's ring focus (BUG-204 R4)"
    );
}

/// Scenario: Enter on the agent's 'Close Agent' button shows the exit
/// confirmation dialog (R4: the Enter arm is the same dialog).
#[tokio::test]
async fn scenario_enter_on_the_agents_close_agent_button_shows_the_exit_confirmation_dialog() {
    // @step Given the agent view shows 1 open idle session and the agent bar is painted
    let (mut app, mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    render_app(&mut app); // cache the agent bar geometry
                          // @step When the ring focuses 'Close Agent' and I press Enter
    ring_to_zone_c(&mut app, 1, 1).await;
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the 'Exit Session?' confirmation dialog (Close Session / Cancel) is shown
    assert!(
        app.compositor().contains(EXIT_DIALOG_ID),
        "Enter on 'Close Agent' must mount the ExitConfirmationDialog (BUG-204 R1)"
    );
    // @step And the session is not destroyed until an option is committed
    assert_eq!(
        mock.destroy_session_calls(),
        0,
        "the dialog must gate the teardown — no direct destroy"
    );
    // @step And the agent bar's ring focus clears
    assert!(
        app.navigator().agent.menu_focus().is_none(),
        "activating the button must clear the agent bar's ring focus (BUG-204 R4)"
    );
}

/// Scenario: 'Close Agent' commits 'Close Session' to destroy the session
/// and return to Board (R2: the existing AgentExitChoice teardown path).
#[tokio::test]
async fn scenario_close_agent_commits_close_session_to_destroy_the_session_and_return_to_board() {
    // @step Given the agent view shows 1 open session attached to work unit AUTH-001
    let (mut app, mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    app.agent_view_store_mut().set_current_work_unit(
        Some("AUTH-001".to_string()),
        Some("implementing".to_string()),
    );
    app.board_store_mut().attach_session("AUTH-001", sid("s-1"));
    // @step And the agent bar is painted
    let buf = render_app(&mut app);
    let x = close_agent_x(&buf);
    // @step And I left-click the 'Close Agent' button
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await;
    assert!(
        app.compositor().contains(EXIT_DIALOG_ID),
        "precondition: the dialog must be open"
    );

    // @step When I commit 'Close Session' in the 'Exit Session?' dialog
    // (pre-selected since BUG-205 removed the Detach option)
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the session is destroyed (backend.destroy_session)
    wait_until(
        || mock.destroy_session_calls() >= 1,
        "backend.destroy_session to fire",
    )
    .await;
    assert_eq!(mock.last_destroyed_session(), Some(sid("s-1")));
    // @step And the view returns to the Board view
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "Close Session must return to the Board view"
    );
    assert_eq!(
        app.board_store().session_for("AUTH-001"),
        None,
        "Close Session must clear the AUTH-001 → s-1 attachment"
    );
}

/// Scenario: 'Close Agent' commits the pre-selected 'Close Session' to
/// destroy the session and return to Board (BUG-205: the Detach option
/// was removed, so Enter commits Close Session — the destructive option
/// is pre-selected like the board 'Exit fspec?' dialog).
#[tokio::test]
async fn scenario_close_agent_commits_the_preselected_close_session_by_default() {
    // @step Given the agent view shows 1 open session and the agent bar is painted
    let (mut app, mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    let buf = render_app(&mut app);
    let x = close_agent_x(&buf);
    // @step And I left-click the 'Close Agent' button
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await;
    assert!(
        app.compositor().contains(EXIT_DIALOG_ID),
        "precondition: the dialog must be open"
    );

    // @step When I press Enter (Close Session is pre-selected)
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the session is destroyed (destroy_session called)
    wait_until(
        || mock.destroy_session_calls() >= 1,
        "backend.destroy_session to fire",
    )
    .await;
    assert_eq!(mock.last_destroyed_session(), Some(sid("s-1")));
    // @step And the view returns to the Board view
    assert_eq!(
        app.active_view(),
        ViewMode::Board,
        "Close Session must return to the Board view"
    );
}

/// Scenario: 'Close Agent' commits 'Cancel' to stay on the Agent view
/// (R2: the existing AgentExitChoice Cancel path).
#[tokio::test]
async fn scenario_close_agent_commits_cancel_to_stay_on_the_agent_view() {
    // @step Given the agent view shows 1 open idle session and the agent bar is painted
    let (mut app, mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    let buf = render_app(&mut app);
    let x = close_agent_x(&buf);
    // @step And I left-click the 'Close Agent' button
    app.handle_event(&click(x, AGENT_BAR_ROW));
    drain_pending(&mut app).await;
    assert!(
        app.compositor().contains(EXIT_DIALOG_ID),
        "precondition: the dialog must be open"
    );

    // @step When I commit 'Cancel' in the 'Exit Session?' dialog
    // (BUG-205: Close Session is pre-selected, so one Right reaches Cancel)
    app.handle_event(&key(KeyCode::Right, KeyModifiers::NONE)); // Close Session → Cancel
    app.handle_event(&key(KeyCode::Enter, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the dialog is dismissed
    assert!(
        !app.compositor().contains(EXIT_DIALOG_ID),
        "Cancel must remove the dialog from the compositor"
    );
    // @step And the view stays the Agent view
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "Cancel must keep the Agent view"
    );
    // @step And the session is not destroyed
    assert_eq!(
        mock.destroy_session_calls(),
        0,
        "Cancel must NOT destroy the session"
    );
}

/// Scenario: Pressing Esc on a running session still interrupts the run
/// (Esc cascade unchanged — R5).
#[tokio::test]
async fn scenario_pressing_esc_on_a_running_session_still_interrupts_the_run_esc_cascade_unchanged()
{
    // @step Given the agent view shows 1 open session that is RUNNING
    let (mut app, mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);

    // @step When I press Esc in the Agent view
    app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;
    // @step Then the run is interrupted
    wait_until(
        || mock.interrupt_calls() == 1,
        "backend.interrupt to fire (the Esc L4/L5 branch)",
    )
    .await;
    assert_eq!(mock.last_interrupt(), Some(sid("s-1")));
    // @step And the view stays the Agent view
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "the Esc interrupt branch must keep the Agent view (RPC-051 L4/L5)"
    );
    // @step And no exit confirmation dialog is shown
    assert!(
        !app.compositor().contains(EXIT_DIALOG_ID),
        "the running branch must NOT show the exit confirmation dialog"
    );
}

/// Scenario: Pressing Esc with a non-empty draft still clears the draft
/// (Esc cascade unchanged — R5).
#[tokio::test]
async fn scenario_pressing_esc_with_a_non_empty_draft_still_clears_the_draft_esc_cascade_unchanged()
{
    // @step Given the agent view shows 1 open idle session
    let (mut app, _mock) = fresh_app();
    seed_sessions(&mut app, 1).await;
    enter_agent_view(&mut app, "s-1").await;
    // @step And the input buffer contains a draft
    app.navigator_mut().agent.input.set_value("hello world");

    // @step When I press Esc in the Agent view
    app.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE));
    drain_pending(&mut app).await;

    // @step Then the input buffer is cleared
    assert_eq!(
        app.navigator().agent.input.value(),
        "",
        "the Esc L6 branch must clear the draft (RPC-095 L6, unchanged by BUG-204)"
    );
    // @step And the view stays the Agent view
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "the L6 branch must keep the Agent view"
    );
    // @step And no exit confirmation dialog is shown
    assert!(
        !app.compositor().contains(EXIT_DIALOG_ID),
        "a non-empty draft short-circuits before the dialog (RPC-095 L6)"
    );
}
