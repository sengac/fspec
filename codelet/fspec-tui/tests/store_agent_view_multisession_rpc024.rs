//! RPC-024 — Unit tests for the multi-session `AgentViewStore` refactor.
//!
//! Feature: spec/features/rpc024-multi-session-store.feature
//!
//! These tests pin the new public surface of `AgentViewStore`:
//!   * `open_sessions: Vec<SessionContext>` + `current_session_index: usize`
//!     replace the single-slot `current_session: Option<SessionId>`.
//!   * `append_session(SessionContext)` is the sole producer.
//!   * `focus_session_index(idx)` repositions focus (RPC-096, replaces
//!     the RPC-024 `cycle_session(±1)` wrap-around which was removed
//!     in favour of TS-parity end-of-list semantics — see
//!     `agentview-shift-arrow-end-of-list-parity.feature`).
//!   * `current_session_context[_mut]()` returns the focused SessionContext.
//!   * `set_input_draft(idx, value)` persists the outgoing MultiLineInput
//!     buffer before a switch.
//!   * `session_index()` is now a derived `(idx+1, len)` tuple — the
//!     `set_session_index` setter from RPC-018 is REMOVED.
//!
//! No `App` is used here — these are pure store-level tests. The
//! App-layer wiring tests live in `app_dispatch_rpc024.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::AgentViewStore;
use codelet_fspec_tui::store::SessionContext;
use codelet_rpc_types::SessionId;

fn sid(s: &str) -> SessionId {
    SessionId::new(s)
}

/// Scenario: An empty AgentViewStore reports zero open sessions
#[test]
fn empty_store_reports_zero_sessions_and_noop_focus() {
    // @step Given a default AgentViewStore
    let mut store = AgentViewStore::default();
    // @step Then open_sessions is empty
    assert!(store.open_sessions().is_empty());
    // @step And session_index() returns (0, 0)
    assert_eq!(store.session_index(), (0, 0));
    // @step And current_session() returns None
    assert!(store.current_session().is_none());
    // @step When focus_session_index(0) is called on the empty store
    store.focus_session_index(0);
    // @step Then current_session_index is still 0
    assert_eq!(store.current_session_index(), 0);
    // @step And open_sessions is still empty
    assert!(store.open_sessions().is_empty());
}

/// Scenario: append_session appends a fresh SessionContext and focuses it
#[test]
fn append_session_appends_and_focuses_first_session() {
    // @step Given a default AgentViewStore with no open sessions
    let mut store = AgentViewStore::default();
    // @step When append_session is called for "s-1"
    store.append_session(SessionContext::new(sid("s-1")));
    // @step Then open_sessions has length 1
    assert_eq!(store.open_sessions().len(), 1);
    // @step And open_sessions[0].id equals "s-1"
    assert_eq!(store.open_sessions()[0].id, sid("s-1"));
    // @step And current_session_index is 0
    assert_eq!(store.current_session_index(), 0);
    // @step And session_index() returns (1, 1)
    assert_eq!(store.session_index(), (1, 1));
    // @step And current_session() returns Some("s-1")
    assert_eq!(store.current_session(), Some(&sid("s-1")));
}

/// Scenario: Successive append_session calls accumulate in creation order
#[test]
fn append_session_accumulates_in_creation_order() {
    // @step Given a default AgentViewStore with no open sessions
    let mut store = AgentViewStore::default();
    // @step When append_session is called for "s-1"
    store.append_session(SessionContext::new(sid("s-1")));
    // @step And append_session is called for "s-2"
    store.append_session(SessionContext::new(sid("s-2")));
    // @step And append_session is called for "s-3"
    store.append_session(SessionContext::new(sid("s-3")));
    // @step Then open_sessions has length 3
    assert_eq!(store.open_sessions().len(), 3);
    // @step And open_sessions ids in order are "s-1", "s-2", "s-3"
    assert_eq!(store.open_sessions()[0].id, sid("s-1"));
    assert_eq!(store.open_sessions()[1].id, sid("s-2"));
    assert_eq!(store.open_sessions()[2].id, sid("s-3"));
    // @step And current_session_index is 2
    assert_eq!(store.current_session_index(), 2);
    // @step And session_index() returns (3, 3)
    assert_eq!(store.session_index(), (3, 3));
    // @step And current_session() returns Some("s-3")
    assert_eq!(store.current_session(), Some(&sid("s-3")));
}

/// Scenario: focus_session_index(idx) decrements current_session_index when idx < current
#[test]
fn focus_session_index_decrements_focus() {
    // @step Given an AgentViewStore with open_sessions ["s-1", "s-2", "s-3"]
    let mut store = AgentViewStore::default();
    store.append_session(SessionContext::new(sid("s-1")));
    store.append_session(SessionContext::new(sid("s-2")));
    store.append_session(SessionContext::new(sid("s-3")));
    // @step And current_session_index is 2
    assert_eq!(store.current_session_index(), 2);
    // @step When focus_session_index(1) is called
    store.focus_session_index(1);
    // @step Then current_session_index is 1
    assert_eq!(store.current_session_index(), 1);
    // @step And session_index() returns (2, 3)
    assert_eq!(store.session_index(), (2, 3));
    // @step And current_session() returns Some("s-2")
    assert_eq!(store.current_session(), Some(&sid("s-2")));
}

/// Scenario: focus_session_index out-of-range is a no-op (RPC-096 removed wrap-around)
#[test]
fn focus_session_index_out_of_range_is_noop() {
    // @step Given an AgentViewStore with open_sessions ["s-1", "s-2", "s-3"]
    let mut store = AgentViewStore::default();
    store.append_session(SessionContext::new(sid("s-1")));
    store.append_session(SessionContext::new(sid("s-2")));
    store.append_session(SessionContext::new(sid("s-3")));
    store.focus_session_index(0);
    assert_eq!(store.current_session_index(), 0);
    // @step When focus_session_index(99) is called
    store.focus_session_index(99);
    // @step Then current_session_index is unchanged at 0
    assert_eq!(store.current_session_index(), 0);
    // @step And current_session() still returns Some("s-1")
    assert_eq!(store.current_session(), Some(&sid("s-1")));
}

/// Scenario: focus_session_index on a single-session store is a self-loop
#[test]
fn focus_session_index_single_session_self_loop() {
    // @step Given an AgentViewStore with open_sessions ["s-1"]
    let mut store = AgentViewStore::default();
    store.append_session(SessionContext::new(sid("s-1")));
    // @step And current_session_index is 0
    assert_eq!(store.current_session_index(), 0);
    // @step When focus_session_index(0) is called
    store.focus_session_index(0);
    // @step Then current_session_index is 0
    assert_eq!(store.current_session_index(), 0);
    // @step And current_session() returns Some("s-1")
    assert_eq!(store.current_session(), Some(&sid("s-1")));
}

/// Scenario: set_input_draft writes into the indexed SessionContext
#[test]
fn set_input_draft_round_trip_via_focus() {
    // @step Given an AgentViewStore with open_sessions ["s-1", "s-2"]
    let mut store = AgentViewStore::default();
    store.append_session(SessionContext::new(sid("s-1")));
    store.append_session(SessionContext::new(sid("s-2")));
    // @step And current_session_index is 1
    assert_eq!(store.current_session_index(), 1);
    // @step When set_input_draft(1, "hello world") is called
    store.set_input_draft(1, "hello world".to_string());
    // @step Then open_sessions[1].input_draft equals "hello world"
    assert_eq!(store.open_sessions()[1].input_draft, "hello world");
    // @step When focus_session_index(0) is called
    store.focus_session_index(0);
    // @step Then the incoming draft is the saved (empty) draft on s-1
    assert_eq!(store.open_sessions()[0].input_draft, "");
    // @step When focus_session_index(1) is called
    store.focus_session_index(1);
    // @step Then the outgoing draft was preserved on s-2
    assert_eq!(store.open_sessions()[1].input_draft, "hello world");
}

/// Scenario: SessionContext owns its own scrollback
#[test]
fn each_session_owns_its_own_scrollback() {
    // @step Given an AgentViewStore with open_sessions ["s-1", "s-2"]
    let mut store = AgentViewStore::default();
    store.append_session(SessionContext::new(sid("s-1")));
    store.append_session(SessionContext::new(sid("s-2")));
    use codelet_rpc_types::StreamChunk;
    // @step When three user-input chunks are recorded on s-1's SessionContext
    // RPC-091: Text deltas accumulate into one chunk — use UserInput
    // (non-accumulating) to assert per-chunk count invariants.
    {
        let ctx = store.session_context_mut_for(&sid("s-1")).expect("s-1");
        ctx.record_chunk(&StreamChunk::user_input("a".to_string()));
        ctx.record_chunk(&StreamChunk::user_input("b".to_string()));
        ctx.record_chunk(&StreamChunk::user_input("c".to_string()));
    }
    // @step And five user-input chunks are recorded on s-2's SessionContext
    {
        let ctx = store.session_context_mut_for(&sid("s-2")).expect("s-2");
        ctx.record_chunk(&StreamChunk::user_input("1".to_string()));
        ctx.record_chunk(&StreamChunk::user_input("2".to_string()));
        ctx.record_chunk(&StreamChunk::user_input("3".to_string()));
        ctx.record_chunk(&StreamChunk::user_input("4".to_string()));
        ctx.record_chunk(&StreamChunk::user_input("5".to_string()));
    }
    // @step Then s-1 still has 3 scrollback chunks
    assert_eq!(store.open_sessions()[0].scrollback.chunk_count(), 3);
    // @step And s-2 still has 5 scrollback chunks
    assert_eq!(store.open_sessions()[1].scrollback.chunk_count(), 5);
}

/// Scenario: session_context_mut_for routes chunks by id, not focus
#[test]
fn session_context_mut_for_routes_chunks_by_id() {
    // @step Given an AgentViewStore with open_sessions ["s-1", "s-2"]
    let mut store = AgentViewStore::default();
    store.append_session(SessionContext::new(sid("s-1")));
    store.append_session(SessionContext::new(sid("s-2")));
    store.focus_session_index(0); // focus s-1
    // @step And current_session_index is 0
    assert_eq!(store.current_session_index(), 0);
    // @step And open_sessions[1].scrollback contains 0 chunks
    assert_eq!(store.open_sessions()[1].scrollback.chunk_count(), 0);
    use codelet_rpc_types::StreamChunk;
    // @step When session_context_mut_for(SessionId("s-2")) records a "background" chunk
    let ctx = store
        .session_context_mut_for(&sid("s-2"))
        .expect("s-2 context exists");
    ctx.record_chunk(&StreamChunk::text("background".to_string()));
    // @step Then open_sessions[1].scrollback contains 1 chunk
    assert_eq!(store.open_sessions()[1].scrollback.chunk_count(), 1);
    // @step And current_session_index is still 0
    assert_eq!(store.current_session_index(), 0);
}

/// Scenario: session_context_mut_for returns None for unknown ids
#[test]
fn session_context_mut_for_returns_none_for_unknown_id() {
    // @step Given an AgentViewStore with open_sessions ["s-1"]
    let mut store = AgentViewStore::default();
    store.append_session(SessionContext::new(sid("s-1")));
    // @step When session_context_mut_for(SessionId("s-ghost")) is called
    let lookup = store.session_context_mut_for(&sid("s-ghost"));
    // @step Then the lookup returns None (caller drops the chunk)
    assert!(lookup.is_none());
    // @step And open_sessions[0].scrollback is unchanged
    assert_eq!(store.open_sessions()[0].scrollback.chunk_count(), 0);
}

/// Scenario: session_index() is derived from current_session_index + 1 and open_sessions.len()
#[test]
fn session_index_is_derived_from_current_index_plus_one_and_len() {
    // @step Given the AgentViewStore type
    let mut store = AgentViewStore::default();
    // @step Then session_index() returns (0, 0) for an empty store
    assert_eq!(store.session_index(), (0, 0));
    // @step When sessions are appended
    store.append_session(SessionContext::new(sid("s-1")));
    store.append_session(SessionContext::new(sid("s-2")));
    // @step Then session_index() returns (current_session_index + 1, len)
    assert_eq!(
        store.session_index(),
        (store.current_session_index() + 1, store.open_sessions().len())
    );
}
