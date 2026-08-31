//! MUX-002 — Multiple agent panes with grouped agent-view cycling.
//!
//! Feature: spec/features/multiple-agent-panes-with-grouped-agent-view-cycling.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios.
//!
//! MUX-002 supersedes MUX-001's single-agent-pane assumption: the /mux
//! pane list defines FIXED SLOTS; agent slots form a WINDOW over the
//! ordered list of open agent sessions. Shift+Right at the right edge
//! prompts to create a new agent; rotation happens on the rightmost
//! agent pane when the window can advance.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::sync::Arc;

use codelet_fspec_tui::components::create_session_dialog::CREATE_SESSION_DIALOG_ID;
use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;
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

/// Open N sessions (s-1..s-N) and enable mux with the given pane list.
async fn app_with_sessions_and_panes(app: &mut App, n_sessions: usize, panes: &[MuxPaneKind]) {
    for i in 1..=n_sessions {
        app.dispatch(Action::SessionCreated(sid(&format!("s-{i}"))));
    }
    drain_pending(app).await;
    let kinds: Vec<&str> = panes
        .iter()
        .map(|k| match k {
            MuxPaneKind::Board => "board",
            MuxPaneKind::Agent => "agent",
            MuxPaneKind::ChangedFiles => "files",
            MuxPaneKind::Checkpoints => "checkpoints",
        })
        .collect();
    submit(app, &format!("/mux {}", kinds.join(" ")));
}

fn shift_right() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::SHIFT))
}

fn shift_left() -> Event {
    Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT))
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: unfilled agent slots are not rendered when fewer sessions are open
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: unfilled agent slots are not rendered when fewer sessions are open
#[tokio::test]
async fn unfilled_agent_slots_are_not_rendered_when_fewer_sessions_are_open() {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, _mock) = fresh_app();
    app_with_sessions_and_panes(
        &mut app,
        1,
        &[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    )
    .await;
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step And one agent session is open
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    // @step When the grid is rendered
    let buf = {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 24))
            .expect("Terminal::new");
        term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
            .expect("draw");
        term.backend().buffer().clone()
    };
    // @step Then the grid shows two panes: Board and the agent session
    // (the second agent slot is NOT rendered — only filled slots appear)
    let rects = app.navigator().mux.pane_rects();
    assert_eq!(
        rects.len(),
        2,
        "only the filled agent slot + board should be rendered (got {})",
        rects.len()
    );
    // @step And no blank or empty agent pane is rendered
    // (asserted via the rect count above — 2 rects, not 3)
    // @step And the Board pane takes the remaining width
    let board_w = rects[0].width;
    let agent_w = rects[1].width;
    assert!(
        board_w + agent_w >= 118,
        "board + agent should absorb the full width (board={board_w}, agent={agent_w})"
    );
    let _ = buf;
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Right at the right edge prompts to create a new agent
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Right at the right edge prompts to create a new agent
#[tokio::test]
async fn shift_right_at_the_right_edge_prompts_to_create_a_new_agent() {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, mock) = fresh_app();
    app_with_sessions_and_panes(
        &mut app,
        1,
        &[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    )
    .await;
    // @step And one agent session is open
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    // @step And the rightmost pane is focused
    let n = app.navigator().mux.pane_rects().len();
    app.navigator_mut().mux.set_focus(n - 1);
    // @step When I press Shift+Right and confirm the new-agent dialog
    let _ = app.handle_event(&shift_right());
    // @step Then the new-agent dialog is shown
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "Shift+Right at the right edge must open the new-agent dialog"
    );
    mock.script_create_session(sid("s-2"));
    app.dispatch(Action::CreateSessionSubmitted { isolated: false });
    drain_pending(&mut app).await;
    // @step And a second agent session is created WITHOUT work-unit attachment
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        2,
        "confirming the dialog must create a second session"
    );
    assert_eq!(
        app.agent_view_store().work_unit_context_for(&sid("s-2")),
        None,
        "the mux-created session must NOT carry a work-unit attachment"
    );
    // @step And the grid shows three panes: Board, agent 1 and agent 2
    let rects = app.navigator().mux.pane_rects();
    assert_eq!(
        rects.len(),
        3,
        "both agent slots must now be filled (got {})",
        rects.len()
    );
    // @step And the agent 2 pane is focused
    assert_eq!(
        app.navigator().mux.focus(),
        2,
        "focus must land on the new (agent 2) pane"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Right at the right edge prompts to create a new agent
// even when the rightmost pane is not an agent
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Right at the right edge prompts to create a new agent even when the rightmost pane is not an agent
#[tokio::test]
async fn shift_right_at_the_right_edge_prompts_even_when_rightmost_is_not_an_agent() {
    // @step Given mux mode is active with the pane list board, agent and files
    let (mut app, mock) = fresh_app();
    app_with_sessions_and_panes(
        &mut app,
        1,
        &[
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::ChangedFiles,
        ],
    )
    .await;
    // @step And one agent session is open
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    // @step And the files pane is focused
    let n = app.navigator().mux.pane_rects().len();
    app.navigator_mut().mux.set_focus(n - 1);
    // @step When I press Shift+Right and confirm the new-agent dialog
    let _ = app.handle_event(&shift_right());
    // @step Then the new-agent dialog is shown
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "Shift+Right at the right edge (files pane) must open the new-agent dialog"
    );
    mock.script_create_session(sid("s-2"));
    app.dispatch(Action::CreateSessionSubmitted { isolated: false });
    drain_pending(&mut app).await;
    // @step And a second agent session is created WITHOUT work-unit attachment
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        2,
        "confirming the dialog must create a second session"
    );
    assert_eq!(
        app.agent_view_store().work_unit_context_for(&sid("s-2")),
        None,
        "the mux-created session must NOT carry a work-unit attachment"
    );
    // @step And the agent window advances so the new session fills the last agent slot
    // (with one agent slot, the window shows the newest session)
    // @step And the new agent pane is focused
    let focus = app.navigator().mux.focus();
    let kind = app.navigator().mux.config().panes.get(focus).copied();
    assert!(
        matches!(kind, Some(MuxPaneKind::Agent)),
        "focus must land on an agent pane (got {kind:?})"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Right rotates the agent window forward when the window
// can advance
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Right rotates the agent window forward when the window can advance
#[tokio::test]
async fn shift_right_rotates_the_agent_window_forward_when_the_window_can_advance() {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, _mock) = fresh_app();
    app_with_sessions_and_panes(
        &mut app,
        3,
        &[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    )
    .await;
    // @step And three agent sessions are open
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);
    // @step And the grid shows Board, agent 1 and agent 2
    let before = app.navigator().mux.window_session_ids();
    assert_eq!(
        before,
        vec![sid("s-1"), sid("s-2")],
        "window must start at [s-1, s-2] (got {before:?})"
    );
    // @step And the rightmost agent pane is focused
    let n = app.navigator().mux.pane_rects().len();
    app.navigator_mut().mux.set_focus(n - 1);
    // @step When I press Shift+Right
    let _ = app.handle_event(&shift_right());
    // @step Then the grid shows Board, agent 2 and agent 3
    let after = app.navigator().mux.window_session_ids();
    assert_eq!(
        after,
        vec![sid("s-2"), sid("s-3")],
        "window must advance to [s-2, s-3] (got {after:?})"
    );
    // @step And the Board pane never moved or changed
    assert_eq!(
        app.navigator().mux.config().panes[0],
        MuxPaneKind::Board,
        "board pane must stay pinned at slot 0"
    );
    // @step And the rightmost agent pane is still focused
    assert_eq!(
        app.navigator().mux.focus(),
        n - 1,
        "focus must stay on the rightmost pane"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Left on the rightmost agent pane rotates the agent
// window backward
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Left on the rightmost agent pane rotates the agent window backward
#[tokio::test]
async fn shift_left_on_the_rightmost_agent_pane_rotates_the_agent_window_backward() {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, _mock) = fresh_app();
    app_with_sessions_and_panes(
        &mut app,
        3,
        &[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    )
    .await;
    // @step And three agent sessions are open
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);
    // @step And the grid shows Board, agent 2 and agent 3
    // (advance the window to [s-2, s-3] first)
    let n = app.navigator().mux.pane_rects().len();
    app.navigator_mut().mux.set_focus(n - 1);
    let _ = app.handle_event(&shift_right());
    assert_eq!(
        app.navigator().mux.window_session_ids(),
        vec![sid("s-2"), sid("s-3")],
        "window must be at [s-2, s-3] before the backward test"
    );
    // @step And the rightmost agent pane is focused
    app.navigator_mut().mux.set_focus(n - 1);
    // @step When I press Shift+Left
    let _ = app.handle_event(&shift_left());
    // @step Then the grid shows Board, agent 1 and agent 2
    let after = app.navigator().mux.window_session_ids();
    assert_eq!(
        after,
        vec![sid("s-1"), sid("s-2")],
        "window must rotate back to [s-1, s-2] (got {after:?})"
    );
    // @step And the rightmost agent pane is still focused
    assert_eq!(
        app.navigator().mux.focus(),
        n - 1,
        "focus must stay on the rightmost pane"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Right on the rightmost files pane at the last window
// position prompts to create a new agent
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Right on the rightmost files pane at the last window position prompts to create a new agent
#[tokio::test]
async fn shift_right_on_the_rightmost_files_pane_at_the_last_window_position_prompts() {
    // @step Given mux mode is active with the pane list board, agent, agent and files
    let (mut app, mock) = fresh_app();
    app_with_sessions_and_panes(
        &mut app,
        3,
        &[
            MuxPaneKind::Board,
            MuxPaneKind::Agent,
            MuxPaneKind::Agent,
            MuxPaneKind::ChangedFiles,
        ],
    )
    .await;
    // @step And three agent sessions are open
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);
    // @step And the agent window shows agent 2 and agent 3
    // (advance the window to the last position [s-2, s-3])
    let n = app.navigator().mux.pane_rects().len();
    app.navigator_mut().mux.set_focus(n - 2); // rightmost AGENT pane
    let _ = app.handle_event(&shift_right());
    assert_eq!(
        app.navigator().mux.window_session_ids(),
        vec![sid("s-2"), sid("s-3")],
        "window must be at [s-2, s-3] before the prompt test"
    );
    // @step And the files pane is focused
    app.navigator_mut().mux.set_focus(n - 1);
    // @step When I press Shift+Right and confirm the new-agent dialog
    let _ = app.handle_event(&shift_right());
    // @step Then the new-agent dialog is shown
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "Shift+Right at the last window position must open the new-agent dialog"
    );
    mock.script_create_session(sid("s-4"));
    app.dispatch(Action::CreateSessionSubmitted { isolated: false });
    drain_pending(&mut app).await;
    // @step And a fourth agent session is created WITHOUT work-unit attachment
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        4,
        "confirming the dialog must create a fourth session"
    );
    assert_eq!(
        app.agent_view_store().work_unit_context_for(&sid("s-4")),
        None,
        "the mux-created session must NOT carry a work-unit attachment"
    );
    // @step And the agent window shows agent 3 and agent 4
    let after = app.navigator().mux.window_session_ids();
    assert_eq!(
        after,
        vec![sid("s-3"), sid("s-4")],
        "window must advance to [s-3, s-4] (got {after:?})"
    );
    // @step And the agent 4 pane is focused
    let focus = app.navigator().mux.focus();
    let kind = app.navigator().mux.config().panes.get(focus).copied();
    assert!(
        matches!(kind, Some(MuxPaneKind::Agent)),
        "focus must land on the new agent pane (got {kind:?})"
    );
    // @step And the files pane stays pinned in its slot
    assert_eq!(
        app.navigator().mux.config().panes[n - 1],
        MuxPaneKind::ChangedFiles,
        "files pane must stay pinned at the last slot"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Right on the rightmost agent pane at the last window
// position prompts to create a new agent
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Right on the rightmost agent pane at the last window position prompts to create a new agent
#[tokio::test]
async fn shift_right_on_the_rightmost_agent_pane_at_the_last_window_position_prompts() {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, mock) = fresh_app();
    app_with_sessions_and_panes(
        &mut app,
        3,
        &[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    )
    .await;
    // @step And three agent sessions are open
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);
    // @step And the agent window shows agent 2 and agent 3
    let n = app.navigator().mux.pane_rects().len();
    app.navigator_mut().mux.set_focus(n - 1);
    let _ = app.handle_event(&shift_right());
    assert_eq!(
        app.navigator().mux.window_session_ids(),
        vec![sid("s-2"), sid("s-3")],
        "window must be at [s-2, s-3] before the prompt test"
    );
    // @step And the rightmost agent pane is focused
    app.navigator_mut().mux.set_focus(n - 1);
    // @step When I press Shift+Right and confirm the new-agent dialog
    let _ = app.handle_event(&shift_right());
    // @step Then the new-agent dialog is shown
    assert!(
        app.compositor().contains(CREATE_SESSION_DIALOG_ID),
        "Shift+Right at the last window position must open the new-agent dialog"
    );
    mock.script_create_session(sid("s-4"));
    app.dispatch(Action::CreateSessionSubmitted { isolated: false });
    drain_pending(&mut app).await;
    // @step And a fourth agent session is created WITHOUT work-unit attachment
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        4,
        "confirming the dialog must create a fourth session"
    );
    assert_eq!(
        app.agent_view_store().work_unit_context_for(&sid("s-4")),
        None,
        "the mux-created session must NOT carry a work-unit attachment"
    );
    // @step And the agent window shows agent 3 and agent 4
    let after = app.navigator().mux.window_session_ids();
    assert_eq!(
        after,
        vec![sid("s-3"), sid("s-4")],
        "window must advance to [s-3, s-4] (got {after:?})"
    );
    // @step And the agent 4 pane is focused
    let focus = app.navigator().mux.focus();
    let kind = app.navigator().mux.config().panes.get(focus).copied();
    assert!(
        matches!(kind, Some(MuxPaneKind::Agent)),
        "focus must land on the new agent pane (got {kind:?})"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Left at the first pane stops without wrapping
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Left at the first pane stops without wrapping
#[tokio::test]
async fn shift_left_at_the_first_pane_stops_without_wrapping() {
    // @step Given mux mode is active with the pane list board and agent
    let (mut app, _mock) = fresh_app();
    app_with_sessions_and_panes(&mut app, 1, &[MuxPaneKind::Board, MuxPaneKind::Agent]).await;
    // @step And the Board pane is focused
    app.navigator_mut().mux.set_focus(0);
    // @step When I press Shift+Left
    let _ = app.handle_event(&shift_left());
    // @step Then the Board pane is still focused
    assert_eq!(
        app.navigator().mux.focus(),
        0,
        "Shift+Left at the first pane must NOT wrap (focus stayed at 0)"
    );
    // @step And the focus did not wrap to the rightmost pane
    assert_ne!(
        app.navigator().mux.focus(),
        1,
        "focus must not wrap to the rightmost pane"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Left falls through to focus movement when the window
// cannot rotate backward
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Left falls through to focus movement when the window cannot rotate backward
#[tokio::test]
async fn shift_left_falls_through_to_focus_movement_when_the_window_cannot_rotate_backward() {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, _mock) = fresh_app();
    app_with_sessions_and_panes(
        &mut app,
        2,
        &[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    )
    .await;
    // @step And two agent sessions are open
    assert_eq!(app.agent_view_store().open_sessions().len(), 2);
    // @step And the grid shows Board, agent 1 and agent 2
    assert_eq!(
        app.navigator().mux.window_session_ids(),
        vec![sid("s-1"), sid("s-2")],
        "window must start at [s-1, s-2]"
    );
    // @step And the rightmost agent pane is focused
    let n = app.navigator().mux.pane_rects().len();
    app.navigator_mut().mux.set_focus(n - 1);
    // @step When I press Shift+Left twice
    let _ = app.handle_event(&shift_left());
    // @step Then the agent 1 pane is focused after the first press
    assert_eq!(
        app.navigator().mux.focus(),
        1,
        "Shift+Left must fall through to focus movement (focus on agent 1 pane)"
    );
    // @step And the agent window did not rotate
    assert_eq!(
        app.navigator().mux.window_session_ids(),
        vec![sid("s-1"), sid("s-2")],
        "window must not rotate (still [s-1, s-2])"
    );
    let _ = app.handle_event(&shift_left());
    // @step And the Board pane is focused after the second press
    assert_eq!(
        app.navigator().mux.focus(),
        0,
        "second Shift+Left must move focus to the Board pane"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: closing an agent session keeps the agent slots and re-clamps
// the window
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: closing an agent session keeps the agent slots and re-clamps the window
#[tokio::test]
async fn closing_an_agent_session_keeps_the_agent_slots_and_re_clamps_the_window() {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, _mock) = fresh_app();
    app_with_sessions_and_panes(
        &mut app,
        3,
        &[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    )
    .await;
    // @step And three agent sessions are open
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);
    // @step And the grid shows Board, agent 2 and agent 3
    let n = app.navigator().mux.pane_rects().len();
    app.navigator_mut().mux.set_focus(n - 1);
    let _ = app.handle_event(&shift_right());
    assert_eq!(
        app.navigator().mux.window_session_ids(),
        vec![sid("s-2"), sid("s-3")],
        "window must be at [s-2, s-3] before the close"
    );
    // @step When the agent 2 session is closed
    app.dispatch(Action::ConfirmDeleteSession(sid("s-2")));
    drain_pending(&mut app).await;
    // @step Then the grid still shows the Board pane and the agent slots
    let rects = app.navigator().mux.pane_rects();
    assert!(
        rects.len() >= 2,
        "board + agent slots must remain after a session close (got {})",
        rects.len()
    );
    // @step And the agent window re-clamps to the remaining sessions
    let ids = app.navigator().mux.window_session_ids();
    assert!(
        ids.iter().all(|s| s != &sid("s-2")),
        "window must not reference the closed session s-2 (got {ids:?})"
    );
    // @step And no pane is removed from the layout
    assert_eq!(
        app.navigator().mux.config().panes.len(),
        3,
        "the pane list must keep all 3 slots (board, agent, agent)"
    );
}
