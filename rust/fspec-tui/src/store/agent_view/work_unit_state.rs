//! RPC-050 — per-session work-unit binding state held by
//! `AgentViewStore`.
//!
//! Feature files:
//!   - spec/features/work-unit-attach-binding.feature
//!   - spec/features/slash-command-detach-and-work-unit-binding.feature
//!
//! This sub-module hosts the AgentViewStore accessors for the new
//! per-session `work_unit_context_by_session: HashMap<SessionId,
//! WorkUnitContext>` slot — updated by `Action::WorkUnitAttached`
//! (BoardView attach path) and cleared by `Action::WorkUnitDetached`
//! (`/detach` slash command). Read by the SessionHeader chip renderer
//! in `views/agent.rs::render_with_store`.
//!
//! Also hosts `reset_token_state(&SessionId)` — invoked by the
//! `Action::WorkUnitDetached` arm to mirror the TS
//! `prepareForNewSession` tokenUsage reset.
//!
//! The block lives in its own sub-module so the parent `agent_view.rs`
//! continues to satisfy the 300-LoC source-shape ceiling pinned by
//! `rpc025-source-shape.feature` and `slash-command-detach-source-shape.feature`.

use codelet_rpc_types::{SessionId, WorkUnitContext};

use super::AgentViewStore;

impl AgentViewStore {
    /// Borrow the per-session `WorkUnitContext` bound to `session`, if any.
    pub fn work_unit_context_for(&self, session: &SessionId) -> Option<&WorkUnitContext> {
        self.work_unit_context_by_session.get(session)
    }

    /// Bind a `WorkUnitContext` to `session`. Replaces any existing
    /// binding. Mutated only on the App task.
    pub fn set_work_unit_context(&mut self, session: SessionId, ctx: WorkUnitContext) {
        self.work_unit_context_by_session.insert(session, ctx);
    }

    /// Clear the per-session work-unit binding for `session`. No-op
    /// when no binding exists.
    pub fn clear_work_unit_context(&mut self, session: &SessionId) {
        self.work_unit_context_by_session.remove(session);
    }

    /// RPC-050: wipe the cached TokenState for `session` so the
    /// SessionHeader's token badges reset to defaults on the next
    /// render. Mirrors TS `prepareForNewSession`'s tokenUsage reset.
    pub fn reset_token_state(&mut self, session: &SessionId) {
        self.token_state_by_session.remove(session);
    }
}
