//! RPC-024 — App::dispatch routing tests for multi-session cycling.
//!
//! Feature: spec/features/rpc024-multi-session-cycling.feature
//!
//! These tests verify that:
//!   * `Action::SessionCreated` appends a new SessionContext to
//!     AgentViewStore.open_sessions (rather than overwriting a single
//!     current_session slot).
//!   * `Action::SessionPrev` / `Action::SessionNext` rotate the focus
//!     between open sessions (wrap-around) and round-trip the
//!     MultiLineInput buffer through the per-session input_draft slot.
//!   * `Action::ChunkReceived(id, chunk)` routes the scrollback append
//!     into the SessionContext whose id matches `id`, NOT the currently
//!     focused one — background sessions accumulate scrollback while
//!     the user is on a different session.
//!   * The AgentView render paints the current session's scrollback
//!     (not the orphaned "AgentView.scrollback" field that RPC-024
//!     deletes).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend};
use codelet_rpc_types::{SessionId, StreamChunk};

mod common;
use common::MockBackend;

fn fresh_app() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new());
    let backend: Arc<dyn FspecBackend> = mock.clone();
    let app = App::new(backend);
    (app, mock)
}

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Scenario: Action::SessionCreated appends a fresh SessionContext and focuses it
#[test]
fn session_created_appends_and_focuses() {
    // @step Given a default AgentViewStore with no open sessions
    let (mut app, _mock) = fresh_app();
    assert!(app.agent_view_store().open_sessions().is_empty());
    // @step When App::dispatch handles Action::SessionCreated for "s-1"
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // @step Then open_sessions has length 1
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
    // @step And open_sessions[0].id equals "s-1"
    assert_eq!(app.agent_view_store().open_sessions()[0].id, sid("s-1"));
    // @step And current_session_index is 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    // @step And session_index() returns (1, 1)
    assert_eq!(app.agent_view_store().session_index(), (1, 1));
    // @step And current_session() returns Some("s-1")
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-1")));
}

/// Scenario: Successive SessionCreated dispatches accumulate in creation order
#[test]
fn three_session_created_dispatches_accumulate() {
    // @step Given a default AgentViewStore with no open sessions
    let (mut app, _mock) = fresh_app();
    // @step When App::dispatch handles Action::SessionCreated for "s-1"
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // @step And App::dispatch handles Action::SessionCreated for "s-2"
    app.dispatch(Action::SessionCreated(sid("s-2")));
    // @step And App::dispatch handles Action::SessionCreated for "s-3"
    app.dispatch(Action::SessionCreated(sid("s-3")));
    // @step Then open_sessions has length 3
    assert_eq!(app.agent_view_store().open_sessions().len(), 3);
    // @step And open_sessions ids in order are "s-1", "s-2", "s-3"
    assert_eq!(app.agent_view_store().open_sessions()[0].id, sid("s-1"));
    assert_eq!(app.agent_view_store().open_sessions()[1].id, sid("s-2"));
    assert_eq!(app.agent_view_store().open_sessions()[2].id, sid("s-3"));
    // @step And current_session_index is 2
    assert_eq!(app.agent_view_store().current_session_index(), 2);
    // @step And session_index() returns (3, 3)
    assert_eq!(app.agent_view_store().session_index(), (3, 3));
    // @step And current_session() returns Some("s-3")
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-3")));
}

/// Scenario: Action::SessionPrev decrements current_session_index without wrap when not at zero
#[test]
fn session_prev_decrements_without_wrap() {
    // @step Given an AgentViewStore with open_sessions ["s-1", "s-2", "s-3"]
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    app.dispatch(Action::SessionCreated(sid("s-3")));
    // @step And current_session_index is 2
    assert_eq!(app.agent_view_store().current_session_index(), 2);
    // @step When App::dispatch handles Action::SessionPrev
    app.dispatch(Action::SessionPrev);
    // @step Then current_session_index is 1
    assert_eq!(app.agent_view_store().current_session_index(), 1);
    // @step And session_index() returns (2, 3)
    assert_eq!(app.agent_view_store().session_index(), (2, 3));
    // @step And current_session() returns Some("s-2")
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-2")));
}

/// Scenario: Action::SessionPrev wraps around from index 0 to len-1; SessionNext wraps back
#[test]
fn session_prev_wraps_zero_to_last_then_next_wraps_back() {
    // @step Given an AgentViewStore with open_sessions ["s-1", "s-2", "s-3"]
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    app.dispatch(Action::SessionCreated(sid("s-3")));
    // @step And current_session_index is 0
    app.dispatch(Action::SessionPrev); // 2 -> 1
    app.dispatch(Action::SessionPrev); // 1 -> 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    // @step When App::dispatch handles Action::SessionPrev
    app.dispatch(Action::SessionPrev);
    // @step Then current_session_index is 2
    assert_eq!(app.agent_view_store().current_session_index(), 2);
    // @step And current_session() returns Some("s-3")
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-3")));
    // @step When App::dispatch handles Action::SessionNext
    app.dispatch(Action::SessionNext);
    // @step Then current_session_index is 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    // @step And current_session() returns Some("s-1")
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-1")));
}

/// Scenario: SessionPrev and SessionNext are self-loops when only one session is open
#[test]
fn session_cycle_is_self_loop_for_single_session() {
    // @step Given an AgentViewStore with open_sessions ["s-1"]
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // @step And current_session_index is 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    // @step When App::dispatch handles Action::SessionPrev
    app.dispatch(Action::SessionPrev);
    // @step Then current_session_index is 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    // @step When App::dispatch handles Action::SessionNext
    app.dispatch(Action::SessionNext);
    // @step Then current_session_index is 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    // @step And current_session() returns Some("s-1")
    assert_eq!(app.agent_view_store().current_session(), Some(&sid("s-1")));
}

/// Scenario: Switching sessions saves the outgoing draft and restores the incoming draft
#[test]
fn switching_sessions_round_trips_input_draft() {
    // @step Given an AgentViewStore with open_sessions ["s-1", "s-2"]
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    // @step And current_session_index is 1
    assert_eq!(app.agent_view_store().current_session_index(), 1);
    // @step And the MultiLineInput buffer is "hello world"
    app.navigator_mut().agent.input.set_value("hello world");
    // @step When App::dispatch handles Action::SessionPrev
    app.dispatch(Action::SessionPrev);
    // @step Then open_sessions[1].input_draft equals "hello world"
    assert_eq!(
        app.agent_view_store().open_sessions()[1].input_draft,
        "hello world"
    );
    // @step And the MultiLineInput buffer equals open_sessions[0].input_draft
    assert_eq!(
        app.navigator().agent.input.value(),
        app.agent_view_store().open_sessions()[0].input_draft
    );
    // @step When App::dispatch handles Action::SessionNext
    app.dispatch(Action::SessionNext);
    // @step Then the MultiLineInput buffer is "hello world"
    assert_eq!(app.navigator().agent.input.value(), "hello world");
}

/// Scenario: Each session retains its own scrollback across cycling
#[test]
fn each_session_keeps_its_own_scrollback_across_cycling() {
    // @step Given an AgentViewStore with open_sessions ["s-1", "s-2"]
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    // @step And open_sessions[0].scrollback contains 3 chunks
    for _ in 0..3 {
        app.dispatch(Action::ChunkReceived(
            sid("s-1"),
            StreamChunk::text("from-s1".to_string()),
        ));
    }
    // @step And open_sessions[1].scrollback contains 5 chunks
    for _ in 0..5 {
        app.dispatch(Action::ChunkReceived(
            sid("s-2"),
            StreamChunk::text("from-s2".to_string()),
        ));
    }
    // @step And current_session_index is 1
    assert_eq!(app.agent_view_store().current_session_index(), 1);
    // @step When App::dispatch handles Action::SessionPrev
    app.dispatch(Action::SessionPrev);
    // @step Then the AgentView render paints "s-1"'s 3 chunks
    assert_eq!(app.navigator().agent.chunk_count(app.agent_view_store()), 3);
    // @step When App::dispatch handles Action::SessionNext
    app.dispatch(Action::SessionNext);
    // @step Then the AgentView render paints "s-2"'s 5 chunks
    assert_eq!(app.navigator().agent.chunk_count(app.agent_view_store()), 5);
    // @step And no chunks have been dropped from either session's scrollback
    assert_eq!(
        app.agent_view_store().open_sessions()[0].scrollback.chunk_count(),
        3
    );
    assert_eq!(
        app.agent_view_store().open_sessions()[1].scrollback.chunk_count(),
        5
    );
}

/// Scenario: A StreamChunk for a non-current session lands in that session's scrollback
#[test]
fn background_chunk_routes_to_its_own_session_context() {
    // @step Given an AgentViewStore with open_sessions ["s-1", "s-2"]
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    app.dispatch(Action::SessionCreated(sid("s-2")));
    app.dispatch(Action::SessionPrev); // focus s-1
    // @step And current_session_index is 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    // @step And open_sessions[1].scrollback contains 0 chunks
    assert_eq!(
        app.agent_view_store().open_sessions()[1].scrollback.chunk_count(),
        0
    );
    // @step When App::dispatch handles Action::ChunkReceived("s-2", StreamChunk::text("background"))
    app.dispatch(Action::ChunkReceived(
        sid("s-2"),
        StreamChunk::text("background".to_string()),
    ));
    // @step Then open_sessions[1].scrollback contains 1 chunk
    assert_eq!(
        app.agent_view_store().open_sessions()[1].scrollback.chunk_count(),
        1
    );
    // @step And current_session_index is still 0
    assert_eq!(app.agent_view_store().current_session_index(), 0);
    // @step And the AgentView render still paints "s-1"'s scrollback (0 chunks)
    assert_eq!(app.navigator().agent.chunk_count(app.agent_view_store()), 0);
    // @step When App::dispatch handles Action::SessionNext
    app.dispatch(Action::SessionNext);
    // @step Then the AgentView render now includes the "background" chunk
    assert_eq!(app.navigator().agent.chunk_count(app.agent_view_store()), 1);
}

/// Scenario: Action::ChunkReceived for an unknown session id is dropped silently
#[test]
fn unknown_session_chunk_is_dropped_silently() {
    // @step Given an AgentViewStore with open_sessions ["s-1"]
    let (mut app, _mock) = fresh_app();
    app.dispatch(Action::SessionCreated(sid("s-1")));
    // @step When App::dispatch handles Action::ChunkReceived("s-ghost", StreamChunk::text("orphan"))
    app.dispatch(Action::ChunkReceived(
        sid("s-ghost"),
        StreamChunk::text("orphan".to_string()),
    ));
    // @step Then App::dispatch does not panic (we got here)
    // @step And open_sessions[0].scrollback is unchanged
    assert_eq!(
        app.agent_view_store().open_sessions()[0].scrollback.chunk_count(),
        0
    );
    // @step And no other SessionContext exists for "s-ghost"
    assert_eq!(app.agent_view_store().open_sessions().len(), 1);
}
