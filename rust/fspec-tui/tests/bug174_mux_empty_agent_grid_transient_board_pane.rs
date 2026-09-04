//! BUG-174 — mux: closing the last agent in an all-agent layout must not
//! leave a blank 0-pane screen.
//!
//! Feature: spec/features/mux-empty-agent-grid-transient-board-pane.feature
//!
//! This test file validates the acceptance criteria defined in the
//! feature file. Scenarios map directly to Gherkin scenarios.
//!
//! Root cause (log evidence: ~/.fspec/logs/fspec-combined.log.2026-09-04
//! 09:55:51-09:56:07, screenshot ~/mux.png): with a saved mux config of
//! panes=[Agent, Agent] (no board pane), `recompute_effective_panes`
//! drops every agent slot once the open-session list empties — the
//! rendered pane list becomes EMPTY. The render pass then paints only the
//! 1-row MUX footer ("MUX 0 panes []"); the keyboard is dead
//! (`forward_mux_event_to_focused_pane` swallows every key at
//! `focus 0 >= 0 panes`, and the BUG-165 stage-4 guard needs a Board
//! pane at the focus index — none exists). Only Ctrl+D gets through.
//!
//! Fix shape (spec rules 1-6): a single full-width TRANSIENT Board pane
//! stands in for an empty rendered pane list (live-only — the stored
//! config is never mutated). The grid never renders zero panes, the
//! board is usable, and Esc / Shift+Right / Enter all behave.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::components::board_exit_confirmation_dialog::BOARD_EXIT_CONFIRMATION_DIALOG_ID;
use codelet_fspec_tui::components::create_session_dialog::CREATE_SESSION_DIALOG_ID;
use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, WorkUnitInfo};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

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

fn submit(app: &mut App, text: &str) {
    app.dispatch(Action::InputSubmitted(text.to_string()));
}

fn esc() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
}

fn enter() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
}

fn shift_right() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT))
}

/// One backlog work unit so the board pane has something to render.
fn wu(id: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.to_string(),
        title: id.to_string(),
        work_type: "story".to_string(),
        status: "backlog".to_string(),
        description: None,
        estimate: None,
        epic: None,
        attachments: Vec::new(),
        last_state_change_at: None,
    }
}

/// Enter mux mode with the all-agent layout `[Agent, Agent]` (no board
/// pane) and `n_sessions` open sessions. This is the user's saved
/// config shape that traps the TUI (BUG-174).
async fn all_agent_mux(app: &mut App, n_sessions: usize) {
    for i in 1..=n_sessions {
        app.dispatch(Action::SessionCreated(sid(&format!("s-{i}"))));
    }
    drain_pending(app).await;
    app.dispatch(Action::WorkUnitsLoaded(vec![wu("AUTH-001")]));
    // Explicit all-agent pane list (MUX-001 R5): 2..=4 kinds, no split.
    submit(app, "/mux agent agent");
    drain_pending(app).await;
}

/// The realistic trap path: open one agent, then Close Session.
async fn open_one_then_close(app: &mut App) {
    all_agent_mux(app, 1).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    // Focus the (only) agent pane, then close it via the exit dialog.
    let n = app.navigator().mux.pane_rects().len();
    app.navigator_mut().mux.set_focus(n - 1);
    app.dispatch(Action::AgentExitChoice {
        choice: codelet_fspec_tui::components::exit_confirmation_dialog::ExitChoice::CloseSession,
    });
    drain_pending(app).await;
}

/// The same trap for the DEFAULT (Board|Agent) layout — the pre-existing
/// BUG-165 shape. Slash submits are silently dropped without a session
/// (`handle_input_submitted`), so the layout must be entered while a
/// session is live and that session closed afterwards.
async fn default_layout_then_close(app: &mut App) {
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::WorkUnitsLoaded(vec![wu("AUTH-001")]));
    drain_pending(app).await;
    submit(app, "/mux on");
    drain_pending(app).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    app.dispatch(Action::AgentExitChoice {
        choice: codelet_fspec_tui::components::exit_confirmation_dialog::ExitChoice::CloseSession,
    });
    drain_pending(app).await;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: closing the last agent in an all-agent layout shows a
// full-width board pane
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: closing the last agent in an all-agent layout shows a full-width board pane
#[tokio::test]
async fn closing_the_last_agent_in_an_all_agent_layout_shows_a_full_width_board_pane() {
    // @step Given mux mode is active with the pane list agent and agent and no board pane
    // @step And one agent session is open
    let (mut app, _mock) = fresh_app();
    open_one_then_close(&mut app).await;
    // @step When the agent session is closed with Close Session
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        0,
        "Close Session must empty the open-session list"
    );
    // @step Then the TUI is still in mux mode with mux enabled
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "BUG-164: closing a session must NOT flip the view out of the mux grid"
    );
    assert!(
        app.navigator().mux.config().enabled,
        "the mux config must stay enabled"
    );
    // @step And the grid renders exactly one pane: a full-width Board pane
    let panes = app.navigator().mux.effective_panes();
    assert_eq!(
        panes.len(),
        1,
        "an all-agent layout with zero sessions must render the transient Board stand-in, not 0 panes (got {panes:?})"
    );
    assert_eq!(
        panes[0],
        MuxPaneKind::Board,
        "the transient stand-in must be a Board pane"
    );
    let rects = app.navigator().mux.pane_rects();
    assert_eq!(rects.len(), 1, "exactly one pane rect must be cached");
    let full = rects[0].width;
    // 120-col terminal seeded body (no divider for a single pane).
    assert!(
        full >= 118,
        "the transient board pane must take the full body width (got {full})"
    );
    // @step And no blank body is painted above the MUX footer row
    // (asserted via the cached rect: the board pane covers the body)
    // @step And the board work units are visible in the board pane
    let buf = {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 24))
            .expect("Terminal::new");
        term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
            .expect("draw");
        term.backend().buffer().clone()
    };
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    assert!(
        joined.contains("AUTH-001"),
        "the board pane must paint the work unit (blank body means the trap)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the transient board pane does not mutate the stored mux config
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the transient board pane does not mutate the stored mux config
#[tokio::test]
async fn the_transient_board_pane_does_not_mutate_the_stored_mux_config() {
    // @step Given mux mode is active with the pane list agent and agent and no board pane
    // @step And one agent session is open
    let (mut app, _mock) = fresh_app();
    open_one_then_close(&mut app).await;
    // @step When the agent session is closed with Close Session
    // (done in the helper)
    // @step Then the stored mux config still lists exactly the panes agent and agent
    let stored = app.navigator().mux.config().panes.clone();
    assert_eq!(
        stored,
        vec![MuxPaneKind::Agent, MuxPaneKind::Agent],
        "the stored config must keep the user's all-agent layout (got {stored:?})"
    );
    // @step And the stored config keeps its orientation and focused pane unchanged
    // (orientation Horizontal + focused_pane preserved by the layout)
    assert!(
        app.navigator().mux.config().enabled,
        "enabled must stay true"
    );
    // @step And the rendered pane list (transient board stand-in) differs from the stored config
    let rendered = app.navigator().mux.effective_panes();
    assert_eq!(
        rendered,
        &[MuxPaneKind::Board],
        "the rendered list must be the transient board stand-in, not the stored all-agent list"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: pressing Esc on the transient board pane shows the exit dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: pressing Esc on the transient board pane shows the exit dialog
#[tokio::test]
async fn pressing_esc_on_the_transient_board_pane_shows_the_exit_dialog() {
    // @step Given mux mode is active with the pane list agent and agent and no board pane
    // @step And no agent sessions are open
    let (mut app, _mock) = fresh_app();
    open_one_then_close(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    assert_eq!(app.agent_view_store().open_sessions().len(), 0);
    assert_eq!(
        app.navigator().mux.effective_panes(),
        &[MuxPaneKind::Board],
        "zero sessions on an all-agent layout must yield the transient board pane"
    );
    assert_eq!(app.compositor().len(), 0, "no dialog may be open yet");
    // @step When I press the Esc key
    let _ = app.handle_event(&esc());
    drain_pending(&mut app).await;
    // @step Then the BoardExitConfirmationDialog is shown over the full screen
    assert!(
        app.compositor().contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID),
        "Esc on the transient board pane must push the BoardExitConfirmationDialog (the pre-fix state swallowed every key)"
    );
    assert_eq!(
        app.compositor().topmost_id(),
        Some(BOARD_EXIT_CONFIRMATION_DIALOG_ID.to_string()),
        "the board exit confirmation dialog must be the topmost layer"
    );
    // @step And the application does not quit directly
    assert!(
        !app.should_quit(),
        "Esc must NOT quit directly — the confirmation dialog must appear first"
    );
    // @step And confirming the Exit option quits the application
    let _ = app.handle_event(&enter());
    drain_pending(&mut app).await;
    assert!(
        app.should_quit(),
        "Enter on the pre-selected 'Exit' option must set should_quit=true"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Right on the transient board pane opens the new-agent dialog
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Right on the transient board pane opens the new-agent dialog
#[tokio::test]
async fn shift_right_on_the_transient_board_pane_opens_the_new_agent_dialog() {
    // @step Given mux mode is active with the pane list agent and agent and no board pane
    // @step And no agent sessions are open
    let (mut app, mock) = fresh_app();
    open_one_then_close(&mut app).await;
    assert_eq!(app.navigator().mux.effective_panes(), &[MuxPaneKind::Board]);
    // @step When I press Shift+Right
    let _ = app.handle_event(&shift_right());
    drain_pending(&mut app).await;
    // @step Then the new-agent CreateSessionDialog is shown
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "Shift+Right on the rightmost (transient board) pane must open the new-agent dialog (pre-fix it was consumed as a dead key)"
    );
    // @step And confirming it creates an agent session
    mock.script_create_session(sid("s-new"));
    app.dispatch(Action::CreateSessionSubmitted { isolated: false });
    drain_pending(&mut app).await;
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        1,
        "confirming the dialog must create one session"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: creating a session restores the configured agent panes
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: creating a session restores the configured agent panes
#[tokio::test]
async fn creating_a_session_restores_the_configured_agent_panes() {
    // @step Given mux mode is active with the pane list agent and agent and no board pane
    // @step And no agent sessions are open
    let (mut app, mock) = fresh_app();
    open_one_then_close(&mut app).await;
    assert_eq!(app.navigator().mux.effective_panes(), &[MuxPaneKind::Board]);
    // @step When I press Shift+Right and confirm the new-agent dialog
    let _ = app.handle_event(&shift_right());
    drain_pending(&mut app).await;
    mock.script_create_session(sid("s-2"));
    app.dispatch(Action::CreateSessionSubmitted { isolated: false });
    drain_pending(&mut app).await;
    // @step Then the grid shows exactly one agent pane (the transient board pane is gone; the still-unfilled second agent slot is dropped per MUX-002 rule 3)
    let panes = app.navigator().mux.effective_panes();
    assert_eq!(
        panes,
        &[MuxPaneKind::Agent],
        "one session + two agent slots renders exactly one agent pane (got {panes:?})"
    );
    // @step And no board pane is rendered
    assert!(
        !panes.contains(&MuxPaneKind::Board),
        "the transient board stand-in must vanish once an agent slot is filled"
    );
    // @step And the new agent pane is focused
    assert_eq!(
        app.navigator().mux.effective_panes()[app.navigator().mux.focus()],
        MuxPaneKind::Agent,
        "focus must land on an agent pane after the layout restores"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: the default board agent layout with zero sessions renders one board pane
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: the default board agent layout with zero sessions renders one board pane
#[tokio::test]
async fn the_default_board_agent_layout_with_zero_sessions_renders_one_board_pane() {
    // @step Given mux mode is active with the default Board and Agent panes
    let (mut app, _mock) = fresh_app();
    default_layout_then_close(&mut app).await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step And no agent sessions are open
    assert_eq!(app.agent_view_store().open_sessions().len(), 0);
    // @step Then the grid renders exactly one pane: the configured Board pane
    let panes = app.navigator().mux.effective_panes();
    assert_eq!(
        panes,
        &[MuxPaneKind::Board],
        "a configured Board|Agent layout with zero sessions renders exactly the Board pane (pre-existing BUG-165 shape — must stay unchanged)"
    );
    // @step And the MUX footer shows one pane: Board
    let buf = {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 24))
            .expect("Terminal::new");
        term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
            .expect("draw");
        term.backend().buffer().clone()
    };
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    assert!(
        joined.contains("MUX 1 panes [Board]"),
        "the footer must report a single Board pane (regression guard for the transient stand-in)"
    );
    // @step And the stored config is unchanged (Board and Agent)
    assert_eq!(
        app.navigator().mux.config().panes,
        vec![MuxPaneKind::Board, MuxPaneKind::Agent],
        "the stored default config must remain Board and Agent"
    );
}
