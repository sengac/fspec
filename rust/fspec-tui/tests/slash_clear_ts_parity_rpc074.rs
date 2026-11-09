//! Feature: spec/features/rpc074-clear-ts-parity.feature
//!
//! Moved out of slash_clear_rpc046.rs (COV-075: one test file per feature).

use crate::common::TaskOutcome;
use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend, RenderedChunk};
use codelet_rpc_types::SessionId;
use codelet_test_helpers::{OrFail, TestResult};
use ratatui::text::Line;
use std::sync::Arc;
mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Push `count` raw scrollback chunks into the SessionContext for `id`.
/// Mirrors the `push_chunk` helper in `view_agent_popups_rpc020.rs` but
/// targets a specific session id rather than the focused one.
fn seed_chunks(
    app: &mut App,
    id: &SessionId,
    count: usize,
) -> Result<(), codelet_test_helpers::TestError> {
    let ctx = app
        .agent_view_store_mut()
        .session_context_mut_for(id)
        .or_fail("SessionContext present for seeded id")?;
    for i in 0..count {
        ctx.scrollback.push(RenderedChunk {
            row_ends: Vec::new(),
            seq: i as u64,
            lines: vec![Line::from(format!("seed-{i}"))],
            source: None,
        });
    }
    Ok(())
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

// ─────────────────────────────────────────────────────────────────────────
// Scenario (RPC-074): /clear resets scrollback but does NOT append any
// `[notice] /clear: history cleared` line. TS handleClearCommand only
// blanks the input + calls backend.clear_history; no scrollback notice
// line is ever pushed by the dispatcher.
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clear_appends_no_notice_line_on_ok() -> TestResult {
    // @step Given an App with an open session s-1 with 3 scrollback chunks wired to a MockBackend whose clear_history returns Ok(())
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    seed_chunks(&mut app, &sid("s-1"), 3)?;

    // @step When SlashCommandSelected(SlashCommandAction::Clear) is dispatched
    app.dispatch(Action::SlashCommandSelected(
        app.current_session(),
        SlashCommandAction::Clear,
    ));

    // @step Then s-1's scrollback chunk_count becomes 0 synchronously
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 0);

    drain_pending(&mut app).await?;

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
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario (RPC-074): /clear with backend Err does NOT append any
// `[error] /clear failed: <reason>` line — TS routes errors to
// logger.error only, Rust routes to `tracing::error!` only.
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clear_appends_no_error_line_on_err() -> TestResult {
    // @step Given an App with an open session s-1 wired to a MockBackend whose clear_history returns Err("boom")
    let mock = Arc::new(MockBackend::new());
    mock.set_clear_history_error("boom".to_string());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));

    // @step When SlashCommandSelected(SlashCommandAction::Clear) is dispatched
    app.dispatch(Action::SlashCommandSelected(
        app.current_session(),
        SlashCommandAction::Clear,
    ));

    drain_pending(&mut app).await?;

    // @step Then after draining pending tasks and the action bus, s-1's scrollback contains zero lines matching "[error] /clear failed"
    let text = session_scrollback_text(&app, &sid("s-1"));
    assert!(
        text.lines().all(|l| !l.contains("[error] /clear failed")),
        "TS parity (RPC-074): backend Err must NOT push `[error] /clear failed: ...` into scrollback (errors go to tracing::error!), got {text:?}",
    );
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// drain_pending — await every spawned tokio task AND fold any queued
// action_tx messages back into the App. Mirrors the same-named helper
// in `slash_command_wiring_rpc022.rs`.
// ─────────────────────────────────────────────────────────────────────────

async fn drain_pending(app: &mut App) -> Result<(), codelet_test_helpers::TestError> {
    while let Some(handle) = app.next_pending_task() {
        handle.await.completed_or_cancelled()?;
    }
    while let Some(action) = app.try_recv_action() {
        app.dispatch(action);
        while let Some(handle) = app.next_pending_task() {
            handle.await.completed_or_cancelled()?;
        }
    }
    Ok(())
}
