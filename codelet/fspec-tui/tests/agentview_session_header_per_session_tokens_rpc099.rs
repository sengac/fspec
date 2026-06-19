//! RPC-099 — AgentView SessionHeader per-session token tracking parity (RED phase).
//!
//! Feature: spec/features/agentview-session-header-per-session-tokens.feature
//!
//! These tests are written BEFORE the implementation and currently FAIL
//! because:
//!   * `TokenState` only carries {input_tokens, output_tokens, context_fill_pct}.
//!     Once the fix lands it must also carry cache_read_input_tokens,
//!     cache_creation_input_tokens, reasoning_tokens, tokens_per_second.
//!   * `chrome_paint::paint_header_and_role` hardcodes `tokens_per_second:
//!     None`, `reasoning_tokens: 0`, `compaction_reduction: None` — the
//!     header therefore never reflects per-session reasoning/tps values.
//!
//! Each `#[test]` here maps 1:1 to a Scenario in the feature file and uses
//! the same fixture pattern as `agentview_esc_exit_confirmation_rpc098.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::sync::Arc;

use codelet_fspec_tui::store::TokenState;
use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend, SessionContext};
use codelet_rpc_types::{SessionId, SessionStatus, StreamChunk, TokenTracker};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

mod common;
use common::MockBackend;

// ───────────────────────── helpers ────────────────────────────────────────

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// TokenTracker builder filling every field with explicit Some(0) for the
/// optional ones so tests that don't care about a particular field still
/// surface deterministic values into the store.
fn token_tracker(
    input: u32,
    output: u32,
    reasoning: Option<u32>,
    tps: Option<f64>,
    cache_read: Option<u32>,
    cache_creation: Option<u32>,
) -> TokenTracker {
    TokenTracker {
        input_tokens: input,
        output_tokens: output,
        cache_read_input_tokens: cache_read,
        cache_creation_input_tokens: cache_creation,
        tokens_per_second: tps,
        cumulative_billed_input: Some(0),
        cumulative_billed_output: Some(0),
        reasoning_tokens: reasoning,
    }
}

/// Build an App in ViewMode::Agent with two open sessions s-1 and s-2,
/// both Running so that `tokens_per_second` flows through the loading
/// branch of `build_right_line` (see header_build.rs:138-145).
fn agent_app_with_two_sessions() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // Use Action::SessionCreated for the first session so the App state
    // initialises identically to production, then append the second via
    // the store directly (mirrors the pattern used in
    // agentview_esc_exit_confirmation_rpc098.rs).
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.agent_view_store_mut()
        .append_session(SessionContext::new(sid("s-2")));

    // Status = Running so `is_loading` is true at the chrome paint site
    // → tokens_per_second prefix renders in build_right_line.
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);
    app.agent_view_store_mut()
        .set_session_status(sid("s-2"), SessionStatus::Running);

    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

/// Render the full App (Navigator + Compositor) into a 100x24 TestBackend
/// and return the buffer for substring assertions on the header line.
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

/// Make s-1 the focused session by calling focus_session_index(0).
fn focus_s1(app: &mut App) {
    app.agent_view_store_mut().focus_session_index(0);
}

// ───────────────────────── tests ──────────────────────────────────────────

/// Scenario: Shift+Right swaps SessionHeader to the new session's full token totals
#[test]
fn shift_right_swaps_session_header_to_new_session_full_token_totals() {
    // @step Given two sessions "s-1" and "s-2" are open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_two_sessions();
    focus_s1(&mut app);

    // @step And Action::ChunkReceived("s-1", StreamChunk::TokenUpdate { tokens: TokenTracker { input_tokens: 100, output_tokens: 50, reasoning_tokens: Some(20), tokens_per_second: Some(8.5), cache_read_input_tokens: Some(0), cache_creation_input_tokens: Some(0) } }) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::TokenUpdate {
            tokens: token_tracker(100, 50, Some(20), Some(8.5), Some(0), Some(0)),
        },
    ));
    // @step And Action::ChunkReceived("s-2", StreamChunk::TokenUpdate { tokens: TokenTracker { input_tokens: 200, output_tokens: 75, reasoning_tokens: Some(60), tokens_per_second: Some(12.0), cache_read_input_tokens: Some(0), cache_creation_input_tokens: Some(0) } }) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-2"),
        StreamChunk::TokenUpdate {
            tokens: token_tracker(200, 75, Some(60), Some(12.0), Some(0), Some(0)),
        },
    ));

    // @step When the App renders the AgentView into a 100x24 TestBackend with "s-1" focused
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "tokens: 100↓ 50↑" and "20🧠" and reflects tokens_per_second=8.5
    assert!(
        header.contains("tokens: 100↓ 50↑"),
        "header should show s-1 totals 'tokens: 100↓ 50↑', got: {header:?}"
    );
    assert!(
        header.contains("20🧠"),
        "header should show s-1 reasoning '20🧠', got: {header:?}"
    );
    assert!(
        header.contains("8.5 tok/s"),
        "header should show s-1 tokens_per_second '8.5 tok/s', got: {header:?}"
    );

    // @step When the App dispatches Action::SessionNext and re-renders
    app.dispatch(Action::SessionNext);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "tokens: 200↓ 75↑" and "60🧠" and reflects tokens_per_second=12.0
    assert!(
        header.contains("tokens: 200↓ 75↑"),
        "after Shift+Right header should show s-2 totals 'tokens: 200↓ 75↑', got: {header:?}"
    );
    assert!(
        header.contains("60🧠"),
        "after Shift+Right header should show s-2 reasoning '60🧠', got: {header:?}"
    );
    assert!(
        header.contains("12.0 tok/s"),
        "after Shift+Right header should show s-2 tokens_per_second '12.0 tok/s', got: {header:?}"
    );

    // @step When the App dispatches Action::SessionPrev and re-renders
    app.dispatch(Action::SessionPrev);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "tokens: 100↓ 50↑" and "20🧠" and reflects tokens_per_second=8.5
    assert!(
        header.contains("tokens: 100↓ 50↑"),
        "after Shift+Left header should show s-1 totals again, got: {header:?}"
    );
    assert!(
        header.contains("20🧠"),
        "after Shift+Left header should show s-1 reasoning '20🧠' again, got: {header:?}"
    );
    assert!(
        header.contains("8.5 tok/s"),
        "after Shift+Left header should show s-1 '8.5 tok/s' again, got: {header:?}"
    );
}

/// Scenario: Background session accumulates token state while not focused
#[test]
fn background_session_accumulates_token_state_while_not_focused() {
    // @step Given two sessions "s-1" and "s-2" are open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_two_sessions();
    focus_s1(&mut app);

    // @step When Action::ChunkReceived("s-2", StreamChunk::TokenUpdate { tokens: TokenTracker { input_tokens: 200, output_tokens: 75, reasoning_tokens: Some(60), tokens_per_second: None, cache_read_input_tokens: Some(0), cache_creation_input_tokens: Some(0) } }) is dispatched while "s-1" remains focused
    app.dispatch(Action::ChunkReceived(
        sid("s-2"),
        StreamChunk::TokenUpdate {
            tokens: token_tracker(200, 75, Some(60), None, Some(0), Some(0)),
        },
    ));
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&sid("s-1")),
        "s-1 should remain focused while s-2 receives the background chunk"
    );

    // @step And the App dispatches Action::SessionNext to focus "s-2" and renders the AgentView into a 100x24 TestBackend
    app.dispatch(Action::SessionNext);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text immediately contains "tokens: 200↓ 75↑" and "60🧠" with no intermediate zero-state frame
    assert!(
        header.contains("tokens: 200↓ 75↑"),
        "background-accumulated s-2 totals should be visible on first frame, got: {header:?}"
    );
    assert!(
        header.contains("60🧠"),
        "background-accumulated s-2 reasoning '60🧠' should be visible on first frame, got: {header:?}"
    );
}

/// Scenario: cache_read_input_tokens and cache_creation_input_tokens are persisted per-session
#[test]
fn cache_tokens_persisted_per_session_across_focus_switches() {
    // @step Given two sessions "s-1" and "s-2" are open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_two_sessions();
    focus_s1(&mut app);

    // @step When Action::ChunkReceived("s-1", StreamChunk::TokenUpdate { tokens: TokenTracker { input_tokens: 100, output_tokens: 50, cache_read_input_tokens: Some(5000), cache_creation_input_tokens: Some(800), reasoning_tokens: None, tokens_per_second: None } }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::TokenUpdate {
            tokens: token_tracker(100, 50, None, None, Some(5000), Some(800)),
        },
    ));

    // @step Then agent_view_store.token_state_for(SessionId("s-1")) returns Some(TokenState) with cache_read_input_tokens = 5000 and cache_creation_input_tokens = 800
    let ts: TokenState = app
        .agent_view_store()
        .token_state_for(&sid("s-1"))
        .copied()
        .expect("token_state_for(s-1) returns Some after TokenUpdate");
    assert_eq!(
        ts.cache_read_input_tokens, 5000,
        "TokenState.cache_read_input_tokens must equal 5000 after TokenUpdate"
    );
    assert_eq!(
        ts.cache_creation_input_tokens, 800,
        "TokenState.cache_creation_input_tokens must equal 800 after TokenUpdate"
    );

    // @step When the App dispatches Action::SessionNext (focus "s-2"), then Action::SessionPrev (focus "s-1") again
    app.dispatch(Action::SessionNext);
    app.dispatch(Action::SessionPrev);

    // @step Then agent_view_store.token_state_for(SessionId("s-1")) still returns Some(TokenState) with cache_read_input_tokens = 5000 and cache_creation_input_tokens = 800
    let ts: TokenState = app
        .agent_view_store()
        .token_state_for(&sid("s-1"))
        .copied()
        .expect("token_state_for(s-1) still Some after focus swap");
    assert_eq!(
        ts.cache_read_input_tokens, 5000,
        "TokenState.cache_read_input_tokens persists across focus swap"
    );
    assert_eq!(
        ts.cache_creation_input_tokens, 800,
        "TokenState.cache_creation_input_tokens persists across focus swap"
    );
}

/// Scenario: Switching to a never-updated session displays zeros (no carry-over)
#[test]
fn switching_to_never_updated_session_displays_zeros_no_carry_over() {
    // @step Given session "s-1" is focused with TokenState { input_tokens: 1234, output_tokens: 567 } from a prior dispatched TokenUpdate
    let (mut app, _mock) = agent_app_with_two_sessions();
    focus_s1(&mut app);
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::TokenUpdate {
            tokens: token_tracker(1234, 567, None, None, Some(0), Some(0)),
        },
    ));

    // @step And a fresh session "s-2" has been opened with append_session and no Action::ChunkReceived has been dispatched for it
    // (agent_app_with_two_sessions already opened s-2 via append_session and no TokenUpdate was dispatched against it)
    assert!(
        app.agent_view_store()
            .token_state_for(&sid("s-2"))
            .is_none(),
        "s-2 should have no TokenState entry yet"
    );

    // @step When the App dispatches Action::SessionNext to focus "s-2" and renders the AgentView into a 100x24 TestBackend
    app.dispatch(Action::SessionNext);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "tokens: 0↓ 0↑"
    assert!(
        header.contains("tokens: 0↓ 0↑"),
        "fresh s-2 should display zeros, got: {header:?}"
    );
    // @step And the SessionHeader text does NOT contain "1234↓"
    assert!(
        !header.contains("1234↓"),
        "s-1 input_tokens must NOT leak into s-2 header, got: {header:?}"
    );
    // @step And the SessionHeader text does NOT contain "567↑"
    assert!(
        !header.contains("567↑"),
        "s-1 output_tokens must NOT leak into s-2 header, got: {header:?}"
    );
}

/// Scenario: Reasoning brain suffix toggles based on the focused session's reasoning_tokens
#[test]
fn reasoning_brain_suffix_toggles_with_focused_session_reasoning_tokens() {
    // @step Given two sessions "s-1" and "s-2" are open with "s-1" focused
    let (mut app, _mock) = agent_app_with_two_sessions();
    focus_s1(&mut app);

    // @step And Action::ChunkReceived("s-1", StreamChunk::TokenUpdate { tokens: TokenTracker { input_tokens: 100, output_tokens: 50, reasoning_tokens: None, tokens_per_second: None, cache_read_input_tokens: Some(0), cache_creation_input_tokens: Some(0) } }) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::TokenUpdate {
            tokens: token_tracker(100, 50, None, None, Some(0), Some(0)),
        },
    ));
    // @step And Action::ChunkReceived("s-2", StreamChunk::TokenUpdate { tokens: TokenTracker { input_tokens: 200, output_tokens: 75, reasoning_tokens: Some(45), tokens_per_second: None, cache_read_input_tokens: Some(0), cache_creation_input_tokens: Some(0) } }) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-2"),
        StreamChunk::TokenUpdate {
            tokens: token_tracker(200, 75, Some(45), None, Some(0), Some(0)),
        },
    ));

    // @step When the App renders the AgentView into a 100x24 TestBackend with "s-1" focused
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "tokens: 100↓ 50↑"
    assert!(
        header.contains("tokens: 100↓ 50↑"),
        "s-1 totals must render, got: {header:?}"
    );
    // @step And the SessionHeader text does NOT contain "🧠"
    assert!(
        !header.contains("🧠"),
        "s-1 has no reasoning_tokens so 🧠 must be absent, got: {header:?}"
    );

    // @step When the App dispatches Action::SessionNext and re-renders
    app.dispatch(Action::SessionNext);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "tokens: 200↓ 75↑" and "45🧠"
    assert!(
        header.contains("tokens: 200↓ 75↑"),
        "s-2 totals must render after Shift+Right, got: {header:?}"
    );
    assert!(
        header.contains("45🧠"),
        "s-2 reasoning '45🧠' must render after Shift+Right, got: {header:?}"
    );
}
