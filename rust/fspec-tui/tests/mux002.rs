//! MUX-002 — Multiple agent panes with grouped agent-view cycling.
//!
//! Feature: spec/features/multiple-agent-panes-with-grouped-agent-view-cycling.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios.
//!
//! MUX-002 supersedes MUX-001's single-agent-pane assumption: the /mux
//! pane list defines FIXED SLOTS; agent slots form a WINDOW over the
//! ordered list of open agent sessions. Shift+Right on a last agent
//! pane prompts to create a new agent (MUX-012: Shift+Left/Right move
//! in layout order and never rotate the window; their MUX-012
//! scenarios are in mux012_layout_order_focus.rs).

#![allow(clippy::too_many_lines)]

use crate::common::TaskOutcome;
use codelet_test_helpers::{OrFail, TestResult};
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

fn submit(app: &mut App, text: &str) {
    if let Some(session) = app.current_session() {
        app.dispatch(Action::InputSubmitted(session, text.to_string()));
    }
}

/// Open N sessions (s-1..s-N) and enable mux with the given pane list.
async fn app_with_sessions_and_panes(
    app: &mut App,
    n_sessions: usize,
    panes: &[MuxPaneKind],
) -> Result<(), codelet_test_helpers::TestError> {
    for i in 1..=n_sessions {
        app.dispatch(Action::SessionCreated(sid(&format!("s-{i}"))));
    }
    drain_pending(app).await?;
    let kinds: Vec<&str> = panes
        .iter()
        .map(|k| match k {
            MuxPaneKind::Board => "board",
            MuxPaneKind::Agent => "agent",
            MuxPaneKind::ChangedFiles => "files",
            MuxPaneKind::Checkpoints => "checkpoints",
        })
        .collect();
    submit(app, &format!("/mux {}", kinds.join(", ")));
    Ok(())
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
async fn unfilled_agent_slots_are_not_rendered_when_fewer_sessions_are_open() -> TestResult {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, _mock) = fresh_app();
    app_with_sessions_and_panes(
        &mut app,
        1,
        &[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    )
    .await?;
    assert_eq!(app.active_view(), ViewMode::Mux);
    // @step And one agent session is open
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    // @step When the grid is rendered
    let buf = {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 24))
            .or_fail("Terminal::new")?;
        term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
            .or_fail("draw")?;
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Right at the right edge prompts to create a new agent
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Right at the right edge prompts to create a new agent
#[tokio::test]
async fn shift_right_at_the_right_edge_prompts_to_create_a_new_agent() -> TestResult {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, mock) = fresh_app();
    app_with_sessions_and_panes(
        &mut app,
        1,
        &[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    )
    .await?;
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
    drain_pending(&mut app).await?;
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Shift+Left at the first pane stops without wrapping
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Shift+Left at the first pane stops without wrapping
#[tokio::test]
async fn shift_left_at_the_first_pane_stops_without_wrapping() -> TestResult {
    // @step Given mux mode is active with the pane list board and agent
    let (mut app, _mock) = fresh_app();
    app_with_sessions_and_panes(&mut app, 1, &[MuxPaneKind::Board, MuxPaneKind::Agent]).await?;
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: closing an agent session keeps the agent slots and re-clamps
// the window
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: closing an agent session keeps the agent slots and re-clamps the window
#[tokio::test]
async fn closing_an_agent_session_keeps_the_agent_slots_and_re_clamps_the_window() -> TestResult {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, _mock) = fresh_app();
    app_with_sessions_and_panes(
        &mut app,
        3,
        &[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    )
    .await?;
    // @step And three agent sessions are open
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);
    // @step And the grid shows Board, agent 2 and agent 3
    // MUX-012: Shift+arrows no longer rotate the window; the chip path
    // (MUX-004) puts agent 3 in the last slot.
    let _ = app
        .navigator_mut()
        .mux
        .focus_agent_pane_for_session(&sid("s-3"));
    assert_eq!(
        app.navigator().mux.window_session_ids(),
        vec![sid("s-2"), sid("s-3")],
        "window must be at [s-2, s-3] before the close"
    );
    // @step When the agent 2 session is closed
    app.dispatch(Action::ConfirmDeleteSession(sid("s-2")));
    drain_pending(&mut app).await?;
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
        app.navigator().mux.config().layout.leaves().len(),
        3,
        "the pane list must keep all 3 slots (board, agent, agent)"
    );
    Ok(())
}
