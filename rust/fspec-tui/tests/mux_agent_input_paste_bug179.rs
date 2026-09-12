// Feature: spec/features/mux-agent-input-paste-drop-routing.feature
//! BUG-179 — mux agent input paste & terminal file-drop routing.
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios.
//!
//! Root cause (spec/attachments/BUG-179/bug179-research.md): the mux
//! keyboard-isolation gate (`Navigator::handle_mux_event`) drops every
//! non-`Event::Key` event, so a bracketed paste (`Event::Paste` — including
//! a terminal file drop delivered as a paste) is silently swallowed while
//! mux is active. `App::handle_paste` also lacks the post-Navigator
//! `sync_mux_focus_to_session` step that `App::handle_event` runs (BUG-163),
//! so even a routed paste would operate on a possibly-stale session.
//!
//! Red-phase expectations (pre-fix):
//!   - Scenarios 1, 3, 4, 7 FAIL: the paste never reaches the composer.
//!   - Scenarios 2, 5, 6 pass both phases (guards: board-pane isolation,
//!     compacting-gate parity, R10 no-change-outside-mux).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, SessionStatus};
use crossterm::event::{
    Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

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

/// Open two sessions (s-1, s-2) and enter mux with `/mux board agent
/// agent` (the feature's shared Given).
///
/// Natural post-`/mux` state: mux focus sits on pane 1 (the first agent
/// pane, window session s-1) while the store's current session is s-2
/// (last created) — a genuine focus/store divergence. A click inside a
/// pane converges it (the production click-to-focus path + the
/// post-Navigator BUG-163 sync in `App::handle_event`).
async fn enter_mux_board_agent_agent(app: &mut App) {
    for i in 1..=2 {
        app.dispatch(Action::SessionCreated(sid(&format!("s-{i}"))));
    }
    drain_pending(app).await;
    submit(app, "/mux board agent agent");
    drain_pending(app).await;
    assert_eq!(
        app.active_view(),
        ViewMode::Mux,
        "/mux must enter the grid"
    );
    assert!(
        app.navigator().mux.config().enabled,
        "the mux config flag must be ON inside the grid"
    );
}

/// Focus the Nth (0-based) rendered pane by clicking inside its cached
/// rect — the production click-to-focus path (Navigator → mux_mouse).
/// The click also runs the post-Navigator BUG-163 sync, converging the
/// store's current session onto the clicked agent pane's window session.
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

/// Render the App once into an off-screen buffer so per-frame caches
/// (`AgentView::last_is_compacting` — the RPC-095 gate input) refresh.
fn render_once(app: &mut App) {
    let area = Rect::new(0, 0, 160, 30);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: multi-line paste into the focused agent pane is inserted into
//           the composer
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn multi_line_paste_into_the_focused_agent_pane_is_inserted_into_the_composer() {
    // @step Given mux mode is active with the pane list board, agent and agent
    // @step And two agent sessions are open
    let (mut app, _mock) = fresh_app();
    enter_mux_board_agent_agent(&mut app).await;
    // @step And the first agent pane is focused
    // `/mux` homes the focus on the first agent pane (pane 1); the click
    // also converges the store's current session (s-2) onto that pane's
    // window session (s-1) via the production click-to-focus path.
    click_pane(&mut app, 1);
    assert_eq!(
        app.navigator().mux.focus(),
        1,
        "the first agent pane must be focused"
    );
    assert_eq!(
        app.current_session(),
        Some(sid("s-1")),
        "click-to-focus must converge the store onto the pane's window session"
    );

    // @step When I paste a 3-line snippet
    let _ = app.handle_paste("line one\nline two\nline three");

    // @step Then the composer holds all 3 lines separated by newlines
    assert_eq!(
        app.navigator().agent.input.value(),
        "line one\nline two\nline three",
        "BUG-179: a bracketed paste must reach the focused mux pane's composer"
    );
    // @step And the cursor is at the end of the pasted text
    assert_eq!(
        app.navigator().agent.input.cursor(),
        (2, "line three".len()),
        "the cursor must land at the end of the pasted text"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: paste while the board pane is focused is ignored
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn paste_while_the_board_pane_is_focused_is_ignored() {
    // @step Given mux mode is active with the pane list board, agent and agent
    // @step And two agent sessions are open
    let (mut app, _mock) = fresh_app();
    enter_mux_board_agent_agent(&mut app).await;
    // @step And the board pane is focused
    click_pane(&mut app, 0);
    assert_eq!(
        app.navigator().mux.focus(),
        0,
        "the board pane must be focused"
    );

    // @step When I paste a text snippet
    let result = app.handle_paste("a text snippet");

    // @step Then the paste is not consumed
    assert!(
        !result.is_consumed(),
        "the board pane has no text input — the paste must stay Ignored (R2 isolation)"
    );
    // @step And the composer remains empty
    assert_eq!(
        app.navigator().agent.input.value(),
        "",
        "a paste ignored by the focused board pane must never leak into the composer"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: paste after click-to-focus lands in the newly focused
//           session's composer
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn paste_after_click_to_focus_lands_in_the_newly_focused_sessions_composer() {
    // @step Given mux mode is active with the pane list board, agent and agent
    // @step And two agent sessions are open
    let (mut app, _mock) = fresh_app();
    enter_mux_board_agent_agent(&mut app).await;

    // @step When I click inside the second agent pane
    click_pane(&mut app, 2);
    assert_eq!(
        app.navigator().mux.focus(),
        2,
        "the click must focus the second agent pane"
    );
    assert_eq!(
        app.current_session(),
        Some(sid("s-2")),
        "click-to-focus must converge the store onto the second pane's window session"
    );

    // @step And I paste a text snippet
    let _ = app.handle_paste("pasted text");

    // @step Then the composer holds the pasted text
    assert_eq!(
        app.navigator().agent.input.value(),
        "pasted text",
        "BUG-179: the paste must land in the newly focused session's composer"
    );
    // @step And the store's current session is the second pane's window session
    assert_eq!(
        app.current_session(),
        Some(sid("s-2")),
        "the post-paste mux focus sync must keep the store on the focused pane's window session"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: paste never mutates the ghost draft of an unfocused agent pane
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn paste_never_mutates_the_ghost_draft_of_an_unfocused_agent_pane() {
    // @step Given mux mode is active with the pane list board, agent and agent
    // @step And two agent sessions are open
    let (mut app, _mock) = fresh_app();
    enter_mux_board_agent_agent(&mut app).await;
    // @step And the first agent pane is focused
    click_pane(&mut app, 1);
    assert_eq!(
        app.navigator().mux.focus(),
        1,
        "the first agent pane must be focused"
    );
    // @step And the second session's input draft holds the text "ghost-draft"
    app.agent_view_store_mut()
        .set_input_draft(1, "ghost-draft".to_string());

    // @step When I paste a text snippet
    let _ = app.handle_paste("pasted text");

    // @step Then the composer holds the pasted text
    assert_eq!(
        app.navigator().agent.input.value(),
        "pasted text",
        "BUG-179: the paste must reach the focused pane's live composer"
    );
    // @step And the second session's input draft still holds the text "ghost-draft"
    assert_eq!(
        app.agent_view_store()
            .session_context_for(&sid("s-2"))
            .expect("s-2 must be open")
            .input_draft,
        "ghost-draft",
        "a paste must never mutate the ghost draft of an unfocused agent pane (BUG-163 model)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: paste while the focused session is compacting is suppressed
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn paste_while_the_focused_session_is_compacting_is_suppressed() {
    // @step Given mux mode is active with the pane list board, agent and agent
    // @step And two agent sessions are open
    let (mut app, _mock) = fresh_app();
    enter_mux_board_agent_agent(&mut app).await;
    // @step And the second agent pane is focused
    click_pane(&mut app, 2);
    assert_eq!(
        app.navigator().mux.focus(),
        2,
        "the second agent pane must be focused"
    );
    assert_eq!(
        app.current_session(),
        Some(sid("s-2")),
        "click-to-focus must converge the store onto the second pane's window session"
    );
    // @step And the focused session is compacting
    app.agent_view_store_mut()
        .set_session_status(sid("s-2"), SessionStatus::Compacting);
    // @step And the composer holds the text "hello"
    app.navigator_mut().agent.input.set_value("hello");
    // A frame render caches the gate (AgentView::last_is_compacting).
    render_once(&mut app);

    // @step When I paste a text snippet
    let _ = app.handle_paste("more text");

    // @step Then the composer still holds the text "hello"
    assert_eq!(
        app.navigator().agent.input.value(),
        "hello",
        "the RPC-095 compacting block_edits gate must suppress the paste in mux (RPC-403 parity)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: single-view paste is unchanged when the mux layout is disabled
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn single_view_paste_is_unchanged_when_the_mux_layout_is_disabled() {
    // @step Given the agent view is active with the mux layout disabled
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::OpenAgentView(Some(sid("s-1"))));
    assert_eq!(
        app.active_view(),
        ViewMode::Agent,
        "the single Agent view must be active"
    );
    assert!(
        !app.navigator().mux.config().enabled,
        "the mux layout must stay disabled"
    );

    // @step When I paste a 3-line snippet
    let _ = app.handle_paste("line one\nline two\nline three");

    // @step Then the composer holds all 3 lines separated by newlines
    assert_eq!(
        app.navigator().agent.input.value(),
        "line one\nline two\nline three",
        "R10: single-view paste routing (RPC-403) must be byte-for-byte unchanged"
    );
    // @step And the cursor is at the end of the pasted text
    assert_eq!(
        app.navigator().agent.input.cursor(),
        (2, "line three".len()),
        "the cursor must land at the end of the pasted text"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: a terminal file drop delivered as a bracketed paste inserts
//           the path verbatim
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_terminal_file_drop_delivered_as_a_bracketed_paste_inserts_the_path_verbatim() {
    // @step Given mux mode is active with the pane list board, agent and agent
    // @step And two agent sessions are open
    let (mut app, _mock) = fresh_app();
    enter_mux_board_agent_agent(&mut app).await;
    // @step And the first agent pane is focused
    click_pane(&mut app, 1);
    assert_eq!(
        app.navigator().mux.focus(),
        1,
        "the first agent pane must be focused"
    );
    assert_eq!(
        app.current_session(),
        Some(sid("s-1")),
        "click-to-focus must converge the store onto the pane's window session"
    );

    // @step When I drop a file that my terminal delivers as a bracketed paste with the text "file:///tmp/screenshot.png"
    // A terminal file drop has no dedicated crossterm event — the terminal
    // synthesizes it as a bracketed paste (`Event::Paste`). The TUI must
    // insert the payload verbatim: no file:// decoding, no drop toast
    // (single-view parity — rule 6 non-goal).
    let _ = app.handle_paste("file:///tmp/screenshot.png");

    // @step Then the composer holds "file:///tmp/screenshot.png" verbatim
    assert_eq!(
        app.navigator().agent.input.value(),
        "file:///tmp/screenshot.png",
        "the drop-as-paste payload must be inserted verbatim (no file:// decoding)"
    );
}
