//! RPC-406 — per-session tool-approval pause slot held by
//! `AgentViewStore`.
//!
//! Feature file: spec/features/inline-tool-approval-pause-prompt.feature
//!
//! Replaces the RPC-053 PauseDialog modal state: when the chunk-driven
//! fetcher (`app/dispatch_pause_hitl.rs::handle_pause_chunk`) resolves a
//! `Some(PauseState)` it dispatches `Action::PauseStateFetched`, whose
//! reducer writes the state here. The AgentView paints the inline
//! prompt from this slot only when the paused session is FOCUSED
//! (TS parity — `InputTransition.tsx:467-533` only ever shows the
//! focused session's pause).
//!
//! The triple-choice selection (0 = Allow Once, 1 = Allow Session,
//! 2 = Deny) is also per-session and lives here so the App reducer is
//! the single authority — the view never caches a selection that could
//! go stale (mirrors TS `triplePauseSelection` state +
//! `AgentView.tsx:1326-1331` reset semantics).

use codelet_rpc_types::{PauseState, SessionId};

use super::AgentViewStore;

/// Number of options on the triple prompt (wraparound modulus).
pub const TRIPLE_PAUSE_OPTIONS: usize = 3;

impl AgentViewStore {
    // ── Per-session PauseState slot ──────────────────────────────────────

    /// Read the active [`PauseState`] for `session`. `None` when the
    /// session is not paused (or the pause was already answered).
    pub fn pause_state_for(&self, session: &SessionId) -> Option<&PauseState> {
        self.pause_state_by_session.get(session)
    }

    /// Persist a fetched [`PauseState`] for `session`. Resets the
    /// triple-pause selection when the pause KIND changes (TS parity:
    /// `AgentView.tsx:1326-1331` resets whenever the kind is no longer
    /// `triple`); a same-kind refresh keeps the user's selection.
    pub fn set_pause_state(&mut self, session: SessionId, state: PauseState) {
        let kind_changed = self
            .pause_state_by_session
            .get(&session)
            .is_none_or(|prev| prev.kind != state.kind);
        if kind_changed {
            self.triple_pause_selection_by_session
                .insert(session.clone(), 0);
        }
        self.pause_state_by_session.insert(session, state);
    }

    /// Drop the pause slot for `session` and reset its selection —
    /// called when the user answers the prompt (Enter/Esc/Y/N) and
    /// when a `Running`/`Idle` chunk clears the pause server-side.
    pub fn clear_pause_state(&mut self, session: &SessionId) {
        self.pause_state_by_session.remove(session);
        self.triple_pause_selection_by_session.remove(session);
    }

    // ── Per-session triple-pause selection ───────────────────────────────

    /// Current triple-prompt selection for `session` (0 when unset).
    pub fn triple_pause_selection_for(&self, session: &SessionId) -> usize {
        self.triple_pause_selection_by_session
            .get(session)
            .copied()
            .unwrap_or(0)
    }

    /// Cycle the triple-prompt selection by `delta` with wraparound
    /// over the 3 options (TS `AgentView.tsx:4571-4580`).
    pub fn cycle_triple_pause_selection(&mut self, session: &SessionId, delta: i32) {
        let n = TRIPLE_PAUSE_OPTIONS as i32;
        let cur = self.triple_pause_selection_for(session) as i32;
        let next = (cur + delta).rem_euclid(n) as usize;
        self.triple_pause_selection_by_session
            .insert(session.clone(), next);
    }
}
