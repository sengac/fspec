//! RPC-018 — per-session model / token / thinking chrome accessors for
//! `AgentViewStore`.
//!
//! Feature files: spec/features/rpc018-agent-chrome.feature,
//! spec/features/rpc018-app-bootstrap.feature
//!
//! Storage for the chrome state lives as fields on `AgentViewStore`
//! (declared in the parent module). This sub-module hosts the public
//! accessors so `agent_view.rs` honours the 300-LoC ceiling pinned by
//! RPC-024's `session_context_module_exists_under_300_loc` source-shape
//! test (and RPC-025's `agent_view_store_stays_under_300_loc_with_history_fields`).
//!
//! Cards: RPC-018 (parent RPC-002).

use codelet_rpc_types::{ModelInfo, SessionId, StreamChunk, ThinkingLevel, WorkspaceInfo};

use super::{AgentViewStore, TokenState};

/// CONT-007: live continue/goal counter snapshot for the footer
/// indicator, folded from `StreamChunk::ContinueStateUpdate`. Cleared by
/// the `/continue` and `/goal` dispatches (their state changes never flow
/// through an active stream) — the next TurnStart emission re-syncs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinueLiveState {
    pub nudges_used: u32,
    /// Display budget: `max(explicit, 15)` while a goal is active,
    /// the explicit `/continue` budget otherwise.
    pub effective_budget: u32,
    pub goal_active: bool,
    /// CONT-008: done() rejection count from the live snapshot — drives
    /// the bare `/goal` "rejections: n" display (no more hard-coded 0).
    pub done_rejections: u32,
}

impl AgentViewStore {
    pub fn model_info_for(&self, session_id: &SessionId) -> Option<&ModelInfo> {
        self.model_info_by_session.get(session_id)
    }

    pub fn set_model_info(&mut self, session_id: SessionId, info: ModelInfo) {
        self.model_info_by_session.insert(session_id, info);
    }

    /// RPC-337: remember the model id the user picked for `session_id`
    /// (set on `Action::ModelSelected`). Seeds the ModelSelector's green
    /// `(current)` marker on reopen.
    pub fn set_selected_model_id(&mut self, session_id: SessionId, model_id: String) {
        self.selected_model_id_by_session
            .insert(session_id, model_id);
    }

    /// RPC-337: the last model id selected for `session_id`, if any.
    pub fn selected_model_id_for(&self, session_id: &SessionId) -> Option<&str> {
        self.selected_model_id_by_session
            .get(session_id)
            .map(String::as_str)
    }

    pub fn thinking_level_for(&self, session_id: &SessionId) -> Option<&ThinkingLevel> {
        self.thinking_level_by_session.get(session_id)
    }

    pub fn set_thinking_level(&mut self, session_id: SessionId, level: ThinkingLevel) {
        self.thinking_level_by_session.insert(session_id, level);
    }

    // ── CONT-002 per-session auto-continue accessors ───────────────────

    /// Cached `(enabled, budget)` auto-continue state for the session.
    /// Defaults to `(false, 10)` (off, default budget) when never set.
    pub fn continue_state_for(&self, session_id: &SessionId) -> (bool, u32) {
        self.continue_state_by_session
            .get(session_id)
            .copied()
            .unwrap_or((false, 10))
    }

    /// Cache the `(enabled, budget)` auto-continue state after a
    /// `/continue` apply or a backend load.
    pub fn set_continue_state(&mut self, session_id: SessionId, enabled: bool, budget: u32) {
        self.continue_state_by_session
            .insert(session_id, (enabled, budget));
    }

    // ── CONT-007 per-session live counter accessors ─────────────────────

    /// Live counter snapshot for the session, if a stream has pushed one
    /// since the last `/continue` / `/goal` change.
    pub fn continue_live_for(&self, session_id: &SessionId) -> Option<ContinueLiveState> {
        self.continue_live_by_session.get(session_id).copied()
    }

    /// Fold a `ContinueStateUpdate` chunk's live counter into the cache.
    pub fn set_continue_live(&mut self, session_id: SessionId, live: ContinueLiveState) {
        self.continue_live_by_session.insert(session_id, live);
    }

    /// Drop the (now stale) live counter after a `/continue` or `/goal`
    /// apply — the footer falls back to the cached `(enabled, budget)`
    /// pair with 0 nudges until the next TurnStart emission.
    pub fn clear_continue_live(&mut self, session_id: &SessionId) {
        self.continue_live_by_session.remove(session_id);
    }

    // ── CONT-003 per-session goal accessors ─────────────────────────────

    /// Cached `(text, verify)` goal state for the session. `None` when no
    /// goal is active.
    pub fn goal_state_for(&self, session_id: &SessionId) -> Option<(String, Option<String>)> {
        self.goal_state_by_session.get(session_id).cloned()
    }

    /// Cache (or clear, with `None`) the goal state after a `/goal` apply
    /// or a backend load.
    pub fn set_goal_state(
        &mut self,
        session_id: SessionId,
        goal: Option<(String, Option<String>)>,
    ) {
        match goal {
            Some(state) => {
                self.goal_state_by_session.insert(session_id, state);
            }
            None => {
                self.goal_state_by_session.remove(&session_id);
            }
        }
    }

    pub fn token_state_for(&self, session_id: &SessionId) -> Option<&TokenState> {
        self.token_state_by_session.get(session_id)
    }

    pub fn set_token_state(&mut self, session_id: SessionId, state: TokenState) {
        self.token_state_by_session.insert(session_id, state);
    }

    pub fn apply_chunk_to_token_state(&mut self, session_id: &SessionId, chunk: &StreamChunk) {
        let entry = self
            .token_state_by_session
            .entry(session_id.clone())
            .or_default();
        entry.apply_chunk(chunk);
    }

    pub fn workspace(&self) -> Option<&WorkspaceInfo> {
        self.workspace.as_ref()
    }

    pub fn set_workspace(&mut self, workspace: Option<WorkspaceInfo>) {
        self.workspace = workspace;
    }

    // ── RPC-100 per-session compaction reduction accessors ────────────

    /// Borrow the per-session compaction-reduction percentage, if any.
    /// `None` means the session has not yet completed a compaction
    /// (or was reset via `SessionStateChange { state: Cleared }`).
    pub fn compaction_reduction_for(&self, session_id: &SessionId) -> Option<i32> {
        self.compaction_reduction_by_session
            .get(session_id)
            .copied()
    }

    /// Persist `reduction` (already computed as
    /// `compression_ratio.round() as i32` — the wire value is the
    /// percent of tokens removed [0,100], RPC-420) so the
    /// SessionHeader can render `[X%: COMPACTED {reduction}%]` on the
    /// next frame. Called by `dispatch_stream_chunks.rs` on
    /// `StreamChunk::CompactionComplete`.
    pub fn set_compaction_reduction(&mut self, session_id: SessionId, reduction: i32) {
        self.compaction_reduction_by_session
            .insert(session_id, reduction);
    }

    /// Drop the cached compaction-reduction entry for `session_id`.
    /// Called on `SessionStateChange { state: Cleared }` so the
    /// SessionHeader drops the COMPACTED suffix after a `/clear`.
    ///
    /// RPC-417: ALSO bumps the per-session auto-hide seq so any still
    /// pending 10-second timer becomes a stale no-op — a `/clear` before
    /// 10s must neutralise the queued `Action::ClearCompactionReduction`.
    pub fn clear_compaction_reduction(&mut self, session_id: &SessionId) {
        self.compaction_reduction_by_session.remove(session_id);
        self.bump_compaction_reduction_seq(session_id.clone());
    }

    // ── RPC-417 per-session compaction auto-hide seq accessors ─────────

    /// Current per-session auto-hide seq (default 0 when never armed).
    pub fn compaction_reduction_seq_for(&self, session_id: &SessionId) -> u64 {
        self.compaction_reduction_seq_by_session
            .get(session_id)
            .copied()
            .unwrap_or(0)
    }

    /// Increment `session_id`'s auto-hide seq and return the NEW value.
    /// Called on `CompactionComplete` (arm) and inside
    /// `clear_compaction_reduction` (invalidate pending timers).
    pub fn bump_compaction_reduction_seq(&mut self, session_id: SessionId) -> u64 {
        let entry = self
            .compaction_reduction_seq_by_session
            .entry(session_id)
            .or_insert(0);
        *entry = entry.wrapping_add(1);
        *entry
    }
}
