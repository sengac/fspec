//! RPC-061 — per-session supervisor / subordinate state held by
//! `AgentViewStore`.
//!
//! Feature files:
//!   - spec/features/rpc061-supervisor-links.feature
//!   - spec/features/rpc061-source-shape.feature
//!
//! Two per-session slots:
//!
//! * `supervisors_by_session: HashMap<SessionId, Vec<SessionId>>` —
//!   snapshot of `backend.get_supervisors(session_id)` applied by
//!   `Action::SupervisorsLoaded`. Read by the SessionHeader badge
//!   renderer ("[Subordinate of: <short-id>]").
//! * `supervisor_pending_count_by_session: HashMap<SessionId, usize>`
//!   — incremented by `StreamChunk::SupervisorPendingInjection` in
//!   `dispatch_stream_chunks::handle_stream_chunk_state_updates`. Read by
//!   the SessionFooter left-aligned chip ("[N pending from
//!   supervisor]"). Resettable via `set_supervisor_pending_count` for
//!   the case where the subordinate consumes the message and the
//!   agent loop emits a fresh count chunk.
//!
//! The block lives in its own sub-module so the parent `agent_view.rs`
//! continues to satisfy the < 300-LoC source-shape ceiling pinned by
//! `source_shape_stores_rpc012`, `source_shape_rpc024`, and
//! `source_shape_rpc025`.

use codelet_rpc_types::SessionId;

use super::AgentViewStore;

impl AgentViewStore {
    /// Borrow the per-session supervisor list for `session`. Returns
    /// an empty slice when no supervisors have been recorded.
    pub fn supervisors_for(&self, session: &SessionId) -> &[SessionId] {
        self.supervisors_by_session
            .get(session)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Replace the per-session supervisor list for `session` with the
    /// fresh snapshot from a `backend.get_supervisors` round-trip.
    pub fn set_supervisors(&mut self, session: SessionId, supervisors: Vec<SessionId>) {
        self.supervisors_by_session.insert(session, supervisors);
    }

    /// Return the current pending-supervisor-injection count for
    /// `session`. Zero when no chunk has bumped the counter yet.
    pub fn supervisor_pending_count_for(&self, session: &SessionId) -> usize {
        self.supervisor_pending_count_by_session
            .get(session)
            .copied()
            .unwrap_or(0)
    }

    /// Replace the per-session pending count for `session`. Used by
    /// the agent loop / TS-parity reset path that ships an explicit
    /// count chunk.
    pub fn set_supervisor_pending_count(&mut self, session: SessionId, count: usize) {
        if count == 0 {
            self.supervisor_pending_count_by_session.remove(&session);
        } else {
            self.supervisor_pending_count_by_session
                .insert(session, count);
        }
    }

    /// RPC-061: bump the pending count for `session` by 1. Invoked by
    /// `dispatch_stream_chunks::handle_stream_chunk_state_updates` on a
    /// `StreamChunk::SupervisorPendingInjection`.
    pub fn apply_supervisor_pending_injection(&mut self, session: &SessionId) {
        let entry = self
            .supervisor_pending_count_by_session
            .entry(session.clone())
            .or_insert(0);
        *entry = entry.saturating_add(1);
    }
}
