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
    /// `((1.0 - compression_ratio) * 100.0).round() as i32`) so the
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
    pub fn clear_compaction_reduction(&mut self, session_id: &SessionId) {
        self.compaction_reduction_by_session.remove(session_id);
    }
}
