//! BUG-163 — Mux agent panes render distinct window sessions.
//!
//! Feature: spec/features/mux-agent-panes-render-distinct-window-sessions.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios.
//!
//! The MUX-002 window math (window_start / window_session_ids) was applied
//! to the pane SLOTS only — `AgentView::render_with_store` always painted
//! `store.current_session()`, so with `/mux board agent agent` the second
//! agent pane duplicated the focused session (identical header, scrollback,
//! input). These tests render the real grid into a TestBackend buffer and
//! assert each agent pane paints ITS window session.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::sync::Arc;

use codelet_fspec_tui::views::multiplex::MuxPaneKind;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

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

/// Focus the Nth (0-based) rendered pane by clicking inside its cached
/// rect — the production click-to-focus path (Navigator → mux_mouse).
fn click_pane(app: &mut App, pane_index: usize) {
    let rect = app
        .navigator()
        .mux
        .pane_rects()
        .get(pane_index)
        .copied()
        .expect("pane rect must exist");
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x + 1,
        row: rect.y + 1,
        modifiers: KeyModifiers::NONE,
    });
    let _ = app.handle_event(&event);
}

/// Render the app into a 160x30 TestBackend buffer.
fn render_buf(app: &mut App) -> (ratatui::buffer::Buffer, ratatui::layout::Rect) {
    let mut term =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(160, 30)).expect("Terminal::new");
    term.draw(|frame| app.render(frame.area(), frame.buffer_mut()))
        .expect("draw");
    (
        term.backend().buffer().clone(),
        ratatui::layout::Rect::new(0, 0, 160, 30),
    )
}

/// The cells of one full terminal row (0..159) as one string.
fn buf_row(buf: &ratatui::buffer::Buffer, y: u16, area: ratatui::layout::Rect) -> String {
    (0..area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect()
}

/// All rows of a pane rect, joined by '\n'.
fn pane_text(buf: &ratatui::buffer::Buffer, pane: ratatui::layout::Rect) -> String {
    (0..pane.height)
        .map(|i| {
            (pane.x..pane.x + pane.width)
                .map(|x| buf[(x, pane.y + i)].symbol().to_string())
                .collect::<String>()
        })
        .collect()
}

/// The text painted at the top-left of an agent pane (its header cell).
/// Reads cells directly (not a byte-slice of a joined row string) so
/// multi-byte glyphs like the `│` divider never break char boundaries.
fn pane_top_left(buf: &ratatui::buffer::Buffer, pane_index: usize, app: &App) -> String {
    let pane = app
        .navigator()
        .mux
        .pane_rects()
        .get(pane_index)
        .copied()
        .expect("pane rect");
    let end = (pane.x + 40).min(160);
    (pane.x..end)
        .map(|x| buf[(x, pane.y)].symbol().to_string())
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: each agent pane paints the session at its window slot position
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: each agent pane paints the session at its window slot position
#[tokio::test]
async fn each_agent_pane_paints_the_session_at_its_window_slot_position() {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, _mock) = fresh_app();
    app_with_sessions_and_panes(
        &mut app,
        2,
        &[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    )
    .await;
    // @step And two agent sessions are open with distinct scrollback content
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::text("alpha-message".to_string()),
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-2"),
        StreamChunk::text("beta-message".to_string()),
    ));
    // @step And the agent window shows agent 1 and agent 2
    assert_eq!(
        app.navigator().mux.window_session_ids(),
        vec![sid("s-1"), sid("s-2")],
    );
    // @step When the grid is rendered
    let (buf, _area) = render_buf(&mut app);
    // @step Then the left agent pane header shows agent 1's session index
    let left = pane_top_left(&buf, 1, &app);
    assert!(
        left.contains("#1"),
        "left agent pane header must show agent 1's #1 (got {left:?})"
    );
    // @step And the right agent pane header shows agent 2's session index
    let right = pane_top_left(&buf, 2, &app);
    assert!(
        right.contains("#2"),
        "right agent pane header must show agent 2's #2 (got {right:?})"
    );
    // @step And the left agent pane scrollback shows agent 1's messages
    let left_pane = app.navigator().mux.pane_rects()[1];
    let left_text = pane_text(&buf, left_pane);
    assert!(
        left_text.contains("alpha-message"),
        "left agent pane must show agent 1's scrollback (alpha-message); got: {left_text}"
    );
    // @step And the right agent pane scrollback shows agent 2's messages
    let right_pane = app.navigator().mux.pane_rects()[2];
    let right_text = pane_text(&buf, right_pane);
    assert!(
        right_text.contains("beta-message"),
        "right agent pane must show agent 2's scrollback (beta-message); got: {right_text}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: no two agent panes render the same session
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: no two agent panes render the same session
#[tokio::test]
async fn no_two_agent_panes_render_the_same_session() {
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
    // @step When the grid is rendered
    let (buf, _area) = render_buf(&mut app);
    // @step Then the two agent panes render different sessions
    let left = pane_top_left(&buf, 1, &app);
    let right = pane_top_left(&buf, 2, &app);
    assert_ne!(
        left, right,
        "the two agent panes must render DIFFERENT sessions (both showed {left:?})"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: an unfilled agent slot shows a single agent pane with the
// live composer
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: an unfilled agent slot shows a single agent pane with the live composer
#[tokio::test]
async fn an_unfilled_agent_slot_shows_a_single_agent_pane_with_the_live_composer() {
    // @step Given mux mode is active with the pane list board, agent and agent
    let (mut app, _mock) = fresh_app();
    app_with_sessions_and_panes(
        &mut app,
        1,
        &[MuxPaneKind::Board, MuxPaneKind::Agent, MuxPaneKind::Agent],
    )
    .await;
    // @step And one agent session is open
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    // @step And the agent pane is focused
    click_pane(&mut app, 1);
    assert_eq!(app.navigator().mux.focus(), 1);
    // @step When the grid is rendered
    let (buf, _area) = render_buf(&mut app);
    // @step Then exactly one agent pane is rendered
    assert_eq!(
        app.navigator().mux.effective_panes(),
        &[MuxPaneKind::Board, MuxPaneKind::Agent],
        "the second (unfilled) agent slot must not be rendered"
    );
    // @step And the agent pane header shows the open session's model and tokens
    let agent = pane_top_left(&buf, 1, &app);
    assert!(
        agent.contains("#1"),
        "the single agent pane header must show the open session (got {agent:?})"
    );
    // @step And the agent pane input shows the live composer
    // The live composer is the LAST row of the agent pane (the agent
    // layout is Header/Role/Scrollback/Footer/Input — input bottom).
    let rect = app.navigator().mux.pane_rects()[1];
    let input_row = buf_row(
        &buf,
        rect.y + rect.height - 1,
        ratatui::layout::Rect::new(0, 0, 160, 30),
    );
    assert!(
        input_row.contains("Type a message..."),
        "the focused agent pane must paint the live composer (got {input_row:?})"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: moving focus between agent panes keeps the unfocused pane's
// draft visible
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: moving focus between agent panes keeps the unfocused pane's draft visible
#[tokio::test]
async fn moving_focus_between_agent_panes_keeps_the_unfocused_pane_draft_visible() {
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
    // @step And the agent 1 pane is focused
    click_pane(&mut app, 1);
    assert_eq!(app.navigator().mux.focus(), 1);
    // @step And the focused agent composer holds the draft "hello"
    for ch in "hello".chars() {
        let _ = app.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char(ch),
            KeyModifiers::NONE,
        )));
    }
    app.dispatch(Action::PendingInputChanged("hello".to_string()));
    drain_pending(&mut app).await;
    // @step When focus moves to the agent 2 pane
    click_pane(&mut app, 2);
    assert_eq!(app.navigator().mux.focus(), 2);
    // @step Then the agent 1 pane still shows the draft "hello" as its ghost input
    let (buf, _area) = render_buf(&mut app);
    let left_pane = app.navigator().mux.pane_rects()[1];
    let left_input = pane_text(&buf, left_pane);
    assert!(
        left_input.contains("hello"),
        "the unfocused agent 1 pane must keep its ghost draft 'hello' visible; pane: {left_input}"
    );
    // @step And the agent 2 pane shows the live composer
    let right_pane = app.navigator().mux.pane_rects()[2];
    let right_input = pane_text(&buf, right_pane);
    assert!(
        right_input.contains("Type a message..."),
        "the focused agent 2 pane must show the live composer (empty draft placeholder); pane: {right_input}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: an unfocused agent pane renders its ghost draft instead of a
// live spinner
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: an unfocused agent pane renders its ghost draft instead of a live spinner
#[tokio::test]
async fn an_unfocused_agent_pane_renders_its_ghost_draft_instead_of_a_live_spinner() {
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
    // @step And the agent 2 session is running
    app.dispatch(Action::SessionStatusChanged(
        sid("s-2"),
        codelet_rpc_types::SessionStatus::Running,
    ));
    // @step And the agent 1 pane is focused
    click_pane(&mut app, 1);
    assert_eq!(app.navigator().mux.focus(), 1);
    // @step When the grid is rendered
    let (buf, _area) = render_buf(&mut app);
    // @step Then the agent 1 pane input shows the live composer
    let left_pane = app.navigator().mux.pane_rects()[1];
    let left_text = pane_text(&buf, left_pane);
    assert!(
        left_text.contains("Type a message..."),
        "the focused agent 1 pane must show the live composer; pane: {left_text}"
    );
    // @step And the agent 2 pane input shows its ghost draft without a live spinner
    let right_pane = app.navigator().mux.pane_rects()[2];
    let right_text = pane_text(&buf, right_pane);
    assert!(
        !right_text.contains("Thinking"),
        "the unfocused agent 2 pane must NOT paint the live 'Thinking' spinner; pane: {right_text}"
    );
    assert!(
        right_text.contains("Type a message..."),
        "the unfocused agent 2 pane (empty draft) must show the placeholder ghost; pane: {right_text}"
    );
}
