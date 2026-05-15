//! AgentViewStore — single source of truth for the AgentView session
//! navigation state PLUS the per-session model/token/thinking chrome
//! state introduced by RPC-018.
//!
//! Feature files:
//!   - spec/features/rpc012-board-agent-navigation.feature
//!   - spec/features/rpc018-agent-chrome.feature
//!   - spec/features/rpc018-app-bootstrap.feature
//! Cards: RPC-012, RPC-018 (parent RPC-002).
//!
//! Plain owned Rust struct held by [`crate::app::App`]. Mirrors the
//! navigation-relevant slice of `src/tui/store/sessionStore.ts` plus the
//! `useModelStore` + `tokenStateUtils` pieces consumed by the TS
//! AgentView chrome (`SessionHeader.tsx` + `SessionFooter.tsx`).

use std::collections::HashMap;

use codelet_rpc_types::{
    ContextFillInfo, ModelInfo, SessionId, StreamChunk, ThinkingLevel, TokenTracker,
    WorkspaceInfo,
};

/// Per-session token state derived from `StreamChunk::TokenUpdate` +
/// `StreamChunk::ContextFillUpdate` events arriving on
/// `Action::ChunkReceived`. Mirrors `ExtractedTokenState` from
/// `src/tui/utils/tokenStateUtils.ts` so the Rust SessionHeader paints
/// the same `tokens: in↓ out↑ [fill%]` triple as the Ink original.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenState {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub context_fill_pct: u8,
}

impl TokenState {
    /// Fold an arriving chunk into this state. `TokenUpdate` overwrites
    /// `input_tokens` + `output_tokens`; `ContextFillUpdate` overwrites
    /// `context_fill_pct`; every other variant is a no-op.
    pub fn apply_chunk(&mut self, chunk: &StreamChunk) {
        match chunk {
            StreamChunk::TokenUpdate { tokens } => self.apply_token_tracker(tokens),
            StreamChunk::ContextFillUpdate { context_fill } => {
                self.apply_context_fill(context_fill);
            }
            _ => {}
        }
    }

    fn apply_token_tracker(&mut self, t: &TokenTracker) {
        self.input_tokens = t.input_tokens as u64;
        self.output_tokens = t.output_tokens as u64;
    }

    fn apply_context_fill(&mut self, info: &ContextFillInfo) {
        // RPC-018: ContextFillInfo.fill_percentage is u32 from the TS
        // shape — clamp to 100 (percentage upper bound) and narrow to
        // u8 so the SessionHeader can paint `[N%]` without per-render
        // arithmetic.
        self.context_fill_pct = info.fill_percentage.min(100) as u8;
    }
}

/// AgentView session navigation state + per-session chrome state.
/// Mutated only on the App task.
#[derive(Debug, Default)]
pub struct AgentViewStore {
    current_session: Option<SessionId>,
    navigation_target_session: Option<SessionId>,
    current_work_unit_id: Option<String>,
    current_work_unit_status: Option<String>,
    show_create_session_dialog: bool,
    should_auto_create_session: bool,

    // ── RPC-018 chrome state ───────────────────────────────────────────
    /// 1-based session index within the sessions list (`#N`). Defaults
    /// to `(0, 0)` until the App publishes a real index.
    session_index: (usize, usize),
    model_info_by_session: HashMap<SessionId, ModelInfo>,
    token_state_by_session: HashMap<SessionId, TokenState>,
    thinking_level_by_session: HashMap<SessionId, ThinkingLevel>,
    workspace: Option<WorkspaceInfo>,
}

impl AgentViewStore {
    pub fn current_session(&self) -> Option<&SessionId> {
        self.current_session.as_ref()
    }

    pub fn set_current_session(&mut self, session: Option<SessionId>) {
        self.current_session = session;
    }

    pub fn navigation_target_session(&self) -> Option<&SessionId> {
        self.navigation_target_session.as_ref()
    }

    pub fn set_navigation_target(&mut self, target: Option<SessionId>) {
        self.navigation_target_session = target;
    }

    pub fn take_navigation_target(&mut self) -> Option<SessionId> {
        self.navigation_target_session.take()
    }

    pub fn current_work_unit_id(&self) -> Option<&str> {
        self.current_work_unit_id.as_deref()
    }

    pub fn current_work_unit_status(&self) -> Option<&str> {
        self.current_work_unit_status.as_deref()
    }

    pub fn set_current_work_unit(
        &mut self,
        id: Option<String>,
        status: Option<String>,
    ) {
        self.current_work_unit_id = id;
        self.current_work_unit_status = status;
    }

    pub fn show_create_session_dialog(&self) -> bool {
        self.show_create_session_dialog
    }

    pub fn should_auto_create_session(&self) -> bool {
        self.should_auto_create_session
    }

    pub fn request_create_session_dialog(&mut self) {
        self.show_create_session_dialog = true;
        self.should_auto_create_session = true;
    }

    pub fn clear_create_session_dialog(&mut self) {
        self.show_create_session_dialog = false;
        self.should_auto_create_session = false;
    }

    // ── RPC-018 chrome accessors ───────────────────────────────────────

    /// 1-based session index + total count. The SessionHeader paints
    /// `#N:` from `current` when `total >= current >= 1`.
    pub fn session_index(&self) -> (usize, usize) {
        self.session_index
    }

    /// Set the (current, total) session index pair. Idempotent.
    pub fn set_session_index(&mut self, current: usize, total: usize) {
        self.session_index = (current, total);
    }

    /// Borrow the ModelInfo for `session_id`, if one has been recorded.
    pub fn model_info_for(&self, session_id: &SessionId) -> Option<&ModelInfo> {
        self.model_info_by_session.get(session_id)
    }

    /// Record the ModelInfo for `session_id`.
    pub fn set_model_info(&mut self, session_id: SessionId, info: ModelInfo) {
        self.model_info_by_session.insert(session_id, info);
    }

    /// Borrow the ThinkingLevel for `session_id`, if one has been recorded.
    pub fn thinking_level_for(&self, session_id: &SessionId) -> Option<&ThinkingLevel> {
        self.thinking_level_by_session.get(session_id)
    }

    /// Record the ThinkingLevel for `session_id`.
    pub fn set_thinking_level(&mut self, session_id: SessionId, level: ThinkingLevel) {
        self.thinking_level_by_session.insert(session_id, level);
    }

    /// Borrow the TokenState for `session_id`, if one has been recorded.
    pub fn token_state_for(&self, session_id: &SessionId) -> Option<&TokenState> {
        self.token_state_by_session.get(session_id)
    }

    /// Replace the TokenState for `session_id`.
    pub fn set_token_state(&mut self, session_id: SessionId, state: TokenState) {
        self.token_state_by_session.insert(session_id, state);
    }

    /// Fold an arriving `StreamChunk` into the token state for
    /// `session_id`, creating a default state if none exists yet.
    pub fn apply_chunk_to_token_state(&mut self, session_id: &SessionId, chunk: &StreamChunk) {
        let entry = self
            .token_state_by_session
            .entry(session_id.clone())
            .or_default();
        entry.apply_chunk(chunk);
    }

    /// Borrow the workspace snapshot, if one has been recorded by
    /// `Action::WorkspaceInfoLoaded`.
    pub fn workspace(&self) -> Option<&WorkspaceInfo> {
        self.workspace.as_ref()
    }

    /// Replace (or clear) the workspace snapshot.
    pub fn set_workspace(&mut self, workspace: Option<WorkspaceInfo>) {
        self.workspace = workspace;
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn defaults_are_all_empty_or_false() {
        let store = AgentViewStore::default();
        assert!(store.current_session().is_none());
        assert!(store.navigation_target_session().is_none());
        assert!(store.workspace().is_none());
        assert_eq!(store.session_index(), (0, 0));
    }

    #[test]
    fn apply_chunk_to_token_state_creates_default_then_folds() {
        let mut store = AgentViewStore::default();
        let sid = SessionId::new("s-1");
        let chunk = StreamChunk::TokenUpdate {
            tokens: TokenTracker {
                input_tokens: 100,
                output_tokens: 50,
                cache_read_input_tokens: None,
                cache_creation_input_tokens: None,
                tokens_per_second: None,
                cumulative_billed_input: None,
                cumulative_billed_output: None,
                reasoning_tokens: None,
            },
        };
        store.apply_chunk_to_token_state(&sid, &chunk);
        let ts = store.token_state_for(&sid).copied().expect("token state");
        assert_eq!(ts.input_tokens, 100);
        assert_eq!(ts.output_tokens, 50);
    }
}
