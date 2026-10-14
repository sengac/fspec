//! RPC-437 — TUI consumes the SessionEvent::Cleared lifecycle event.
//!
//! Feature: spec/features/session-events-tui-cleared-consumption.feature
//!
//! This file owns the five TUI-facing scenarios of the feature, driven
//! through the MockBackend (embedded-transport equivalent):
//!
//!   #1 A session cleared from another client resets its local tab live
//!      (no poll)
//!   #2 A clear event for a background session resets only that session
//!   #3 Clear events for a session without an open tab are a no-op
//!   #4 Consecutive clear events for the same live session are
//!      idempotent
//!   #5 Bootstrap spawns the session-cleared subscriber (9th task) and
//!      degenerate defaults leak nothing
//!
//! RED PHASE NOTE: `Action::SessionCleared`, the
//! `FspecBackend::session_cleared_rx` trait surface,
//! `MockBackend::push_session_cleared` and the App-level
//! `handle_session_cleared` handler do NOT exist yet — this file fails
//! to COMPILE until RPC-437's implementation lands. The WS live-wire
//! counterpart (client A clears, client B's transport delivers the
//! frame) is covered at the wire level by
//! rpc-server/tests/rpc436_session_cleared_fan_out.rs; the App-level
//! behaviour is transport-agnostic, so these mock-driven tests pin the
//! TUI half.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::SessionId;

mod common;
use common::MockBackend;

fn sid(value: &str) -> SessionId {
    SessionId::new(value)
}

/// Count the scrollback chunks held by the SessionContext for `id`.
fn session_chunk_count(app: &App, id: &SessionId) -> usize {
    app.agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.chunk_count())
        .unwrap_or(0)
}

/// Count the open tabs whose SessionContext id matches `id`.
fn tabs_for(app: &App, id: &SessionId) -> usize {
    app.agent_view_store()
        .open_sessions()
        .iter()
        .filter(|c| &c.id == id)
        .count()
}

/// Push `count` raw scrollback chunks into the SessionContext for `id`.
fn seed_chunks(app: &mut App, id: &SessionId, count: usize) {
    use codelet_fspec_tui::RenderedChunk;
    use ratatui::text::Line;

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

/// Flatten every visible chunk's text in `id`'s scrollback into a
/// single String.
fn session_scrollback_text(app: &App, id: &SessionId) -> String {
    let chunks = app
        .agent_view_store()
        .session_context_for(id)
        .map(|c| c.scrollback.visible_window(1024))
        .unwrap_or_default();
    chunks
        .iter()
        .flat_map(|chunk| chunk.lines.iter().map(ToString::to_string))
        .collect()
}

/// Drain the App's action bus until a matching action arrives or 2s
/// elapses (mirrors `wait_for_action` in
/// rpc434_session_events_tui_consumption.rs). Drains every queued
/// action each tick so a backlog cannot starve the predicate.
async fn wait_for_action<F: Fn(&Action) -> bool>(app: &mut App, pred: F) -> Option<Action> {
    let deadline = std::time::Instant::now() + Duration::from_millis(2000);
    while std::time::Instant::now() < deadline {
        while let Some(action) = app.try_recv_action() {
            if pred(&action) {
                return Some(action);
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    None
}

// =============================================================================
// Scenario: A session cleared from another client resets its local tab live
//           (no poll)
// =============================================================================

#[tokio::test]
async fn a_session_cleared_from_another_client_resets_its_local_tab_live_no_poll() {
    // @step Given a TUI connected to a daemon over WebSocket with one session S whose tab is focused and has scrollback content
    // "Connected to a daemon" = bootstrap has run — the 9th subscriber
    // (session_cleared_rx) is what folds the mock's cleared broadcast
    // onto the action bus below (the App's bus is transport-agnostic).
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    let s = sid("S-clear");
    app.dispatch(Action::SessionCreated(s.clone()));
    seed_chunks(&mut app, &s, 4);
    app.agent_view_store_mut()
        .session_context_mut_for(&s)
        .expect("ctx")
        .input_draft
        .clear();
    app.navigator_mut()
        .agent
        .input
        .set_value("typed before the remote clear");
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&s),
        "precondition: S is focused"
    );
    assert_eq!(session_chunk_count(&app, &s), 4, "precondition: scrollback seeded");

    // @step When session S is cleared from another machine connected to the same daemon
    // The bootstrap session-cleared subscriber folds the broadcast frame
    // into Action::SessionCleared on the action bus; the App dispatch arm
    // routes it to the idempotent handle_session_cleared handler.
    mock.push_session_cleared(s.clone());
    let action = wait_for_action(
        &mut app,
        |a| matches!(a, Action::SessionCleared(id) if id.value == "S-clear"),
    )
    .await
    .expect("the session-cleared event must reach the action bus");
    app.dispatch(action);

    // @step Then S's local scrollback and input draft are reset without any poll or user action
    assert_eq!(
        session_chunk_count(&app, &s),
        0,
        "the remote clear must reset S's local scrollback"
    );
    assert!(
        app.agent_view_store()
            .session_context_for(&s)
            .map(|c| c.input_draft.is_empty())
            .unwrap_or(true),
        "the remote clear must blank S's input draft"
    );
    assert!(
        app.navigator().agent.input.is_empty(),
        "the remote clear must blank the focused session's input buffer"
    );

    // @step And S's per-session SessionHeader chrome (status badge, token counters, compaction reduction) returns to the cleared state
    assert_eq!(
        app.agent_view_store().session_status_for(&s),
        Some(&codelet_rpc_types::SessionStatus::Cleared),
        "the RPC-100 chrome fold must mark S Cleared"
    );
    assert!(
        app.agent_view_store().compaction_reduction_for(&s).is_none(),
        "the RPC-100 chrome fold must clear S's compaction reduction"
    );
}

// =============================================================================
// Scenario: A clear event for a background session resets only that session
// =============================================================================

#[tokio::test]
async fn a_clear_event_for_a_background_session_resets_only_that_session() {
    // @step Given a TUI with two open sessions S-1 (background) and S-2 (focused), each with scrollback, and S-2's input buffer holding a typed draft
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);
    app.bootstrap().await.expect("bootstrap");
    let s1 = sid("S-1");
    let s2 = sid("S-2");
    app.dispatch(Action::SessionCreated(s1.clone()));
    app.dispatch(Action::SessionCreated(s2.clone()));
    // S-2 was created second → it is focused; S-1 is the background tab.
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&s2),
        "precondition: S-2 is focused, S-1 is background"
    );
    seed_chunks(&mut app, &s1, 3);
    seed_chunks(&mut app, &s2, 3);
    app.agent_view_store_mut()
        .session_context_mut_for(&s1)
        .expect("ctx s-1")
        .input_draft
        .push_str("draft on background session");
    app.navigator_mut().agent.input.set_value("draft on focused session");

    // @step When a SessionCleared event for S-1 is processed
    // Direct dispatch (the handler is a property of the App, transport
    // independent) — the live bus path is pinned by scenario #1.
    app.dispatch(Action::SessionCleared(s1.clone()));

    // @step Then S-1's local scrollback and input draft are reset
    assert_eq!(
        session_chunk_count(&app, &s1),
        0,
        "the remote clear must reset S-1's scrollback"
    );
    assert!(
        app.agent_view_store()
            .session_context_for(&s1)
            .map(|c| c.input_draft.is_empty())
            .unwrap_or(true),
        "the remote clear must blank S-1's input draft"
    );
    assert_eq!(
        app.agent_view_store().session_status_for(&s1),
        Some(&codelet_rpc_types::SessionStatus::Cleared),
        "S-1's status must fold to Cleared (RPC-100 chrome)"
    );

    // @step And S-2's scrollback, input buffer and focus are untouched
    assert_eq!(session_chunk_count(&app, &s2), 3, "S-2's scrollback must survive");
    assert!(
        !app.navigator().agent.input.is_empty(),
        "S-2's (focused) input buffer must survive — only the cleared session is touched"
    );
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&s2),
        "focus must stay on S-2"
    );

    // @step And S-1's per-session token and compaction badge state is reset
    assert!(
        app.agent_view_store().compaction_reduction_for(&s1).is_none(),
        "S-1's compaction reduction must be cleared (RPC-100 chrome)"
    );
}

// =============================================================================
// Scenario: Clear events for a session without an open tab are a no-op
// =============================================================================

#[tokio::test]
async fn clear_events_for_a_session_without_an_open_tab_are_a_no_op() {
    // @step Given a TUI where session S was already closed locally (its tab already removed) or was never opened in this client
    // (The mock is moved into the App: this test dispatches directly and
    // never pushes, so the Arc's keep-alive role is enough.)
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    let closed = sid("S-closed");
    let keep = sid("S-keep");
    app.dispatch(Action::SessionCreated(keep.clone()));
    // Local close path: the tab is already gone when the remote event
    // arrives (the local close path removes synchronously BEFORE the
    // daemon fires the event).
    app.agent_view_store_mut().remove_session_if_open(&closed);
    assert_eq!(tabs_for(&app, &closed), 0, "precondition: tab is closed");

    // @step When the TUI processes a SessionCleared event for S (fired twice to pin the double-fire no-op)
    // Fired TWICE to pin the double-fire no-op (local close path +
    // remote event), mirroring the RPC-434 removal idempotency pin.
    app.dispatch(Action::SessionCleared(closed.clone()));
    app.dispatch(Action::SessionCleared(closed.clone()));

    // @step Then the TUI ignores the events as no-ops — no tab is created, no panic, and the surviving session's focus and state are untouched
    assert_eq!(tabs_for(&app, &closed), 0, "no tab may be created by a stale clear");
    assert_eq!(tabs_for(&app, &keep), 1, "the surviving tab must be untouched");
    assert_eq!(
        app.agent_view_store().current_session(),
        Some(&keep),
        "focus must be unaffected by a clear of a non-open session"
    );
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        1,
        "no tab must be lost or duplicated by the idempotent clear"
    );
}

// =============================================================================
// Scenario: Consecutive clear events for the same live session are
//           idempotent
// =============================================================================

#[tokio::test]
async fn consecutive_clear_events_for_the_same_live_session_are_idempotent() {
    // @step Given a TUI with one open session S whose tab was already reset by a first SessionCleared event
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock;
    let mut app = App::new(backend);
    let s = sid("S-repeat");
    app.dispatch(Action::SessionCreated(s.clone()));
    seed_chunks(&mut app, &s, 3);
    app.dispatch(Action::SessionCleared(s.clone()));
    assert_eq!(
        session_chunk_count(&app, &s),
        0,
        "the first clear must reset the tab"
    );

    // @step When a second SessionCleared event for S arrives (clear is repeatable — the session stays live)
    app.dispatch(Action::SessionCleared(s.clone()));

    // @step Then the TUI holds exactly one tab for S with no duplicate cleanup, no scrollback notice lines, and no state drift
    assert_eq!(
        tabs_for(&app, &s),
        1,
        "exactly one tab must survive the double clear"
    );
    assert_eq!(
        session_chunk_count(&app, &s),
        0,
        "the second clear must be a no-op on the already-empty tab"
    );
    let text = session_scrollback_text(&app, &s);
    assert!(
        text.lines().all(|l| !l.contains("[notice]")),
        "no scrollback notice line may be pushed by the clear fold (RPC-074 TS parity), got {text:?}"
    );
    assert_eq!(
        app.agent_view_store().session_status_for(&s),
        Some(&codelet_rpc_types::SessionStatus::Cleared),
        "the status must stay Cleared after the second clear"
    );
    assert_eq!(
        app.agent_view_store().open_sessions().len(),
        1,
        "no duplicate tabs may be created"
    );
}

// =============================================================================
// Scenario: Bootstrap spawns the session-cleared subscriber and degenerate
//           defaults leak nothing
// =============================================================================

#[tokio::test]
async fn bootstrap_spawns_the_session_cleared_subscriber_with_degenerate_defaults() {
    // @step Given an App bootstrapped against a backend whose session-cleared receiver is the degenerate default (closed)
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let mut app = App::new(backend);

    // @step When the App's bootstrap runs
    app.bootstrap().await.expect("bootstrap");

    // @step Then bootstrap spawns nine subscriber tasks — the existing eight plus the new session_cleared_rx subscriber
    // (work_units + chunks + logs + status_changes + session_created +
    // checkpoints_progress + git_state + session_removed = 8; RPC-437
    // adds the 9th: session_cleared).
    assert_eq!(
        app.subscriber_task_count(),
        9,
        "bootstrap must spawn the new session-cleared subscriber task (8 pre-RPC-437)"
    );

    // @step And the degenerate (closed) session-cleared receiver must not leak SessionCleared events onto the action bus
    tokio::time::sleep(Duration::from_millis(100)).await;
    let stale = wait_for_action(&mut app, |a| matches!(a, Action::SessionCleared(_))).await;
    assert!(
        stale.is_none(),
        "a closed session-cleared receiver must never emit SessionCleared; got {stale:?}"
    );
    assert!(
        app.agent_view_store().open_sessions().is_empty(),
        "a degenerate default must seed no tabs"
    );
    let _ = mock; // keep the backend alive for the subscriber's lifetime
}
