// Feature: spec/features/exec-stdin-push-tui-composer.feature
//! BUG-171 — TUI acceptance tests for the PUSH trigger: the exec-stdin
//! stdin notice must surface from a `StreamChunk::ExecStdinRequest`
//! arriving on the chunk stream while the session stays Running — no
//! focus switch, no Paused state change.
//!
//! Covers the TUI-layer scenarios of the BUG-171 feature file:
//! - request chunk → `handle_exec_stdin_prompt_fetched` → slot populated
//!   → notice visible (session still Running)
//! - cleared chunk → `Action::ExecStdinDismissed` → slot cleared,
//!   nothing sent to the backend
//! - both variants are state-only (no scrollback chunks)
//! - the pre-existing pull probe (Paused chunk) still works (parity)
//!
//! Harness: `exec_stdin_push_chunk_bug171_support` (fresh_app + MockBackend +
//! render_app_frame + drain_pending).

use codelet_fspec_tui::{Action, SessionContext};
use codelet_rpc_types::StreamChunk;
use codelet_test_helpers::TestResult;

mod common;
mod exec_stdin_push_chunk_bug171_support;
use exec_stdin_push_chunk_bug171_support::*;

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Exec-stdin request chunk shows the stdin notice while
// the session stays Running
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Exec-stdin request chunk shows the stdin notice while the session stays Running
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exec_stdin_request_chunk_shows_the_stdin_notice() -> TestResult {
    // @step Given the agent session is Running with no exec-stdin slot
    let (mut app, _mock) = fresh_app(&["s-1"]);
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_none(),
        "slot must start empty"
    );

    // @step When an exec-stdin request StreamChunk arrives for that session
    // (delivered the way the chunks subscriber forwards broadcast chunks:
    //  Action::ChunkReceived → handle_stream_chunk_state_updates)
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::ExecStdinRequest {
            request: exec_request("exec-abc", "git commit", 5),
        },
    ));
    drain_pending(&mut app).await?;

    // @step Then the stdin notice is visible in the focused pane's input area
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_some(),
        "the request chunk must populate the exec-stdin slot"
    );
    let rows = render_app_frame(&mut app);
    assert!(
        rows.iter()
            .any(|r| r.contains("git commit may be waiting for input")),
        "the notice must paint; rows: {rows:?}"
    );

    // @step And the slot precedence is respected (HITL > exec-stdin > pause > composer)
    // With only the exec-stdin slot set, the notice shows over the
    // plain composer; no HITL/pause slot is present for this scenario.
    assert!(
        app.agent_view_store()
            .hitl_prompt_for(&sid("s-1"))
            .is_none(),
        "no HITL slot may be present for this scenario"
    );
    assert!(
        app.agent_view_store()
            .pause_state_for(&sid("s-1"))
            .is_none(),
        "no pause slot may be present for this scenario"
    );

    // @step And no Paused state change chunk was emitted
    // The request-chunk reducer must not synthesize a status flip — the
    // pause slot stays empty.
    assert!(
        app.agent_view_store()
            .pause_state_for(&sid("s-1"))
            .is_none(),
        "the request chunk must not flip the session into a paused state"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: The cleared chunk clears the TUI slot without sending anything
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: The cleared chunk clears the TUI slot without sending anything
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_cleared_chunk_clears_the_tui_slot_without_sending_anything() -> TestResult {
    // @step Given the stdin notice is visible for a session
    let (mut app, mock) = fresh_app(&["s-1"]);
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::ExecStdinRequest {
            request: exec_request("exec-abc", "git commit", 5),
        },
    ));
    drain_pending(&mut app).await?;
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_some(),
        "the notice must be visible before the cleared chunk"
    );

    // @step When an exec-stdin cleared StreamChunk arrives for that session
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::ExecStdinRequestCleared,
    ));
    drain_pending(&mut app).await?;

    // @step Then the exec-stdin slot is cleared
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_none(),
        "the cleared chunk must clear the slot"
    );
    let rows = render_app_frame(&mut app);
    assert!(
        !rows.iter().any(|r| r.contains("may be waiting for input")),
        "the notice must not paint after the clear; rows: {rows:?}"
    );

    // @step And nothing was written to any exec session stdin
    assert_eq!(
        mock.write_exec_stdin_calls(),
        0,
        "the cleared chunk must not trigger a write_exec_stdin"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: State-only chunks never land in the transcript scrollback
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: State-only chunks never land in the transcript scrollback
#[test]
fn state_only_chunks_never_land_in_the_transcript_scrollback() -> TestResult {
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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Existing pull probe sites still surface and clear the notice
// (regression guard — the Paused-chunk probe path keeps working)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: Existing pull probe sites still surface and clear the notice
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn existing_pull_probe_sites_still_surface_and_clear_the_notice() -> TestResult {
    // @step Given a Running agent session with a live quiet exec session
    // (two sessions; s-2 is focused after SessionCreated ordering, s-1
    //  carries the pending exec-stdin request)
    let (mut app, mock) = fresh_app(&["s-1", "s-2"]);
    mock.script_exec_stdin_request(sid("s-1"), Some(exec_request("exec-live", "git commit", 4)));

    // @step When the user switches focus away and back
    // (SessionPrev moves focus s-2 → s-1 and runs the focus-switch probe
    //  — probe_exec_stdin_for — on the incoming session)
    app.dispatch(Action::SessionPrev);
    drain_pending(&mut app).await?;

    // @step Then the notice is re-probed on focus return
    wait_until(
        || mock.get_exec_stdin_request_calls() >= 1,
        "focus-switch probe hit the backend",
    )
    .await?;
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_some(),
        "the focus-switch probe must surface the pending request"
    );

    // @step And a Paused state change probe still reads the pending request
    // (re-probe on Paused chunk → get_exec_stdin_request Some → slot
    //  stays populated via ExecStdinProbeFetched)
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::SessionStateChange {
            state: codelet_rpc_types::SessionState::Paused,
        },
    ));
    drain_pending(&mut app).await?;
    wait_until(
        || mock.get_exec_stdin_request_calls() >= 2,
        "Paused probe re-read the backend",
    )
    .await?;
    assert!(
        exec_stdin_slot(&app, &sid("s-1")).is_some(),
        "the Paused-state probe must keep the slot populated"
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: End-to-end — interactive command surfaces the notice without
// a focus switch or status flip (TUI half of the integration scenario;
// the sessions half is covered by
// rust/sessions/tests/exec_stdin_push_chunk_bug171.rs)
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: End-to-end — interactive command surfaces the notice without a focus switch or status flip
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn end_to_end_interactive_command_surfaces_the_notice_without_a_focus_switch_or_status_flip(
) -> TestResult {
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
    drain_pending(&mut app).await?;

    // @step When the command goes quiet for at least the detector threshold
    // (carried by the request chunk's quiet_seconds)
    assert_eq!(
        exec_stdin_slot(&app, &sid("s-1")).map(|r| r.quiet_seconds),
        Some(5),
        "the request must carry the detector's quiet seconds"
    );

    // @step Then the stdin notice appears in the focused pane without any session switch or status change
    let rows = render_app_frame(&mut app);
    assert!(
        rows.iter()
            .any(|r| r.contains("cat may be waiting for input")),
        "the notice must paint in the focused pane; rows: {rows:?}"
    );
    assert!(
        app.agent_view_store()
            .pause_state_for(&sid("s-1"))
            .is_none(),
        "no status/pause change may accompany the notice"
    );

    // @step And opening the answer box with Ctrl+O, typing a value and pressing Enter sends the value plus newline to the command stdin
    app.handle_event(&crossterm::event::Event::Key(
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('o'),
            crossterm::event::KeyModifiers::CONTROL,
        ),
    ));
    drain_pending(&mut app).await?;
    render_app_frame(&mut app);
    app.handle_event(&key_event(crossterm::event::KeyCode::Char('y')));
    drain_pending(&mut app).await?;
    app.handle_event(&key_event(crossterm::event::KeyCode::Enter));
    drain_pending(&mut app).await?;

    wait_until(
        || mock.write_exec_stdin_calls() == 1,
        "write_exec_stdin called on Enter",
    )
    .await?;
    let log = mock.write_exec_stdin_log();
    assert_eq!(log.len(), 1, "exactly one write expected");
    let (agent_session, exec_session, text) = &log[0];
    assert_eq!(agent_session, &sid("s-1"));
    assert_eq!(exec_session, "exec-abc");
    assert_eq!(text, "y");
    Ok(())
}
