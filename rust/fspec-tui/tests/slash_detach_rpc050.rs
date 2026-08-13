//! RPC-050 — `/detach` slash command + work-unit context teardown.
//!
//! Feature: spec/features/slash-command-detach-and-work-unit-binding.feature
//!
//! Drives the App::dispatch routing for `SlashCommandSelected(Detach)`
//! through the three documented paths:
//!   1. Active session AND bound work unit: spawns
//!      `backend.set_work_unit_context(session, None)`, on Ok dispatches
//!      `Action::WorkUnitDetached(session)` which clears the binding,
//!      resets scrollback (TS prepareForNewSession parity) and resets
//!      the per-session TokenState.
//!   2. No active session: silent no-op (matches `/clear` no-session
//!      behaviour).
//!   3. Active session without a bound work unit: emits
//!      `[notice] /detach: no work unit attached` and skips the backend.
//!   4. Backend error: emits `[error] /detach failed: {reason}` and
//!      preserves the local binding so the user can retry.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::views::agent::slash_commands::SlashCommandAction;
use codelet_fspec_tui::{Action, App, FspecBackend, RenderedChunk};
use codelet_rpc_types::{ContextFillInfo, SessionId, StreamChunk, TokenTracker, WorkUnitContext};
use ratatui::text::Line;
use tokio::time::timeout;

mod common;
use common::MockBackend;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

fn ctx(id: &str, status: &str) -> WorkUnitContext {
    WorkUnitContext {
        id: id.to_string(),
        title: id.to_string(),
        status: status.to_string(),
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

fn session_chunk_count(app: &App, id: &SessionId) -> usize {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.chunk_count())
        .unwrap_or(0)
}

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

fn fresh_app_with_session(work_unit: Option<&str>) -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.dispatch(Action::SessionCreated(sid("s-1")));
    if let Some(wu) = work_unit {
        // Bind by dispatching the Attached action directly (bypasses the
        // BoardView attach path; the chrome path tests the bound state
        // assertion in isolation from the attach round-trip).
        app.dispatch(Action::WorkUnitAttached(
            sid("s-1"),
            ctx(wu, "implementing"),
        ));
    }
    (app, mock)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /detach with a bound work unit clears the binding, resets
// scrollback and TokenState
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detach_with_bound_work_unit_clears_binding_scrollback_and_token_state() {
    // @step Given an App with open session s-1 bound to AUTH-001
    let (mut app, mock) = fresh_app_with_session(Some("AUTH-001"));
    assert!(app
        .agent_view_store()
        .work_unit_context_for(&sid("s-1"))
        .is_some());

    // @step And s-1's scrollback has 3 chunks
    seed_chunks(&mut app, &sid("s-1"), 3);
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 3);

    // @step And s-1's TokenState has input_tokens=42 and output_tokens=7
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::TokenUpdate {
            tokens: TokenTracker {
                input_tokens: 42,
                output_tokens: 7,
                cache_read_input_tokens: None,
                cache_creation_input_tokens: None,
                tokens_per_second: None,
                cumulative_billed_input: None,
                cumulative_billed_output: None,
                reasoning_tokens: None,
            },
        },
    ));
    app.dispatch(Action::ChunkReceived(
        sid("s-1"),
        StreamChunk::ContextFillUpdate {
            context_fill: ContextFillInfo {
                fill_percentage: 10,
                effective_tokens: 1000.0,
                threshold: 8000.0,
                context_window: 10000.0,
            },
        },
    ));
    assert_eq!(
        app.agent_view_store()
            .token_state_for(&sid("s-1"))
            .copied()
            .unwrap_or_default()
            .input_tokens,
        42
    );

    // @step And the MockBackend's set_work_unit_context returns Ok(())
    // (default for MockBackend — no scripting needed)

    // @step When SlashCommandSelected(SlashCommandAction::Detach) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Detach));
    drain_pending(&mut app).await;

    // @step Then within 1 second backend.set_work_unit_context is called exactly once with (s-1, None)
    wait_until(
        || mock.set_work_unit_context_calls() == 1,
        "set_work_unit_context call count to reach 1",
    )
    .await;
    let last = mock.last_set_work_unit_context().expect("last set call");
    assert_eq!(last.0, sid("s-1"));
    assert_eq!(last.1, None);

    // @step And within 1 second AgentViewStore.work_unit_context_for(s-1) returns None
    wait_until(
        || {
            app.agent_view_store()
                .work_unit_context_for(&sid("s-1"))
                .is_none()
        },
        "work_unit_context_for(s-1) to clear",
    )
    .await;

    // @step And within 1 second s-1's scrollback chunk_count becomes 0
    assert_eq!(
        session_chunk_count(&app, &sid("s-1")),
        0,
        "s-1's scrollback must be reset by /detach",
    );

    // @step And within 1 second s-1's TokenState equals TokenState::default()
    let tokens = app
        .agent_view_store()
        .token_state_for(&sid("s-1"))
        .copied()
        .unwrap_or_default();
    assert_eq!(tokens.input_tokens, 0);
    assert_eq!(tokens.output_tokens, 0);
    assert_eq!(tokens.context_fill_pct, 0);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /detach with no active session is a silent no-op
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detach_with_no_active_session_is_silent_no_op() {
    // @step Given an App with NO open session
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    assert!(app.agent_view_store().current_session().is_none());

    // @step When SlashCommandSelected(SlashCommandAction::Detach) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Detach));
    drain_pending(&mut app).await;

    // @step Then backend.set_work_unit_context is NEVER called
    assert_eq!(
        mock.set_work_unit_context_calls(),
        0,
        "set_work_unit_context must not run without a session",
    );

    // @step And no scrollback chunk is appended to any session
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        0,
        "open_sessions must remain empty",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /detach with a session but no work unit attached emits a
// notice
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detach_with_no_bound_work_unit_emits_notice() {
    // @step Given an App with open session s-1 NOT bound to any work unit
    let (mut app, mock) = fresh_app_with_session(None);
    assert!(app
        .agent_view_store()
        .work_unit_context_for(&sid("s-1"))
        .is_none());

    // @step When SlashCommandSelected(SlashCommandAction::Detach) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Detach));
    drain_pending(&mut app).await;

    // @step Then within 1 second s-1's scrollback contains a chunk whose text equals "[notice] /detach: no work unit attached"
    let text = session_scrollback_text(&app, &sid("s-1"));
    assert!(
        text.lines()
            .any(|l| l == "[notice] /detach: no work unit attached"),
        "expected no-work-unit notice in s-1 scrollback, got {text:?}",
    );

    // @step And backend.set_work_unit_context is NEVER called
    assert_eq!(
        mock.set_work_unit_context_calls(),
        0,
        "set_work_unit_context must not run without a bound work unit",
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: /detach failure surfaces an error notice and preserves
// local state
// ─────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detach_failure_surfaces_error_notice_and_preserves_local_state() {
    // @step Given an App with open session s-1 bound to AUTH-001
    let (mut app, mock) = fresh_app_with_session(Some("AUTH-001"));

    // @step And s-1's scrollback has 3 chunks
    seed_chunks(&mut app, &sid("s-1"), 3);
    assert_eq!(session_chunk_count(&app, &sid("s-1")), 3);

    // @step And the MockBackend's set_work_unit_context returns Err("corrupt manifest")
    mock.set_work_unit_context_error("corrupt manifest".to_string());

    // @step When SlashCommandSelected(SlashCommandAction::Detach) is dispatched
    app.dispatch(Action::SlashCommandSelected(SlashCommandAction::Detach));
    drain_pending(&mut app).await;

    // @step Then within 1 second s-1's scrollback contains a chunk whose text equals "[error] /detach failed: corrupt manifest"
    let text = session_scrollback_text(&app, &sid("s-1"));
    assert!(
        text.lines()
            .any(|l| l == "[error] /detach failed: corrupt manifest"),
        "expected /detach error notice in s-1 scrollback, got {text:?}",
    );

    // @step And AgentViewStore.work_unit_context_for(s-1) still returns Some(ctx) with id "AUTH-001"
    let stored = app
        .agent_view_store()
        .work_unit_context_for(&sid("s-1"))
        .expect("work-unit binding must be preserved on Err");
    assert_eq!(stored.id, "AUTH-001");
}
