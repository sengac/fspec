//! BUG-178 — SessionHeader [ISOLATED] badge chrome_paint wiring (GREEN phase).
//!
//! Feature: spec/features/sessionheader-isolated-badge-chrome-paint-wiring.feature
//!
//! These tests were written BEFORE the implementation (RED phase — the two
//! "badge must paint" scenarios failed because
//! `chrome_paint::paint_header_and_role` hardcoded
//! `is_isolated: false` when constructing the `SessionHeader`, so the
//! green [ISOLATED] badge never painted even though the per-session
//! store slot (`isolation_state_by_session`) is populated by
//! `Action::ChunkReceived(_, StreamChunk::IsolationStateChange)` via
//! `App::handle_stream_chunk_state_updates`).
//!
//! The implementation reads the per-session slot exactly the way
//! `is_debug_enabled` reads its slot, and all five scenarios now pass.
//!
//! Each `#[test]` maps 1:1 to a Scenario in the feature file and uses
//! the same fixture pattern as
//! `agentview_session_header_per_session_tokens_rpc099.rs`:
//! real `App` + real `AgentViewStore` + `MockBackend`, rendered into
//! a `ratatui::TestBackend`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::sync::Arc;

use codelet_fspec_tui::store::IsolationState;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;
use ratatui::Terminal;

mod common;
use common::MockBackend;

// ───────────────────────── helpers ────────────────────────────────────────

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Build an App in ViewMode::Agent with a single session s-1, mirroring
/// the fixture pattern of the RPC-099 tests.
fn agent_app_with_session() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

/// Render the full App (Navigator + Compositor) into a 100x24
/// TestBackend and return the buffer for header-row assertions.
fn render_app_buffer(app: &mut App) -> Buffer {
    let backend = TestBackend::new(100, 24);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|frame| {
        app.render(frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    term.backend().buffer().clone()
}

/// Header row text — row 0 holds the SessionHeader strip.
fn header_text(buf: &Buffer) -> String {
    let mut s = String::new();
    for x in 0..buf.area.width {
        s.push_str(buf[(x, 0)].symbol());
    }
    s
}

/// Column index of the `I` glyph inside the first `[ISOLATED]` badge
/// on row 0 (panics via `expect` when the badge is absent — only call
/// after asserting `header_text` contains `[ISOLATED]`).
fn isolated_badge_i_col(buf: &Buffer) -> u16 {
    let row = header_text(buf);
    let idx = row.find("[ISOLATED]").expect("find [ISOLATED] badge");
    let col = row[..idx].chars().count() as u16 + 1; // skip the '['
    assert_eq!(buf[(col, 0)].symbol(), "I", "cell at col {col} should be 'I'");
    col
}

// ───────────────────────── tests ──────────────────────────────────────────

/// Scenario: paint_header_and_role paints [ISOLATED] when store says session is isolated
#[test]
fn scenario_paint_header_and_role_paints_isolated_when_store_says_session_is_isolated() {
    // @step Given an App with a single session s-1 in AgentView
    let (mut app, _mock) = agent_app_with_session();
    // @step And the store's isolation_state_by_session[s-1] is IsolationState { is_isolated: true, worktree_path: Some("/tmp/wt"), base_commit: Some("abc123") }
    app.agent_view_store_mut().set_isolation_state(
        sid("s-1"),
        IsolationState {
            is_isolated: true,
            worktree_path: Some("/tmp/wt".to_string()),
            base_commit: Some("abc123".to_string()),
        },
    );
    // @step When the App renders
    let buf = render_app_buffer(&mut app);
    // @step Then the header row contains the substring "[ISOLATED]"
    let row = header_text(&buf);
    assert!(
        row.contains("[ISOLATED]"),
        "header must paint '[ISOLATED]' when the store says isolated; got: {row:?}"
    );
    // And the badge paints green (RPC-029 badge colour contract)
    let col = isolated_badge_i_col(&buf);
    assert_eq!(
        buf[(col, 0)].fg,
        Color::Green,
        "[ISOLATED] fg must be Green"
    );
}

/// Scenario: paint_header_and_role does not paint [ISOLATED] when store says session is not isolated
#[test]
fn scenario_paint_header_and_role_does_not_paint_isolated_when_store_says_not_isolated() {
    // @step Given an App with a single session s-1 in AgentView
    let (mut app, _mock) = agent_app_with_session();
    // @step And the store's isolation_state_by_session[s-1] is IsolationState { is_isolated: false, worktree_path: None, base_commit: None }
    app.agent_view_store_mut().set_isolation_state(
        sid("s-1"),
        IsolationState {
            is_isolated: false,
            worktree_path: None,
            base_commit: None,
        },
    );
    // @step When the App renders
    let buf = render_app_buffer(&mut app);
    // @step Then the header row does NOT contain the substring "[ISOLATED]"
    let row = header_text(&buf);
    assert!(
        !row.contains("[ISOLATED]"),
        "header must NOT paint '[ISOLATED]' when the store says non-isolated; got: {row:?}"
    );
}

/// Scenario: paint_header_and_role does not paint [ISOLATED] when no isolation chunk has arrived
#[test]
fn scenario_paint_header_and_role_does_not_paint_isolated_when_no_isolation_chunk_has_arrived() {
    // @step Given an App with a single session s-1 in AgentView
    let (mut app, _mock) = agent_app_with_session();
    // @step When the App renders
    let buf = render_app_buffer(&mut app);
    // @step Then the header row does NOT contain the substring "[ISOLATED]"
    let row = header_text(&buf);
    assert!(
        !row.contains("[ISOLATED]"),
        "header must NOT paint '[ISOLATED]' when no IsolationState entry exists; got: {row:?}"
    );
}

/// Scenario: IsolationStateChange(true, ...) chunk dispatched before render makes badge appear
#[test]
fn scenario_isolation_state_change_true_chunk_dispatched_before_render_makes_badge_appear() {
    // @step Given an App with a single session s-1 in AgentView
    let (mut app, _mock) = agent_app_with_session();
    // @step When Action::ChunkReceived(s-1, StreamChunk::IsolationStateChange { is_isolated: true, worktree_path: Some("/tmp/wt"), base_commit: Some("abc123") }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::IsolationStateChange {
            is_isolated: true,
            worktree_path: Some("/tmp/wt".to_string()),
            base_commit: Some("abc123".to_string()),
        },
    ));
    // @step And the App renders
    let buf = render_app_buffer(&mut app);
    // @step Then the header row contains the substring "[ISOLATED]"
    let row = header_text(&buf);
    assert!(
        row.contains("[ISOLATED]"),
        "header must paint '[ISOLATED]' after an IsolationStateChange(true, ...) chunk; got: {row:?}"
    );
    let col = isolated_badge_i_col(&buf);
    assert_eq!(
        buf[(col, 0)].fg,
        Color::Green,
        "[ISOLATED] fg must be Green"
    );
}

/// Scenario: IsolationStateChange(false, None) chunk dispatched before render removes badge
#[test]
fn scenario_isolation_state_change_false_chunk_dispatched_before_render_removes_badge() {
    // @step Given an App with a single session s-1 in AgentView
    let (mut app, _mock) = agent_app_with_session();
    // @step And Action::ChunkReceived(s-1, StreamChunk::IsolationStateChange { is_isolated: true, worktree_path: Some("/tmp/wt"), base_commit: Some("abc123") }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::IsolationStateChange {
            is_isolated: true,
            worktree_path: Some("/tmp/wt".to_string()),
            base_commit: Some("abc123".to_string()),
        },
    ));
    // @step When Action::ChunkReceived(s-1, StreamChunk::IsolationStateChange { is_isolated: false, worktree_path: None, base_commit: None }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::IsolationStateChange {
            is_isolated: false,
            worktree_path: None,
            base_commit: None,
        },
    ));
    // @step And the App renders
    let buf = render_app_buffer(&mut app);
    // @step Then the header row does NOT contain the substring "[ISOLATED]"
    let row = header_text(&buf);
    assert!(
        !row.contains("[ISOLATED]"),
        "header must drop '[ISOLATED]' after an IsolationStateChange(false, None) chunk; got: {row:?}"
    );
}
