//! App::dispatch routing for RPC-024 multi-session cycling.
//!
//! Factored out of `app/dispatch.rs` so the orchestrator file stays
//! under the 300-LoC ceiling pinned by `rpc013-source-shape.feature` +
//! `rpc024-source-shape.feature`.
//!
//! Hosts `handle_session_cycle`, the helper invoked from `App::dispatch`
//! when an `Action::SessionPrev` or `Action::SessionNext` arrives.
//!
//! Per RPC-024 rule [3]: on every session switch BEFORE mutating
//! `current_session_index`, the helper snapshots the live
//! `MultiLineInput` buffer into the outgoing
//! `SessionContext.input_draft`; AFTER mutating, the helper restores
//! the incoming session's saved draft back into the MultiLineInput.

use super::state::App;

impl App {
    /// Rotate the focused session by `delta` (-1 for SessionPrev, 1 for
    /// SessionNext) and round-trip the MultiLineInput buffer through the
    /// per-session `input_draft` slot. No-op when fewer than two
    /// sessions are open (single-session is a self-loop, empty is a
    /// no-op — both already encoded in
    /// [`crate::store::AgentViewStore::cycle_session`]).
    pub(crate) fn handle_session_cycle(&mut self, delta: isize) {
        // Snapshot the outgoing draft before the index moves so the
        // restore-on-return path picks it up unchanged.
        let outgoing_idx = self.agent_view_store.current_session_index();
        let outgoing_draft = self.navigator.agent.input.value();
        self.agent_view_store
            .set_input_draft(outgoing_idx, outgoing_draft);

        // Cycle the index with wrap-around (no-op for 0 or 1 sessions).
        self.agent_view_store.cycle_session(delta);

        // Restore the incoming session's saved draft into the live
        // MultiLineInput so the user sees their previous typing.
        let incoming_draft = self
            .agent_view_store
            .current_session_context()
            .map(|c| c.input_draft.clone())
            .unwrap_or_default();
        self.navigator.agent.input.set_value(&incoming_draft);
    }
}
