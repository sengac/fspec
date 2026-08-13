//! RPC-046 — `/clear` slash command end-to-end (Phase 6.4 of RPC-030).
//!
//! Feature: spec/features/slash-command-clear.feature
//! Feature: spec/features/rpc074-clear-ts-parity.feature
//!
//! Drives the App::dispatch routing for `SlashCommandSelected(Clear)` so
//! it reaches PAST the local scrollback reset and into the backend's
//! `clear_history(session_id)` RPC.
//!
//! TS PARITY (RPC-074): The Rust port mirrors
//! `src/tui/components/AgentView.tsx:1554-1564` (handleClearCommand). On
//! both Ok and Err paths NO scrollback notice line is appended — the
//! reactive UI reset comes from a `StreamChunk::SessionStateChange {
//! state: Cleared }` chunk emitted by `BackgroundSession::clear_history`
//! (matches the TS TUI-066 contract). Errors go to `tracing::error!`
//! only. With no current session, /clear is a silent no-op. Background
//! sessions are never touched.
//!
//! The original RPC-046 success-/error-notice scenarios that asserted
//! `[notice] /clear: history cleared` and `[error] /clear failed: <e>`
//! lines were retired in RPC-074 — those strings were pure Rust-side
//! invention with no counterpart in the TS reference.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend, RenderedChunk};
use codelet_rpc_types::SessionId;
use ratatui::text::Line;
use tokio::time::timeout;

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

/// Push `count` raw scrollback chunks into the SessionContext for `id`.
/// Mirrors the `push_chunk` helper in `view_agent_popups_rpc020.rs` but
/// targets a specific session id rather than the focused one.
fn seed_chunks(app: &mut App, id: &SessionId, count: usize) {
    let ctx = app
        .agent_view_store_mut()
        .session_context_mut_for(id)
        .expect("SessionContext present for seeded id");
    for i in 0..count {
        ctx.scrollback.push(RenderedChunk {
            seq: i as u64,
            lines: vec![Line::from(format!("seed-{i}"))],
            source: None,
        });
    }
}

/// Flatten every visible chunk's text in `id`'s scrollback into a single
/// String. Mirrors the `scrollback_text` helper in
/// `view_agent_popups_rpc020.rs` but takes a session id so we can read
/// background-session scrollback too.
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

/// Poll-until-true helper for spawned-task assertions. Mirrors the
/// `timeout(Duration::from_secs(1), async { loop { … sleep(10ms) } })`
/// idiom from RPC-045 tests.
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

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /clear resets local scrollback synchronously for the focused
// session
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn clear_resets_local_scrollback_synchronously_for_focused_session() {
    // @step Given an App with an open session s-1 whose scrollback has 5 chunks
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    seed_chunks(&mut app, &sid("s-1"), 5);
    app.navigator_mut().agent.input.set_value("/clear");
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 5);

    // @step When SlashCommandSelected(SlashCommandAction::Clear) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Clear));

    // @step Then s-1's scrollback chunk_count becomes 0 synchronously
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 0);

    // @step And the MultiLineInput's buffer is empty
    assert!(app.navigator().agent.input.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /clear calls backend.clear_history for the focused session
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clear_calls_backend_clear_history_for_focused_session() {
    // @step Given an App with an open session s-1 wired to a MockBackend whose clear_history returns Ok(())
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    assert_eq!(mock.clear_history_calls(), 0);

    // @step When SlashCommandSelected(SlashCommandAction::Clear) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Clear));

    // @step Then within 1 second backend.clear_history is called exactly once with session_id s-1
    wait_until(
        || mock.clear_history_calls() == 1,
        "backend.clear_history call count to reach 1",
    )
    .await;
    assert_eq!(mock.clear_history_calls(), 1);
    assert_eq!(mock.last_clear_history_session(), Some(sid("s-1")));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario (RPC-074): /clear resets scrollback but does NOT append any
// `[notice] /clear: history cleared` line. TS handleClearCommand only
// blanks the input + calls backend.clear_history; no scrollback notice
// line is ever pushed by the dispatcher.
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clear_appends_no_notice_line_on_ok() {
    // @step Given an App with an open session s-1 with 3 scrollback chunks wired to a MockBackend whose clear_history returns Ok(())
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    seed_chunks(&mut app, &sid("s-1"), 3);

    // @step When SlashCommandSelected(SlashCommandAction::Clear) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Clear));

    // @step Then s-1's scrollback chunk_count becomes 0 synchronously
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 0);

    drain_pending(&mut app).await;

    // @step Then after draining pending tasks and the action bus, s-1's scrollback contains zero lines matching "[notice] /clear"
    let text = session_scrollback_text(&app, &sid("s-1"));
    assert!(
        text.lines().all(|l| !l.contains("[notice] /clear")),
        "TS parity (RPC-074): no `[notice] /clear ...` line should appear in scrollback, got {text:?}",
    );

    // @step Then after draining pending tasks and the action bus, s-1's scrollback contains zero lines matching "history cleared"
    assert!(
        text.lines().all(|l| !l.contains("history cleared")),
        "TS parity (RPC-074): no `history cleared` line should appear in scrollback, got {text:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario (RPC-074): /clear with backend Err does NOT append any
// `[error] /clear failed: <reason>` line — TS routes errors to
// logger.error only, Rust routes to `tracing::error!` only.
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clear_appends_no_error_line_on_err() {
    // @step Given an App with an open session s-1 wired to a MockBackend whose clear_history returns Err("boom")
    let mock = Arc::new(MockBackend::new());
    mock.set_clear_history_error("boom".to_string());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When SlashCommandSelected(SlashCommandAction::Clear) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Clear));

    drain_pending(&mut app).await;

    // @step Then after draining pending tasks and the action bus, s-1's scrollback contains zero lines matching "[error] /clear failed"
    let text = session_scrollback_text(&app, &sid("s-1"));
    assert!(
        text.lines().all(|l| !l.contains("[error] /clear failed")),
        "TS parity (RPC-074): backend Err must NOT push `[error] /clear failed: ...` into scrollback (errors go to tracing::error!), got {text:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /clear with no current session is a silent no-op
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clear_with_no_current_session_is_a_silent_no_op() {
    // @step Given an App with NO current session
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    assert!(app.agent_view_store().current_session().is_none());

    // @step When SlashCommandSelected(SlashCommandAction::Clear) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Clear));

    // Give any (incorrectly) spawned task a chance to run before we
    // assert the no-op property.
    tokio::time::sleep(Duration::from_millis(100)).await;
    drain_pending(&mut app).await;

    // @step Then backend.clear_history is never called
    assert_eq!(
        mock.clear_history_calls(),
        0,
        "clear_history must NOT be called when there is no current session",
    );

    // @step And no scrollback chunk is appended to any session
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        0,
        "no sessions should exist for the no-op assertion",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /clear only affects the focused session — background sessions
// are untouched
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clear_only_affects_focused_session_background_untouched() {
    // @step Given an App with two open sessions s-1 (focused) and s-2 (background), each with 3 scrollback chunks
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    // s-2 was created second → it is currently focused. Cycle back to s-1
    // so s-1 is focused and s-2 is the background tab.
    app.dispatch(Action::SessionPrev);
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-1")));
    seed_chunks(&mut app, &sid("s-1"), 3);
    seed_chunks(&mut app, &sid("s-2"), 3);
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 3);
    assert_eq!(session_chunk_count(&app, &sid("s-2")), 3);
    // @step And the MockBackend's clear_history returns Ok(())
    // (default impl of MockBackend.clear_history)

    // @step When SlashCommandSelected(SlashCommandAction::Clear) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Clear));

    // @step Then s-1's scrollback chunk_count becomes 0 synchronously
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 0);

    // @step And s-2's scrollback chunk_count remains 3
    assert_eq!(session_chunk_count(&app, &sid("s-2")), 3);

    // @step And within 1 second backend.clear_history is called exactly once with session_id s-1
    wait_until(
        || mock.clear_history_calls() == 1,
        "backend.clear_history call count to reach 1",
    )
    .await;
    assert_eq!(mock.clear_history_calls(), 1);
    assert_eq!(mock.last_clear_history_session(), Some(sid("s-1")));

    drain_pending(&mut app).await;

    // @step And s-1's scrollback contains zero lines matching "[notice] /clear" (TS parity, RPC-074)
    let s1_text = session_scrollback_text(&app, &sid("s-1"));
    assert!(
        s1_text.lines().all(|l| !l.contains("[notice] /clear")),
        "TS parity: s-1 must NOT contain a `[notice] /clear ...` line, got {s1_text:?}",
    );

    // @step And s-2's scrollback also contains zero lines matching "[notice] /clear"
    let s2_text = session_scrollback_text(&app, &sid("s-2"));
    assert!(
        s2_text.lines().all(|l| !l.contains("[notice] /clear")),
        "TS parity: s-2 must NOT contain a `[notice] /clear ...` line, got {s2_text:?}",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// drain_pending — await every spawned tokio task AND fold any queued
// action_tx messages back into the App. Mirrors the same-named helper
// in `slash_command_wiring_rpc022.rs`.
// ─────────────────────────────────────────────────────────────────────────

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
