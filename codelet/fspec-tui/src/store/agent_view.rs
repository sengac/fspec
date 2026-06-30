//! AgentViewStore — single source of truth for the AgentView session
//! navigation state PLUS the per-session model/token/thinking chrome
//! state introduced by RPC-018 and the multi-session container
//! introduced by RPC-024. End-of-list navigation (RPC-096) lives in
//! the `navigation` sibling module. Feature files:
//! rpc012-board-agent-navigation, rpc018-agent-chrome,
//! rpc018-app-bootstrap, rpc024-multi-session-store,
//! rpc025-source-shape, agentview-shift-arrow-end-of-list-parity.
use std::collections::HashMap;

use codelet_rpc_types::{
    CompactionProgress, ModelInfo, SessionId, SessionStatus, ThinkingLevel, WorkUnitContext,
    WorkspaceInfo,
};

pub mod blocklist_state;
pub mod chrome_state;
pub mod chunk_processor;
pub mod chunk_wrap;
pub mod diff_decode;
pub mod diff_format;
pub mod history_state;
pub mod isolation_state;
pub mod markdown_table_render;
pub mod markdown_tables;
pub mod navigation;
pub mod pending_tool_diff;
pub mod role_state;
pub mod session_context;
pub mod supervisor_state;
pub mod token_state;
pub mod tool_args;
pub mod work_unit_state;
pub use history_state::HistoryNavState;
pub use isolation_state::IsolationState;
pub use navigation::NavTarget;
pub use session_context::SessionContext;
pub use token_state::TokenState;
pub use tool_args::extract_tool_args_display;

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
    /// RPC-337: last model id selected for each session (set on
    /// `Action::ModelSelected`). Seeds the full-screen ModelSelector's
    /// green `(current)` marker — `ModelInfo` carries only a display
    /// name, not the api model id, so this is the available source.
    selected_model_id_by_session: HashMap<SessionId, String>,

    // ── RPC-025 per-session history-recall state ────────────────────────
    /// Walk position into the cached history snapshot. See
    /// [`HistoryNavState`] for semantics.
    history_state_by_session: HashMap<SessionId, HistoryNavState>,
    /// Cached history snapshot per session — loaded by the first
    /// Action::HistoryPrev, cleared on Action::InputSubmitted.
    cached_history_snapshot: HashMap<SessionId, Vec<String>>,

    // ── RPC-022 per-session role overlay state ──────────────────────────
    /// Optional role overlay text per session. Populated by
    /// `Action::SessionRoleLoaded` / `Action::SetSessionRole`. Accessors
    /// live in `store/agent_view/role_state.rs`.
    role_by_session: HashMap<SessionId, String>,

    // ── RPC-045 per-session push-driven state ───────────────────────────
    // Accessors in `isolation_state.rs`.
    pub(crate) session_status_by_session: HashMap<SessionId, SessionStatus>,
    pub(crate) isolation_state_by_session: HashMap<SessionId, IsolationState>,
    pub(crate) debug_enabled_by_session: HashMap<SessionId, bool>,
    /// RPC-047: per-session live compaction progress.
    pub(crate) compaction_progress_by_session: HashMap<SessionId, CompactionProgress>,

    // ── RPC-050 per-session work-unit binding ───────────────────────────
    // Accessors live in `store/agent_view/work_unit_state.rs`.
    pub(crate) work_unit_context_by_session: HashMap<SessionId, WorkUnitContext>,

    // ── RPC-056 per-session blocklist-disabled set ──────────────────────
    // Accessors live in `store/agent_view/blocklist_state.rs`.
    pub(crate) blocklist_disabled_by_session: HashMap<SessionId, std::collections::HashSet<String>>,

    // ── RPC-061 per-session supervisor / subordinate state ──────────────
    // Accessors in `supervisor_state.rs`.
    pub(crate) supervisors_by_session: HashMap<SessionId, Vec<SessionId>>,
    pub(crate) supervisor_pending_count_by_session: HashMap<SessionId, usize>,

    // ── RPC-100 per-session compaction reduction percentage ─────────────
    // Populated by `dispatch_stream_chunks.rs::handle_stream_chunk_state_updates`
    // when a `StreamChunk::CompactionComplete` arrives, and cleared on
    // `SessionStateChange { state: Cleared }`. Read by
    // `views/agent/chrome_paint.rs::paint_header_and_role` to render the
    // `[X%: COMPACTED Y%]` SessionHeader suffix (mirrors TS
    // `AgentView.tsx:946-979` `setCompactionReductionRef`).
    // Accessors live in `store/agent_view/chrome_state.rs`.
    pub(crate) compaction_reduction_by_session: HashMap<SessionId, i32>,
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
    ///
    /// RPC-385: idempotent — if a SessionContext with the same `id` is already
    /// open, this is a no-op (no duplicate tab, no focus change). This makes
    /// the session-created broadcast safe for ALL creation paths: a
    /// user-initiated tab that already exists is left untouched, while a
    /// spawned subordinate is appended exactly once.
    ///
    /// This guard is the AUTHORITATIVE store-level dedup invariant. The
    /// matching guard in `App::handle_session_created`
    /// (app/dispatch_create_session_dialog.rs) is a side-effect-suppression
    /// optimization layered on top of this one; correctness does not depend
    /// on it.
    pub fn append_session(&mut self, ctx: SessionContext) {
        if self.open_sessions.iter().any(|c| c.id == ctx.id) {
            return;
        }
        self.open_sessions.push(ctx);
        self.current_session_index = self.open_sessions.len().saturating_sub(1);
    }

    /// RPC-096: focus the SessionContext at `index` (no-op if out of range).
    pub fn focus_session_index(&mut self, index: usize) {
        if index < self.open_sessions.len() {
            self.current_session_index = index;
        }
    }

    /// Persist a string into `open_sessions[idx].input_draft`.
    pub fn set_input_draft(&mut self, idx: usize, value: String) {
        if let Some(ctx) = self.open_sessions.get_mut(idx) {
            ctx.input_draft = value;
        }
    }

    /// RPC-026: remove `id` from `open_sessions` and clamp index.
    pub fn remove_session_if_open(&mut self, id: &SessionId) -> bool {
        let Some(idx) = self.open_sessions.iter().position(|c| &c.id == id) else {
            return false;
        };
        self.open_sessions.remove(idx);
        let len = self.open_sessions.len();
        if len == 0 {
            self.current_session_index = 0;
        } else if self.current_session_index >= len {
            self.current_session_index = len - 1;
        } else if idx < self.current_session_index {
            self.current_session_index -= 1;
        }
        true
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

    /// RPC-096: open the Create Session dialog WITHOUT auto-spawning.
    /// Mirrors TS `sessionStore.openCreateSessionDialog()`.
    pub fn request_create_session_dialog_no_auto(&mut self) {
        self.show_create_session_dialog = true;
    }

    pub fn clear_create_session_dialog(&mut self) {
        self.show_create_session_dialog = false;
        self.should_auto_create_session = false;
    }

    /// 1-based `(current, total)` index of the focused session.
    /// Kept inline in `agent_view.rs` (rather than `chrome_state.rs`)
    /// to satisfy the RPC-024 source-shape invariant
    /// `agent_view_store_no_longer_exposes_set_session_index`, which
    /// pins `pub fn session_index` to this file.
    pub fn session_index(&self) -> (usize, usize) {
        let len = self.open_sessions.len();
        if len == 0 {
            (0, 0)
        } else {
            (self.current_session_index + 1, len)
        }
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

// Other accessors live in store/agent_view/{chrome,role,work_unit,blocklist,supervisor}_state.rs.
