//! RPC-417 — COMPACTED header badge 10-second auto-hide (TS parity).
//!
//! Feature: spec/features/agentview-compaction-badge-auto-hide.feature
//!
//! These tests are written BEFORE the implementation (ACDD red phase) and
//! currently FAIL because:
//!   * `Action::ClearCompactionReduction { session_id, seq }` does not yet
//!     exist on the `Action` enum.
//!   * `dispatch_stream_chunks.rs::CompactionComplete` does not arm a
//!     10-second auto-hide timer after `set_compaction_reduction`.
//!   * The store has no per-session compaction-reduction seq map / accessors
//!     (`compaction_reduction_seq_for` / `bump_compaction_reduction_seq`).
//!
//! Each `#[test]` / `#[tokio::test]` maps 1:1 to a Scenario in the feature
//! file and reuses the fixture pattern from
//! `agentview_session_header_compaction_percentage_rpc100.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::ViewMode;
use codelet_fspec_tui::{Action, App, FspecBackend, SessionContext};
use codelet_rpc_types::{
    CompactionResult, ContextFillInfo, SessionId, SessionState, SessionStatus, StreamChunk,
};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

mod common;
use common::MockBackend;

// ───────────────────────── helpers ────────────────────────────────────────

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
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

// ───────────────────────── tests ──────────────────────────────────────────

/// Scenario: The COMPACTED badge auto-hides after the 10-second timer fires
#[tokio::test(start_paused = true)]
async fn compacted_badge_auto_hides_after_10_second_timer_fires() {
    // @step Given session "s-1" is open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_single_session();
    focus_s1(&mut app);

    // @step And Action::ChunkReceived("s-1", StreamChunk::ContextFillUpdate with fill_percentage 80) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::ContextFillUpdate {
            context_fill: context_fill(80),
        },
    ));

    // @step And Action::ChunkReceived("s-1", StreamChunk::CompactionComplete with compression_ratio 60.0) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: compaction_result(10000, 4000, 60.0),
        },
    ));

    // @step And the SessionHeader text contains "[80%: COMPACTED 60%]"
    let buf = render_app_buffer(&mut app);
    assert!(
        header_text(&buf).contains("[80%: COMPACTED 60%]"),
        "header should show '[80%: COMPACTED 60%]', got: {:?}",
        header_text(&buf)
    );

    // @step When the 10-second auto-hide timer elapses and the queued Action::ClearCompactionReduction for "s-1" is dispatched
    // Yield first so the spawned timer task starts and registers its
    // sleep against the paused clock BEFORE we advance virtual time.
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    drain_actions(&mut app);

    // @step And the App renders the AgentView into a 100x24 TestBackend
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "[80%]"
    assert!(
        header.contains("[80%]"),
        "after auto-hide header should show plain '[80%]', got: {header:?}"
    );
    // @step And the SessionHeader text does NOT contain "COMPACTED"
    assert!(
        !header.contains("COMPACTED"),
        "after auto-hide the COMPACTED suffix must be gone, got: {header:?}"
    );
}

/// Scenario: A stale auto-hide action is ignored when a newer compaction re-armed the timer
#[test]
fn stale_auto_hide_action_is_ignored_when_newer_compaction_rearmed() {
    // @step Given session "s-1" is open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_single_session();
    focus_s1(&mut app);

    // @step And Action::ChunkReceived("s-1", StreamChunk::CompactionComplete with compression_ratio 60.0) has been dispatched, recording its auto-hide sequence number
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: compaction_result(10000, 4000, 60.0),
        },
    ));
    drain_actions(&mut app);
    // Capture the REAL auto-hide seq recorded by the first compaction — this
    // is the sequence the (superseded) timer would have fired with. The
    // implementation bumps from 0 → 1 on the first compaction, so this is 1,
    // not a magic 0.
    let stale_seq = app
        .agent_view_store()
        .compaction_reduction_seq_for(&sid("s-1"));

    // @step And a second Action::ChunkReceived("s-1", StreamChunk::CompactionComplete with compression_ratio 70.0) has been dispatched, superseding the first with a newer auto-hide sequence
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: compaction_result(10000, 3000, 70.0),
        },
    ));
    drain_actions(&mut app);

    // @step When the now-superseded Action::ClearCompactionReduction for "s-1" (the earlier sequence) is dispatched
    app.dispatch(Action::ClearCompactionReduction {
        session_id: sid("s-1"),
        seq: stale_seq,
    });

    // @step And the App renders the AgentView into a 100x24 TestBackend
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "COMPACTED 70%"
    assert!(
        header.contains("COMPACTED 70%"),
        "the superseded earlier-seq clear must be a no-op so the newer 'COMPACTED 70%' survives, got: {header:?}"
    );
}

/// Scenario: Auto-hiding one session's badge does not affect another session's badge
#[test]
fn auto_hiding_one_session_badge_does_not_affect_another() {
    // @step Given two sessions "s-1" and "s-2" are open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_two_sessions();
    focus_s1(&mut app);

    // @step And Action::ChunkReceived("s-1", StreamChunk::CompactionComplete with compression_ratio 60.0) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: compaction_result(10000, 4000, 60.0),
        },
    ));
    drain_actions(&mut app);
    // @step And Action::ChunkReceived("s-2", StreamChunk::CompactionComplete with compression_ratio 70.0) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-2"),
        StreamChunk::CompactionComplete {
            compaction_result: compaction_result(10000, 3000, 70.0),
        },
    ));
    drain_actions(&mut app);

    // @step When the Action::ClearCompactionReduction for "s-1" is dispatched
    let seq_s1 = app
        .agent_view_store()
        .compaction_reduction_seq_for(&sid("s-1"));
    app.dispatch(Action::ClearCompactionReduction {
        session_id: sid("s-1"),
        seq: seq_s1,
    });

    // @step And the App renders the AgentView into a 100x24 TestBackend with "s-1" focused
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text does NOT contain "COMPACTED"
    assert!(
        !header.contains("COMPACTED"),
        "s-1's badge should be cleared, got: {header:?}"
    );

    // @step When the App dispatches Action::SessionNext to focus "s-2" and re-renders
    app.dispatch(Action::SessionNext);
    drain_actions(&mut app);
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "COMPACTED 70%"
    assert!(
        header.contains("COMPACTED 70%"),
        "s-2's badge must be unaffected by clearing s-1, got: {header:?}"
    );
}

/// Scenario: A /clear before 10 seconds hides the badge immediately and neutralises the pending timer
#[test]
fn clear_before_10s_hides_immediately_and_neutralises_pending_timer() {
    // @step Given session "s-1" is open in AgentView with "s-1" focused
    let (mut app, _mock) = agent_app_with_single_session();
    focus_s1(&mut app);

    // @step And Action::ChunkReceived("s-1", StreamChunk::CompactionComplete with compression_ratio 60.0) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: compaction_result(10000, 4000, 60.0),
        },
    ));
    drain_actions(&mut app);
    let pre_clear_seq = app
        .agent_view_store()
        .compaction_reduction_seq_for(&sid("s-1"));

    // @step When Action::ChunkReceived("s-1", StreamChunk::SessionStateChange { state: Cleared }) is dispatched
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
        "after Cleared the header should show '[0%]', got: {header:?}"
    );
    // @step And the SessionHeader text does NOT contain "COMPACTED"
    assert!(
        !header.contains("COMPACTED"),
        "after Cleared the compaction suffix must be gone, got: {header:?}"
    );

    // @step When the original pending Action::ClearCompactionReduction for "s-1" with the pre-clear seq is dispatched
    app.dispatch(Action::ClearCompactionReduction {
        session_id: sid("s-1"),
        seq: pre_clear_seq,
    });

    // @step And the App renders the AgentView into a 100x24 TestBackend
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then the SessionHeader text contains "[0%]"
    assert!(
        header.contains("[0%]"),
        "the stale pre-clear timer must be a no-op leaving '[0%]', got: {header:?}"
    );
}

/// Scenario: Under paused tokio time the real timer arms and emits the clear action at 10 seconds
#[tokio::test(start_paused = true)]
async fn paused_tokio_time_real_timer_arms_and_emits_clear_at_10s() {
    // @step Given session "s-1" is open in AgentView with "s-1" focused under a start_paused tokio runtime
    let (mut app, _mock) = agent_app_with_single_session();
    focus_s1(&mut app);

    // @step And Action::ChunkReceived("s-1", StreamChunk::CompactionComplete with compression_ratio 60.0) has been dispatched
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::CompactionComplete {
            compaction_result: compaction_result(10000, 4000, 60.0),
        },
    ));

    // @step When the virtual clock is advanced by 10 seconds
    // Yield first so the spawned timer task registers its sleep against
    // the paused clock before we advance virtual time past it.
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;

    // @step And the pending actions on the App action channel are drained and dispatched
    let mut saw_clear = false;
    while let Some(action) = app.try_recv_action() {
        if matches!(
            &action,
            Action::ClearCompactionReduction { session_id, .. } if session_id == &sid("s-1")
        ) {
            saw_clear = true;
        }
        app.dispatch(action);
    }

    // @step And the App renders the AgentView into a 100x24 TestBackend
    let buf = render_app_buffer(&mut app);
    let header = header_text(&buf);

    // @step Then an Action::ClearCompactionReduction for "s-1" was emitted on the channel
    assert!(
        saw_clear,
        "the armed 10s timer must have emitted Action::ClearCompactionReduction for s-1"
    );
    // @step And the SessionHeader text does NOT contain "COMPACTED"
    assert!(
        !header.contains("COMPACTED"),
        "after the timer fired the COMPACTED suffix must be gone, got: {header:?}"
    );
}
