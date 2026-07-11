//! CMPCT-040 — COMPACTED badge sign-integrity (Rust twin).
//!
//! Feature: spec/features/compaction-badge-sign-integrity.feature
//!
//! The header badge must never sign-flip a negative compaction
//! reduction into a fake positive percentage. The invariant is
//! enforced at the single writer (`dispatch_stream_chunks.rs`
//! `CompactionComplete` handler clamps the stored reduction to >= 0)
//! and the renderer (`header_build.rs`) renders the stored value
//! verbatim — no `.abs()`.
//!
//! Fixture pattern reuses
//! `compaction_reduction_display_contract_rpc420.rs` /
//! `agentview_session_header_compaction_percentage_rpc100.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{CompactionResult, SessionId, SessionStatus, StreamChunk};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

mod common;
use common::MockBackend;

// ───────────────────────── helpers ────────────────────────────────────────

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn agent_app_with_single_session() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.agent_view_store_mut()
        .set_session_status(sid("s-1"), SessionStatus::Running);
    app.navigator_mut().active_view = ViewMode::Agent;
    app.agent_view_store_mut().focus_session_index(0);
    (app, mock)
}

/// Drain every pending Action queued onto `action_tx` and re-dispatch
/// them so side-effects land before the next render.
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

/// Header row text — row 0 holds the SessionHeader strip.
fn header_text(buf: &Buffer) -> String {
    let mut s = String::new();
    for x in 0..buf.area.width {
        s.push_str(buf[(x, 0)].symbol());
    }
    s
}

// ───────────────────────── tests ──────────────────────────────────────────

/// Scenario: Rust writer clamps a negative wire reduction to zero
#[test]
fn rust_writer_clamps_negative_wire_reduction_to_zero() {
    // @step Given session "s-1" is open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_single_session();

    // @step When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 1000, compacted_tokens: 2500, compression_ratio: -150.0, turns_summarized: 4, turns_kept: 2 } }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: CompactionResult {
                original_tokens: 1000,
                compacted_tokens: 2500,
                compression_ratio: -150.0,
                turns_summarized: 4,
                turns_kept: 2,
            },
        },
    ));

    // @step And the App renders the AgentView into a 100x24 TestBackend
    drain_actions(&mut app);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the stored compaction reduction for "s-1" is 0
    assert_eq!(
        app.agent_view_store().compaction_reduction_for(&sid("s-1")),
        Some(0),
        "the single writer must clamp a negative wire reduction to 0"
    );

    // @step And the SessionHeader text contains "COMPACTED 0%"
    assert!(
        header.contains("COMPACTED 0%"),
        "a clamped negative reduction must render 'COMPACTED 0%', got: {header:?}"
    );

    // @step And the SessionHeader text does NOT contain "COMPACTED 150%"
    assert!(
        !header.contains("COMPACTED 150%"),
        "growth must never be sign-flipped into a fake positive badge, got: {header:?}"
    );

    // @step And the SessionHeader text does NOT contain "COMPACTED -150%"
    assert!(
        !header.contains("COMPACTED -150%"),
        "the writer clamp must prevent a raw negative reaching the badge, got: {header:?}"
    );
}

/// Scenario: Rust renderer renders a stored negative reduction verbatim without sign-flipping
#[test]
fn rust_renderer_renders_stored_negative_verbatim() {
    // @step Given session "s-1" is open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_single_session();

    // @step And a compaction reduction of -35 is forced directly into the store via set_compaction_reduction
    app.agent_view_store_mut()
        .set_compaction_reduction(sid("s-1"), -35);

    // @step When the App renders the AgentView into a 100x24 TestBackend
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "COMPACTED -35%"
    assert!(
        header.contains("COMPACTED -35%"),
        "the renderer must render the stored value verbatim (honesty over \
         cosmetics), got: {header:?}"
    );

    // @step And the SessionHeader text does NOT contain "COMPACTED 35%"
    assert!(
        !header.contains("COMPACTED 35%"),
        "the renderer must never .abs()-flip a stored negative into a fake \
         positive badge, got: {header:?}"
    );
}

/// Scenario: Rust positive wire reduction still renders unchanged with the writer clamp in place
#[test]
fn rust_positive_wire_reduction_renders_unchanged() {
    // @step Given session "s-1" is open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_single_session();

    // @step When Action::ChunkReceived("s-1", StreamChunk::CompactionComplete { compaction_result: CompactionResult { original_tokens: 10000, compacted_tokens: 4000, compression_ratio: 60.0, turns_summarized: 12, turns_kept: 3 } }) is dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: CompactionResult {
                original_tokens: 10000,
                compacted_tokens: 4000,
                compression_ratio: 60.0,
                turns_summarized: 12,
                turns_kept: 3,
            },
        },
    ));

    // @step And the App renders the AgentView into a 100x24 TestBackend
    drain_actions(&mut app);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the stored compaction reduction for "s-1" is 60
    assert_eq!(
        app.agent_view_store().compaction_reduction_for(&sid("s-1")),
        Some(60),
        "the writer clamp must not alter legitimate positive reductions"
    );

    // @step And the SessionHeader text contains "COMPACTED 60%"
    assert!(
        header.contains("COMPACTED 60%"),
        "a positive wire reduction must still render unchanged, got: {header:?}"
    );
}
