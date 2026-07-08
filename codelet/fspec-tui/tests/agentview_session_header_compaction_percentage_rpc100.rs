//! RPC-100 — SessionHeader compaction percentage parity (RED phase).
//!
//! Feature: spec/features/agentview-session-header-compaction-percentage.feature
//!
//! These tests are written BEFORE the implementation and currently FAIL
//! because:
//!   * `TokenState::apply_context_fill` clamps `fill_percentage` with
//!     `.min(100) as u8`, hiding the >100% pre-compaction overshoot.
//!   * `chrome_paint::paint_header_and_role` hardcodes
//!     `compaction_reduction: None`, so the `[X%: COMPACTED Y%]`
//!     suffix never renders even after a `CompactionComplete` chunk.
//!   * The `CompactionComplete` arm of `dispatch_stream_chunks::handle_stream_chunk_state_updates`
//!     does not persist the reduction percentage on a per-session slot.
//!   * `SessionStateChange { state: Cleared }` does not reset
//!     `TokenState` or the (missing) per-session `compaction_reduction`.
//!
//! Each `#[test]` here maps 1:1 to a Scenario in the feature file and uses
//! the same fixture pattern as
//! `agentview_session_header_per_session_tokens_rpc099.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::sync::Arc;

use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend, SessionContext};
use codelet_rpc_types::{
    CompactionResult, ContextFillInfo, SessionId, SessionState, SessionStatus, StreamChunk,
    TokenTracker,
};
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

fn token_tracker(input: u32, output: u32) -> TokenTracker {
    TokenTracker {
        input_tokens: input,
        output_tokens: output,
        cache_read_input_tokens: Some(0),
        cache_creation_input_tokens: Some(0),
        tokens_per_second: None,
        cumulative_billed_input: Some(0),
        cumulative_billed_output: Some(0),
        reasoning_tokens: None,
    }
}

fn context_fill(pct: u32) -> ContextFillInfo {
    ContextFillInfo {
        fill_percentage: pct,
        effective_tokens: 0.0,
        threshold: 0.0,
        context_window: 0.0,
    }
}

fn compaction_result(original: u32, compacted: u32, ratio: f64) -> CompactionResult {
    CompactionResult {
        original_tokens: original,
        compacted_tokens: compacted,
        compression_ratio: ratio,
        turns_summarized: 8,
        turns_kept: 2,
    }
}

/// Build an App in `ViewMode::Agent` with two open sessions s-1 and s-2.
fn agent_app_with_two_sessions() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.agent_view_store_mut()
        .append_session(SessionContext::new(sid("s-2")));

    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);
    app.agent_view_store_mut()
        .set_session_status(sid("s-2"), SessionStatus::Running);

    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

fn agent_app_with_single_session() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

fn focus_s1(app: &mut App) {
    app.agent_view_store_mut().focus_session_index(0);
}

/// Drain every pending Action queued onto `action_tx` (e.g. by
/// `dispatch_stream_chunks.rs::CompactionComplete` enqueueing an
/// `EmitSessionNotice`) and re-dispatch them so side-effects land
/// before the next render.
fn drain_actions(app: &mut App) {
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
    }
}

/// Render the full App into a 100x24 `TestBackend` and return the buffer.
fn render_app_buffer(app: &mut App) -> Buffer {
    let backend = TestBackend::new(100, 24);
    let mut term = Terminal::new(backend).expect("Terminal::new");
    term.draw(|frame| {
        app.render(frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    term.backend().buffer().clone()
}

/// Concatenate the symbol of every cell on row `y`.
fn row_text(buf: &Buffer, y: u16) -> String {
    let mut s = String::new();
    for x in 0..buf.area.width {
        s.push_str(buf[(x, y)].symbol());
    }
    s
}

/// Header row text — row 0 holds the SessionHeader strip.
fn header_text(buf: &Buffer) -> String {
    row_text(buf, 0)
}

/// Scrape ALL rows below the header (rows 1..height) — used by the
/// formula-coherence scenario to assert the scrollback contains the
/// `[compaction] N.N% reduction …` notice line.
fn body_text(buf: &Buffer) -> String {
    let mut s = String::new();
    for y in 1..buf.area.height {
        s.push_str(&row_text(buf, y));
        s.push('\n');
    }
    s
}

/// Locate the column index where `needle` starts in row 0, then return
/// the foreground colour of that first cell. Used to assert `[105%]`
/// renders red.
fn fg_color_at_substring(buf: &Buffer, needle: &str) -> Option<Color> {
    let line = header_text(buf);
    let byte_idx = line.find(needle)?;
    // Map the byte index of the substring back to a column index by
    // scanning cells until the accumulated text length reaches byte_idx.
    let mut acc = String::new();
    for x in 0..buf.area.width {
        if acc.len() >= byte_idx {
            return Some(buf[(x, 0)].fg);
        }
        acc.push_str(buf[(x, 0)].symbol());
    }
    None
}

// ───────────────────────── tests ──────────────────────────────────────────

/// Scenario: ContextFillUpdate above 100% renders the raw value (not clamped to 100)
#[test]
fn context_fill_update_above_100_renders_raw_value_not_clamped() {
    // @step Given session "s-1" is open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_single_session();
    focus_s1(&mut app);

    // @step When Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate { context_fill: ContextFillInfo { fill_percentage: 105, effective_tokens: 0.0, threshold: 0.0, context_window: 0.0 } }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::ContextFillUpdate {
            context_fill: context_fill(105),
        },
    ));

    // @step And the App renders the AgentView into a 100x24 TestBackend
    drain_actions(&mut app);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "[105%]"
    assert!(
        header.contains("[105%]"),
        "header should render raw 105% (not clamped to 100), got: {header:?}"
    );
    // @step And the SessionHeader text does NOT contain "[100%]"
    assert!(
        !header.contains("[100%]"),
        ".min(100) clamp must NOT collapse 105 → 100, got: {header:?}"
    );
    // @step And the percentage span foreground style is Color::Red
    let fg = fg_color_at_substring(&buf, "[105%]");
    assert_eq!(
        fg,
        Some(Color::Red),
        "context_fill_color band for >=85 must be Red, got fg={fg:?} header={header:?}"
    );
}

/// Scenario: CompactionComplete adds the COMPACTED suffix with reduction taken directly from the percent-unit compression_ratio
#[test]
fn compaction_complete_adds_compacted_suffix_with_correct_reduction_formula() {
    // @step Given session "s-1" is open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_single_session();
    focus_s1(&mut app);

    // @step And Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate { context_fill: ContextFillInfo { fill_percentage: 80, effective_tokens: 0.0, threshold: 0.0, context_window: 0.0 } }) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::ContextFillUpdate {
            context_fill: context_fill(80),
        },
    ));

    // @step When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 10000, compacted_tokens: 4000, compression_ratio: 60.0, turns_summarized: 12 } }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: compaction_result(10000, 4000, 60.0),
        },
    ));

    // @step And the App renders the AgentView into a 100x24 TestBackend
    drain_actions(&mut app);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "[80%: COMPACTED 60%]"
    assert!(
        header.contains("[80%: COMPACTED 60%]"),
        "header should show '[80%: COMPACTED 60%]' (wire 60.0 is already percent removed — RPC-420), got: {header:?}"
    );
}

/// Scenario: compaction_reduction is per-session and does not leak across Shift+Right
#[test]
fn compaction_reduction_is_per_session_and_does_not_leak() {
    // @step Given two sessions "s-1" and "s-2" are open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_two_sessions();
    focus_s1(&mut app);

    // @step And Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate { context_fill: ContextFillInfo { fill_percentage: 50, effective_tokens: 0.0, threshold: 0.0, context_window: 0.0 } }) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::ContextFillUpdate {
            context_fill: context_fill(50),
        },
    ));
    // @step And Action::ChunkReceived("s-2", StreamChunk::ContextFillUpdate { context_fill: ContextFillInfo { fill_percentage: 50, effective_tokens: 0.0, threshold: 0.0, context_window: 0.0 } }) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-2"),
        StreamChunk::ContextFillUpdate {
            context_fill: context_fill(50),
        },
    ));
    // @step And Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 10000, compacted_tokens: 3000, compression_ratio: 70.0, turns_summarized: 8 } }) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: compaction_result(10000, 3000, 70.0),
        },
    ));
    drain_actions(&mut app);

    // @step When the App renders the AgentView into a 100x24 TestBackend with "s-1" focused
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "COMPACTED 70%"
    assert!(
        header.contains("COMPACTED 70%"),
        "s-1 header should show 'COMPACTED 70%' (wire 70.0 is already percent removed — RPC-420), got: {header:?}"
    );

    // @step When the App dispatches Action::SessionNext to focus "s-2" and re-renders
    app.dispatch(Action::SessionNext);
    drain_actions(&mut app);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "[50%]"
    assert!(
        header.contains("[50%]"),
        "s-2 header should show plain '[50%]' badge, got: {header:?}"
    );
    // @step And the SessionHeader text does NOT contain "COMPACTED"
    assert!(
        !header.contains("COMPACTED"),
        "s-1's COMPACTED suffix must NOT leak into s-2 header, got: {header:?}"
    );

    // @step When the App dispatches Action::SessionPrev to focus "s-1" and re-renders
    app.dispatch(Action::SessionPrev);
    drain_actions(&mut app);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "COMPACTED 70%"
    assert!(
        header.contains("COMPACTED 70%"),
        "s-1 header should show 'COMPACTED 70%' again after Shift+Left, got: {header:?}"
    );
}

/// Scenario: SessionStateChange Cleared resets context_fill_pct, tokens, and compaction_reduction
#[test]
fn session_state_change_cleared_resets_fill_tokens_and_compaction_reduction() {
    // @step Given session "s-1" is open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_single_session();
    focus_s1(&mut app);

    // @step And Action::ChunkReceived("s-1", StreamChunk::TokenUpdate { tokens: TokenTracker { input_tokens: 5000, output_tokens: 1200, reasoning_tokens: None, tokens_per_second: None, cache_read_input_tokens: Some(0), cache_creation_input_tokens: Some(0) } }) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::TokenUpdate {
            tokens: token_tracker(5000, 1200),
        },
    ));
    // @step And Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate { context_fill: ContextFillInfo { fill_percentage: 45, effective_tokens: 0.0, threshold: 0.0, context_window: 0.0 } }) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::ContextFillUpdate {
            context_fill: context_fill(45),
        },
    ));
    // @step And Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 10000, compacted_tokens: 3000, compression_ratio: 70.0, turns_summarized: 8 } }) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: compaction_result(10000, 3000, 70.0),
        },
    ));
    drain_actions(&mut app);

    // @step When Action::ChunkReceived("s-1", StreamChunk::SessionStateChange { state: SessionState::Cleared }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: SessionState::Cleared,
        },
    ));
    drain_actions(&mut app);

    // @step And the App renders the AgentView into a 100x24 TestBackend
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "[0%]"
    assert!(
        header.contains("[0%]"),
        "after Cleared the header should show '[0%]' (TokenState reset), got: {header:?}"
    );
    // @step And the SessionHeader text does NOT contain "COMPACTED"
    assert!(
        !header.contains("COMPACTED"),
        "after Cleared the compaction_reduction must be wiped, got: {header:?}"
    );
    // @step And the SessionHeader text contains "tokens: 0↓ 0↑"
    assert!(
        header.contains("tokens: 0↓ 0↑"),
        "after Cleared the token totals must be zero, got: {header:?}"
    );
}

/// Scenario: Compaction notice line and header badge agree on the reduction percentage
#[test]
fn compaction_notice_line_and_header_badge_agree_on_reduction_percentage() {
    // @step Given session "s-1" is open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_single_session();
    focus_s1(&mut app);

    // @step When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compression_ratio: 60.0, ... }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: compaction_result(10000, 4000, 60.0),
        },
    ));
    // Drain the queued Action::EmitSessionNotice so the scrollback gets
    // the `[compaction] 60.0% reduction …` line BEFORE we render.
    drain_actions(&mut app);

    // @step And the App renders the AgentView into a 100x24 TestBackend
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);
    let body = body_text(&buf);

    // @step Then the SessionHeader text contains "COMPACTED 60%"
    assert!(
        header.contains("COMPACTED 60%"),
        "header badge must show 'COMPACTED 60%', got: {header:?}"
    );
    // @step And the scrollback contains a notice line containing "60.0% reduction"
    assert!(
        body.contains("60.0% reduction"),
        "scrollback must contain '60.0% reduction' notice line (format_compaction_notice), got body:\n{body}"
    );
}
