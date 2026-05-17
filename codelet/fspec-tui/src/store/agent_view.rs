//! AgentViewStore — single source of truth for the AgentView session
//! navigation state PLUS the per-session model/token/thinking chrome
//! state introduced by RPC-018 and the multi-session container
//! introduced by RPC-024.
//!
//! Feature files:
//!   - spec/features/rpc012-board-agent-navigation.feature
//!   - spec/features/rpc018-agent-chrome.feature
//!   - spec/features/rpc018-app-bootstrap.feature
//!   - spec/features/rpc024-multi-session-store.feature
//!   - spec/features/rpc025-source-shape.feature (per-session history)
//!
//! Cards: RPC-012, RPC-018, RPC-024, RPC-025 (parent RPC-002).

use std::collections::HashMap;

use codelet_rpc_types::{
    ContextFillInfo, ModelInfo, SessionId, StreamChunk, ThinkingLevel, TokenTracker,
    WorkspaceInfo,
};

pub mod history_state;
pub mod session_context;
pub use history_state::HistoryNavState;
pub use session_context::SessionContext;

/// Per-session token state derived from `StreamChunk::TokenUpdate` +
/// `StreamChunk::ContextFillUpdate` events. Mirrors `ExtractedTokenState`
/// from `src/tui/utils/tokenStateUtils.ts`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenState {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub context_fill_pct: u8,
}

impl TokenState {
    /// Fold an arriving chunk into this state.
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
        self.context_fill_pct = info.fill_percentage.min(100) as u8;
    }
}

/// AgentView session navigation state + per-session chrome state.
/// Mutated only on the App task.
#[derive(Debug, Default)]
pub struct AgentViewStore {
    // ── RPC-024 multi-session container ───────────────────────────────
    open_sessions: Vec<SessionContext>,
    current_session_index: usize,

    // ── Legacy navigation slots (preserved from RPC-012) ───────────────
    navigation_target_session: Option<SessionId>,
    current_work_unit_id: Option<String>,
    current_work_unit_status: Option<String>,
    show_create_session_dialog: bool,
    should_auto_create_session: bool,

    // ── RPC-018 chrome state ───────────────────────────────────────────
    model_info_by_session: HashMap<SessionId, ModelInfo>,
    token_state_by_session: HashMap<SessionId, TokenState>,
    thinking_level_by_session: HashMap<SessionId, ThinkingLevel>,
    workspace: Option<WorkspaceInfo>,

    // ── RPC-025 per-session history-recall state ────────────────────────
    /// Walk position into the cached history snapshot. See
    /// [`HistoryNavState`] for semantics.
    history_state_by_session: HashMap<SessionId, HistoryNavState>,
    /// Per-session cached history snapshot loaded by the first
    /// Action::HistoryPrev. Cleared on Action::InputSubmitted.
    cached_history_snapshot: HashMap<SessionId, Vec<String>>,
}

impl AgentViewStore {
    // ── RPC-024 multi-session accessors ─────────────────────────────────

    pub fn open_sessions(&self) -> &[SessionContext] {
        &self.open_sessions
    }

    pub fn open_sessions_mut(&mut self) -> &mut Vec<SessionContext> {
        &mut self.open_sessions
    }

    pub fn current_session_index(&self) -> usize {
        self.current_session_index
    }

    pub fn current_session_context(&self) -> Option<&SessionContext> {
        self.open_sessions.get(self.current_session_index)
    }

    pub fn current_session_context_mut(&mut self) -> Option<&mut SessionContext> {
        self.open_sessions.get_mut(self.current_session_index)
    }

    pub fn session_context_for(&self, id: &SessionId) -> Option<&SessionContext> {
        self.open_sessions.iter().find(|c| &c.id == id)
    }

    pub fn session_context_mut_for(&mut self, id: &SessionId) -> Option<&mut SessionContext> {
        self.open_sessions.iter_mut().find(|c| &c.id == id)
    }

    /// Append a fresh SessionContext to `open_sessions` and focus it.
    pub fn append_session(&mut self, ctx: SessionContext) {
        self.open_sessions.push(ctx);
        self.current_session_index = self.open_sessions.len().saturating_sub(1);
    }

    /// Rotate `current_session_index` by `delta` with wrap-around.
    pub fn cycle_session(&mut self, delta: isize) {
        let len = self.open_sessions.len();
        if len <= 1 {
            return;
        }
        let len_i = len as isize;
        let cur = self.current_session_index as isize;
        let next = (cur + delta).rem_euclid(len_i);
        self.current_session_index = next as usize;
    }

    /// Persist a string into `open_sessions[idx].input_draft`.
    pub fn set_input_draft(&mut self, idx: usize, value: String) {
        if let Some(ctx) = self.open_sessions.get_mut(idx) {
            ctx.input_draft = value;
        }
    }

    pub fn current_session(&self) -> Option<&SessionId> {
        self.current_session_context().map(|c| &c.id)
    }

    // ── Legacy slot accessors ────────────────────────────────────────────

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

    pub fn set_current_work_unit(&mut self, id: Option<String>, status: Option<String>) {
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
}

impl AgentViewStore {
    // ── RPC-018 chrome accessors ───────────────────────────────────────

    pub fn session_index(&self) -> (usize, usize) {
        let len = self.open_sessions.len();
        if len == 0 {
            (0, 0)
        } else {
            (self.current_session_index + 1, len)
        }
    }

    pub fn model_info_for(&self, session_id: &SessionId) -> Option<&ModelInfo> {
        self.model_info_by_session.get(session_id)
    }

    pub fn set_model_info(&mut self, session_id: SessionId, info: ModelInfo) {
        self.model_info_by_session.insert(session_id, info);
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
}

impl AgentViewStore {
    // ── RPC-025 per-session history accessors ───────────────────────────

    /// Borrow the current HistoryNavState for `session`, if any.
    pub fn history_state_for(&self, session: &SessionId) -> Option<&HistoryNavState> {
        self.history_state_by_session.get(session)
    }

    /// Mutable accessor — inserts a default state when missing.
    pub fn history_state_for_mut(&mut self, session: &SessionId) -> &mut HistoryNavState {
        self.history_state_by_session
            .entry(session.clone())
            .or_default()
    }

    /// Borrow the cached history snapshot for `session`, if loaded.
    pub fn cached_history_snapshot(&self, session: &SessionId) -> Option<&Vec<String>> {
        self.cached_history_snapshot.get(session)
    }

    /// Replace the cached history snapshot for `session`.
    pub fn set_history_snapshot(&mut self, session: SessionId, snapshot: Vec<String>) {
        self.cached_history_snapshot.insert(session, snapshot);
    }

    /// Reset the per-session HistoryNavState and clear the cached snapshot.
    pub fn reset_history_state(&mut self, session: &SessionId) {
        self.history_state_by_session
            .insert(session.clone(), HistoryNavState::default());
        self.cached_history_snapshot.remove(session);
    }
}

// Inline AgentViewStore unit tests were removed in RPC-024 — the
// equivalent coverage now lives in
// `tests/store_agent_view_multisession_rpc024.rs`.
