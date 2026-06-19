//! RPC-045 — per-session isolation / debug / status state held by
//! `AgentViewStore`.
//!
//! Feature file: spec/features/agentview-subscribe-broadcasts.feature
//!
//! This sub-module hosts the [`IsolationState`] struct PLUS the
//! AgentViewStore accessors for the three RPC-045 push-driven slots:
//!
//! - `session_status_by_session: HashMap<SessionId, SessionStatus>` —
//!   updated by [`Action::SessionStatusChanged`] AND by
//!   `StreamChunk::SessionStateChange`. Read by the SessionFooter
//!   status-pill renderer.
//! - `isolation_state_by_session: HashMap<SessionId, IsolationState>` —
//!   updated by `StreamChunk::IsolationStateChange`. Read by the
//!   isolation badge in `SessionHeader`.
//! - `debug_enabled_by_session: HashMap<SessionId, bool>` — updated by
//!   `StreamChunk::DebugStateChange`. Read by the debug badge in
//!   `SessionHeader`.
//!
//! The block lives in its own sub-module so the parent `agent_view.rs`
//! continues to satisfy the
//! `agent_view_store_stays_under_300_loc_with_history_fields`
//! source-shape invariant pinned by `rpc025-source-shape.feature`.

use codelet_rpc_types::{CompactionProgress, SessionId, SessionState, SessionStatus};

use super::AgentViewStore;

/// Per-session isolation snapshot — mirrors the wire shape of
/// `StreamChunk::IsolationStateChange { is_isolated, worktree_path,
/// base_commit }`. The Rust AgentView reads this struct to paint the
/// isolation badge in `SessionHeader` and the `[⎇ <branch>]` indicator
/// when an isolated session targets a worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsolationState {
    pub is_isolated: bool,
    pub worktree_path: Option<String>,
    pub base_commit: Option<String>,
}

/// Map `SessionState` (the wire enum carried by `StreamChunk::SessionStateChange`)
/// onto the broader `SessionStatus` enum used elsewhere in the TUI.
/// Variant order matches exactly between the two enums; the helper
/// stays inline here so callers don't need to import both types.
pub(crate) fn session_status_from_state(state: SessionState) -> SessionStatus {
    match state {
        SessionState::Idle => SessionStatus::Idle,
        SessionState::Running => SessionStatus::Running,
        SessionState::Paused => SessionStatus::Paused,
        SessionState::Compacting => SessionStatus::Compacting,
        SessionState::Interrupted => SessionStatus::Interrupted,
        SessionState::Cleared => SessionStatus::Cleared,
    }
}

impl AgentViewStore {
    // ── Per-session SessionStatus ────────────────────────────────────────

    /// Read the most recently observed [`SessionStatus`] for `session`.
    /// `None` when no transition has been broadcast yet.
    pub fn session_status_for(&self, session: &SessionId) -> Option<&SessionStatus> {
        self.session_status_by_session.get(session)
    }

    /// Persist a [`SessionStatus`] for `session`. Called by both the
    /// `Action::ChunkReceived(_, StreamChunk::SessionStateChange)`
    /// arm AND the `Action::SessionStatusChanged` arm so push-driven
    /// updates from either channel land in the same slot.
    pub fn set_session_status(&mut self, session: SessionId, status: SessionStatus) {
        self.session_status_by_session.insert(session, status);
    }

    // ── Per-session IsolationState ───────────────────────────────────────

    /// Read the most recently observed [`IsolationState`] for `session`.
    pub fn isolation_state_for(&self, session: &SessionId) -> Option<&IsolationState> {
        self.isolation_state_by_session.get(session)
    }

    /// Persist an [`IsolationState`] for `session`. Replaces any
    /// previous entry — the StreamChunk variant is authoritative.
    pub fn set_isolation_state(&mut self, session: SessionId, state: IsolationState) {
        self.isolation_state_by_session.insert(session, state);
    }

    // ── Per-session debug-capture flag ───────────────────────────────────

    /// Read the most recently observed debug-capture flag for `session`.
    /// `None` when no `DebugStateChange` chunk has been seen.
    pub fn debug_enabled_for(&self, session: &SessionId) -> Option<bool> {
        self.debug_enabled_by_session.get(session).copied()
    }

    /// Persist the debug-capture flag for `session`.
    pub fn set_debug_enabled(&mut self, session: SessionId, enabled: bool) {
        self.debug_enabled_by_session.insert(session, enabled);
    }

    // ── Per-session CompactionProgress (RPC-047) ─────────────────────────

    /// Read the most recently observed [`CompactionProgress`] for
    /// `session`. `None` when no compaction is in flight.
    pub fn compaction_progress_for(&self, session: &SessionId) -> Option<&CompactionProgress> {
        self.compaction_progress_by_session.get(session)
    }

    /// Persist a [`CompactionProgress`] snapshot for `session`. Called
    /// by the `/compact` slash handler at request time AND by any future
    /// chunk-driven progress update (e.g. a `CompactionProgressUpdate`
    /// StreamChunk variant — not in scope for RPC-047 but the slot is
    /// already shaped to accept it).
    pub fn set_compaction_progress(&mut self, session: SessionId, progress: CompactionProgress) {
        self.compaction_progress_by_session
            .insert(session, progress);
    }

    /// Drop the per-session compaction progress entry — called when
    /// `StreamChunk::CompactionComplete` arrives so the SessionFooter
    /// stops painting the bar on the next frame.
    pub fn clear_compaction_progress(&mut self, session: &SessionId) {
        self.compaction_progress_by_session.remove(session);
    }
}
