// Feature: spec/features/exec-stdin-push-tui-composer.feature
//! BUG-171 — TUI acceptance tests for the PUSH trigger: the exec-stdin
//! composer overlay must surface from a `StreamChunk::ExecStdinRequest`
//! arriving on the chunk stream while the session stays Running — no
//! focus switch, no Paused state change.
//!
//! Covers the TUI-layer scenarios of the BUG-171 feature file:
//! - request chunk → `Action::ExecStdinPromptFetched` → slot populated
//!   → overlay visible (session still Running)
//! - cleared chunk → `Action::ExecStdinDismissed` → slot cleared,
//!   nothing sent to the backend
//! - both variants are state-only (no scrollback chunks)
//! - the pre-existing pull probe (Paused chunk) still works (parity)
//!
//! Harness mirrors `exec_stdin_prompt_p2.rs` (fresh_app + MockBackend +
//! render_app_frame + drain_pending).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend, SessionContext, ViewMode};
use codelet_rpc_types::{ExecStdinRequest, SessionId, StreamChunk};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn exec_request(exec_id: &str, command: &str, quiet_seconds: i64) -> ExecStdinRequest {
    ExecStdinRequest {
        exec_session_id: exec_id.to_string(),
        command: command.to_string(),
        quiet_seconds,
        ts_ms: 1_700_000_000_000,
    }
}

fn key_event(code: crossterm::event::KeyCode) -> crossterm::event::Event {
    crossterm::event::Event::Key(crossterm::event::KeyEvent {
        code,
        modifiers: crossterm::event::KeyModifiers::NONE,
        kind: crossterm::event::KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    })
}

fn fresh_app(sessions: &[&str]) -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    for s in sessions {
        app.dispatch(Action::SessionCreated(sid(s)));
    }
    app.navigator_mut().active_view = ViewMode::Agent;
    (app, mock)
}

fn render_app_frame(app: &mut App) -> Vec<String> {
    let area = Rect::new(0, 0, 100, 20);
    let mut buf = Buffer::empty(area);
    app.render(area, &mut buf);
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect()
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

async fn wait_until<F: FnMut() -> bool>(mut predicate: F, label: &str) {
    timeout(Duration::from_secs(1), async {
        loop {
            if predicate() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for: {label}"));
}

fn exec_stdin_slot<'a>(app: &'a App, session: &SessionId) -> Option<&'a ExecStdinRequest> {
    app.agent_view_store().exec_stdin_for(session)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Exec-stdin request chunk populates the composer overlay while
// the session stays Running
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Exec-stdin request chunk populates the composer overlay while the session stays Running
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_stdin_request_chunk_populates_the_composer_overlay() {
    // @step Given the agent session is Running with no exec-stdin slot
    let (mut app, _mock) = fresh_app(&["s-1"]);
    assert!(exec_stdin_slot(&app, &sid("s-1")).is_none(), "slot must start empty");

    // @step When an exec-stdin request StreamChunk arrives for that session
    // (delivered the way the chunks subscriber forwards broadcast chunks:
    //  Action::ChunkReceived → handle_stream_chunk_state_updates)
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::ExecStdinRequest {
            request: exec_request("exec-abc", "git commit", 5),
        },
    ));
    drain_pending(&mut app).await;

    // @step Then the exec-stdin composer overlay is visible in the focused pane's input area
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_some(),
        "the request chunk must populate the exec-stdin slot"
    );
    let rows = render_app_frame(&mut app);
    assert!(
        rows.iter().any(|r| r.contains("⌨ git commit")),
        "overlay header must paint; rows: {rows:?}"
    );

    // @step And the slot precedence is respected (HITL > exec-stdin > pause > composer)
    // With only the exec-stdin slot set, the overlay wins over the
    // plain composer; no HITL/pause slot is present for this scenario.
    assert!(
        app.agent_view_store().hitl_prompt_for(&sid("s-1")).is_none(),
        "no HITL slot may be present for this scenario"
    );
    assert!(
        app.agent_view_store().pause_state_for(&sid("s-1")).is_none(),
        "no pause slot may be present for this scenario"
    );

    // @step And no Paused state change chunk was emitted
    // The request-chunk reducer must not synthesize a status flip — the
    // pause slot stays empty.
    assert!(
        app.agent_view_store().pause_state_for(&sid("s-1")).is_none(),
        "the request chunk must not flip the session into a paused state"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The cleared chunk clears the TUI slot without sending anything
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: The cleared chunk clears the TUI slot without sending anything
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_cleared_chunk_clears_the_tui_slot_without_sending_anything() {
    // @step Given the exec-stdin composer overlay is visible for a session
    let (mut app, mock) = fresh_app(&["s-1"]);
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::ExecStdinRequest {
            request: exec_request("exec-abc", "git commit", 5),
        },
    ));
    drain_pending(&mut app).await;
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_some(),
        "the overlay must be visible before the cleared chunk"
    );

    // @step When an exec-stdin cleared StreamChunk arrives for that session
    app.dispatch(Action::ChunkReceived(sid("s-1"), StreamChunk::ExecStdinRequestCleared));
    drain_pending(&mut app).await;

    // @step Then the exec-stdin slot is cleared
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_none(),
        "the cleared chunk must clear the slot"
    );
    let rows = render_app_frame(&mut app);
    assert!(
        !rows.iter().any(|r| r.contains("has been quiet for")),
        "the overlay header must not paint after the clear; rows: {rows:?}"
    );

    // @step And nothing was written to any exec session stdin
    assert_eq!(
        mock.write_exec_stdin_calls(),
        0,
        "the cleared chunk must not trigger a write_exec_stdin"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: State-only chunks never land in the transcript scrollback
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: State-only chunks never land in the transcript scrollback
#[test]
fn state_only_chunks_never_land_in_the_transcript_scrollback() {
    // @step Given a session context that records chunks
    let mut ctx = SessionContext::new(sid("s-1"));

    // @step When an exec-stdin request chunk and a cleared chunk are recorded for the session
    ctx.record_chunk(&StreamChunk::ExecStdinRequest {
        request: exec_request("exec-abc", "git commit", 5),
    });
    ctx.record_chunk(&StreamChunk::ExecStdinRequestCleared);

    // @step Then no scrollback chunk is created for either variant
    assert_eq!(
        ctx.scrollback.chunks().len(),
        0,
        "state-only exec-stdin chunks must not create scrollback chunks"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Existing pull probe sites still surface and clear the overlay
// (regression guard — the Paused-chunk probe path keeps working)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Existing pull probe sites still surface and clear the overlay
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn existing_pull_probe_sites_still_surface_and_clear_the_overlay() {
    // @step Given a Running agent session with a live quiet exec session
    // (two sessions; s-2 is focused after SessionCreated ordering, s-1
    //  carries the pending exec-stdin request)
    let (mut app, mock) = fresh_app(&["s-1", "s-2"]);
    mock.script_exec_stdin_request(
        sid("s-1"),
        Some(exec_request("exec-live", "git commit", 4)),
    );

    // @step When the user switches focus away and back
    // (SessionPrev moves focus s-2 → s-1 and runs the focus-switch probe
    //  — probe_exec_stdin_for — on the incoming session)
    app.dispatch(Action::SessionPrev);
    drain_pending(&mut app).await;

    // @step Then the overlay is re-probed on focus return
    wait_until(
        || mock.get_exec_stdin_request_calls() >= 1,
        "focus-switch probe hit the backend",
    )
    .await;
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_some(),
        "the focus-switch probe must surface the pending request"
    );

    // @step And a Paused state change probe still reads the pending request
    // (re-probe on Paused chunk → get_exec_stdin_request Some → slot
    //  stays populated via ExecStdinPromptFetched)
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: codelet_rpc_types::SessionState::Paused,
        },
    ));
    drain_pending(&mut app).await;
    wait_until(
        || mock.get_exec_stdin_request_calls() >= 2,
        "Paused probe re-read the backend",
    )
    .await;
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_some(),
        "the Paused-state probe must keep the slot populated"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: End-to-end — interactive command surfaces the overlay without
// a focus switch or status flip (TUI half of the integration scenario;
// the sessions half is covered by
// rust/sessions/tests/exec_stdin_push_chunk_bug171.rs)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: End-to-end — interactive command surfaces the overlay without a focus switch or status flip
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn end_to_end_interactive_command_surfaces_the_overlay_without_a_focus_switch_or_status_flip() {
    // @step Given the agent runs an interactive Bash command that reads stdin and the session stays focused and Running
    let (mut app, mock) = fresh_app(&["s-1"]);
    // The chunk stream delivers the push (the sessions half of the
    // scenario proves the detector → stored request → chunk path).
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::ExecStdinRequest {
            request: exec_request("exec-abc", "cat", 5),
        },
    ));
    drain_pending(&mut app).await;

    // @step When the command goes quiet for at least the detector threshold
    // (carried by the request chunk's quiet_seconds)
    assert_eq!(
        exec_stdin_slot(&app, &sid("s-1")).map(|r| r.quiet_seconds),
        Some(5),
        "the request must carry the detector's quiet seconds"
    );

    // @step Then the exec-stdin composer overlay appears in the focused pane without any session switch or status change
    let rows = render_app_frame(&mut app);
    assert!(
        rows.iter().any(|r| r.contains("⌨ cat")),
        "overlay must paint in the focused pane; rows: {rows:?}"
    );
    assert!(
        app.agent_view_store().pause_state_for(&sid("s-1")).is_none(),
        "no status/pause change may accompany the overlay"
    );

    // @step And typing a value and pressing Enter sends the value plus newline to the command stdin
    app.handle_event(&key_event(crossterm::event::KeyCode::Char('y')));
    drain_pending(&mut app).await;
    app.handle_event(&key_event(crossterm::event::KeyCode::Enter));
    drain_pending(&mut app).await;

    wait_until(
        || mock.write_exec_stdin_calls() == 1,
        "write_exec_stdin called on Enter",
    )
    .await;
    let log = mock.write_exec_stdin_log();
    assert_eq!(log.len(), 1, "exactly one write expected");
    let (agent_session, exec_session, text) = &log[0];
    assert_eq!(agent_session, &sid("s-1"));
    assert_eq!(exec_session, "exec-abc");
    assert_eq!(text, "y");
}
