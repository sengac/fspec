//! RPC-049 — `/resume` durable restore via `restore_session_messages` +
//! `restore_session_token_state` (Phase 6.4 of RPC-030).
//!
//! Feature: spec/features/slash-command-resume.feature
//!
//! Drives the App::dispatch routing for `Action::AttachToSession` so it
//! reaches PAST the open_sessions move/append into a spawned
//! `backend.resume_session(session_id)` round-trip. On success the
//! follow-up `Action::SessionResumeComplete(id)` triggers a second
//! spawned task that calls `backend.get_buffered_output(id, 1000)` and
//! replays each returned chunk as `Action::ChunkReceived(id, chunk)` —
//! so the resumed session's scrollback is seeded from the backend's
//! replay buffer. On failure the error is surfaced via
//! `Action::EmitSessionNotice` to the originating session.
//!
//! Mirrors the spawned-task + action-bus round-trip pattern established
//! by `dispatch_slash_commands.rs::handle_slash_command(Clear/Compact)` and
//! `dispatch_resume_search_views.rs::handle_confirm_delete_session`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk};
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Flatten every visible chunk's text in `id`'s scrollback into a single
/// String. Mirrors the helper in `slash_clear_rpc046.rs`.
fn session_scrollback_text(app: &App, id: &SessionId) -> String {
    let chunks = app
        .agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default();
    chunks
        .iter()
        .flat_map(|c| {
            c.lines.iter().map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn session_chunk_count(app: &App, id: &SessionId) -> usize {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.chunk_count())
        .unwrap_or(0)
}

/// Poll-until-true helper for spawned-task assertions.
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

/// Await every spawned tokio task AND fold any queued action_tx messages
/// back into the App. Mirrors the helper in `slash_clear_rpc046.rs`.
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

// ─────────────────────────────────────────────────────────────────────────
// Scenario: AttachToSession spawns backend.resume_session for the
// selected session
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attach_to_session_spawns_backend_resume_session() {
    // @step Given an App wired to a MockBackend with sessions ["s-1", "s-2"]
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    assert_eq!(mock.resume_session_calls(), 0);

    // @step And ResumeSessionView is open with row 0 (s-1) selected
    // (Modelled by directly dispatching the AttachToSession action — the
    // ResumeSessionView's Enter handler dispatches exactly this action.)

    // @step When the user presses Enter on the resume view
    app.dispatch(Action::AttachToSession(sid("s-1")));

    // @step Then within 1 second backend.resume_session is called exactly once with session_id s-1
    wait_until(
        || mock.resume_session_calls() == 1,
        "backend.resume_session call count to reach 1",
    )
    .await;
    assert_eq!(mock.resume_session_calls(), 1);
    assert_eq!(mock.last_resume_session(), Some(sid("s-1")));

    // @step And the AgentViewStore's open_sessions contains s-1
    assert!(app
        .agent_view_store()
        .open_sessions()
        .iter()
        .any(|c| c.id == sid("s-1")));

    // @step And the AgentViewStore's current_session is s-1
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-1")));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: SessionResumeComplete seeds scrollback from
// get_buffered_output on Ok
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_resume_complete_seeds_scrollback_from_buffered_output() {
    // @step Given an App wired to a MockBackend whose resume_session returns Ok(())
    let mock = Arc::new(MockBackend::new());

    // @step And the MockBackend's buffered_output for s-1 is [StreamChunk::text("hello"), StreamChunk::text("world")]
    mock.set_buffered_output(vec![
        StreamChunk::text("hello".to_string()),
        StreamChunk::text("world".to_string()),
    ]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When Action::AttachToSession(s-1) is dispatched
    app.dispatch(Action::AttachToSession(sid("s-1")));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.resume_session is called exactly once with session_id s-1
    assert_eq!(mock.resume_session_calls(), 1);
    assert_eq!(mock.last_resume_session(), Some(sid("s-1")));

    // @step And within 1 second backend.get_buffered_output is called exactly once with session_id s-1 and limit 1000
    assert_eq!(mock.get_buffered_output_calls(), 1);
    assert_eq!(
        mock.last_get_buffered_output(),
        Some((sid("s-1"), 1000_u32))
    );

    // @step And within 1 second the SessionContext for s-1 contains a scrollback chunk whose text equals "hello"
    let text = session_scrollback_text(&app, &sid("s-1"));
    assert!(
        text.lines().any(|l| l.contains("hello")),
        "expected `hello` text in s-1 scrollback, got {text:?}",
    );

    // @step And within 1 second the SessionContext for s-1 contains a scrollback chunk whose text equals "world"
    assert!(
        text.lines().any(|l| l.contains("world")),
        "expected `world` text in s-1 scrollback, got {text:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: SessionResumeFailed emits an error notice and skips
// get_buffered_output
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_resume_failed_emits_error_notice_and_skips_buffered_output() {
    // @step Given an App wired to a MockBackend whose resume_session returns Err("corrupt manifest")
    let mock = Arc::new(MockBackend::new());
    mock.set_resume_session_error("corrupt manifest".to_string());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When Action::AttachToSession(s-1) is dispatched
    app.dispatch(Action::AttachToSession(sid("s-1")));
    drain_pending(&mut app).await;

    // @step Then within 1 second the SessionContext for s-1 contains a scrollback chunk whose text equals "[error] /resume failed: corrupt manifest"
    let text = session_scrollback_text(&app, &sid("s-1"));
    assert!(
        text.lines()
            .any(|l| l == "[error] /resume failed: corrupt manifest"),
        "expected `[error] /resume failed: corrupt manifest` in s-1 scrollback, got {text:?}",
    );

    // @step And backend.get_buffered_output is NEVER called
    assert_eq!(
        mock.get_buffered_output_calls(),
        0,
        "get_buffered_output must not run when resume_session fails",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Idempotent re-restore — AttachToSession on an already-open
// session still calls resume_session
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idempotent_re_restore_attach_on_open_session_still_calls_resume_session() {
    // @step Given an App with open_sessions [s-1] and current_session s-1
    let mock = Arc::new(MockBackend::new());

    // @step And the MockBackend's resume_session returns Ok(())
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    let initial_resume_calls = mock.resume_session_calls();

    // @step When Action::AttachToSession(s-1) is dispatched
    app.dispatch(Action::AttachToSession(sid("s-1")));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.resume_session is called exactly once with session_id s-1
    assert_eq!(mock.resume_session_calls() - initial_resume_calls, 1);
    assert_eq!(mock.last_resume_session(), Some(sid("s-1")));

    // @step And open_sessions length stays 1 (no duplicate append)
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        1,
        "open_sessions should NOT have duplicate entries for s-1",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Bonus sanity check: success path doesn't leak an error notice
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn success_path_does_not_emit_error_notice() {
    let mock = Arc::new(MockBackend::new());
    mock.set_buffered_output(vec![]);
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    app.dispatch(Action::AttachToSession(sid("s-1")));
    drain_pending(&mut app).await;

    let text = session_scrollback_text(&app, &sid("s-1"));
    assert!(
        text.lines()
            .all(|l| !l.starts_with("[error] /resume failed:")),
        "success path must NOT push an error notice into scrollback, got {text:?}",
    );
    // And the chunk count is 0 since buffered_output was empty.
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 0);
}
