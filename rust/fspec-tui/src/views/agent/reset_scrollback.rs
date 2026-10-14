//! RPC-437 — AgentView scrollback-reset methods (extracted from
//! `views/agent.rs` so that orchestrator stays under the 300-LoC
//! ceiling pinned by the rpc013/rpc019/rpc020/rpc026/rpc094
//! source-shape suites).

use crate::store::AgentViewStore;

use super::AgentView;

impl AgentView {
    /// Local /clear path (RPC-046): reset the focused session's
    /// scrollback + the view-local input state.
    pub fn reset_scrollback(&mut self, store: &mut AgentViewStore) {
        // RPC-437: the local /clear path delegates to the shared
        // helper so the remote fold (handle_session_cleared, focused
        // case) and this path share ONE code path (DRY).
        self.reset_local_scrollback(store);
    }

    /// RPC-437 (DRY): the AgentView-local step of a local /clear —
    /// resets the FOCUSED session's scrollback context, blanks the
    /// input buffer and drops the slash/file popups. Shared by
    /// `reset_scrollback` (the local /clear path) and
    /// `App::handle_session_cleared` (the RPC-437 remote fold, only
    /// when the cleared session is the focused one) so both clear
    /// paths reset the view-local state through ONE code path. The
    /// per-session store-side chrome reset (status/tokens/compaction)
    /// is NOT here — that lives in `handle_session_cleared` so a
    /// background-session clear is complete without touching focus.
    pub fn reset_local_scrollback(&mut self, store: &mut AgentViewStore) {
        if let Some(ctx) = store.current_session_context_mut() {
            ctx.reset_scrollback();
        }
        self.input.reset();
        self.slash_popup = None;
        self.file_popup = None;
    }
}
