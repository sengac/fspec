//! MUX-002 — agent-window math: the agent slots form a WINDOW over the
//! ordered list of open agent sessions.
//!
//! Feature: spec/features/multiple-agent-panes-with-grouped-agent-view-cycling.feature
//!
//! - `window_start` is clamped to `max(0, sessions - agent_slots)` on every
//!   render / session change (`sync_window`).
//! - Agent slot `i` renders the session at `window_start + i`; unfilled agent
//!   slots are filtered out of the rendered pane list (`effective_panes`)
//!   and the remaining panes absorb the space.
//! - MUX-012: Shift+Left/Right never rotate the window (`focus_moves.rs`);
//!   it moves when a chip selects a session outside it
//!   (`focus_agent_pane_for_session`) and when a new agent fills the last
//!   slot (`note_session_created`).

use codelet_rpc_types::SessionId;

use super::{MultiplexLayout, MuxPaneKind};

impl MultiplexLayout {
    /// Number of agent slots in the configured pane list.
    pub fn agent_slot_count(&self) -> usize {
        self.config
            .layout
            .leaves()
            .iter()
            .filter(|k| **k == MuxPaneKind::Agent)
            .count()
    }

    /// The offset of the agent window into the ordered open-session list.
    pub fn window_start(&self) -> usize {
        self.window_start
    }

    /// The open-session list the window is positioned over (synced via
    /// [`Self::sync_window`]).
    pub fn open_session_ids(&self) -> &[SessionId] {
        &self.sessions
    }

    /// Clamp the agent window to the current open-session list and re-derive
    /// the rendered pane list. Called on every session change and every
    /// render. If the focused session is no longer inside the window (e.g.
    /// the session before it was closed), the window advances so the focused
    /// session stays visible in the last agent slot.
    pub fn sync_window(&mut self, sessions: &[SessionId]) {
        self.sessions = sessions.to_vec();
        let max_start = sessions.len().saturating_sub(self.agent_slot_count());
        self.window_start = self.window_start.min(max_start);
        if let Some(focused) = self.focused_session_id() {
            if !self.window_session_ids().contains(&focused) {
                if let Some(idx) = sessions.iter().position(|s| *s == focused) {
                    self.window_start = idx.min(max_start);
                }
            }
        }
        self.recompute_effective_panes();
        // Keep the cached pane rects in step with the (possibly changed)
        // rendered pane list — mouse hit-testing and pre-render
        // assertions read `pane_rects()` between renders.
        self.recompute_rects();
    }

    /// The session ids currently visible in the agent slots (window order).
    pub fn window_session_ids(&self) -> Vec<SessionId> {
        (0..self.agent_slot_count())
            .filter_map(|i| self.sessions.get(self.window_start + i).cloned())
            .collect()
    }

    /// The pane list actually rendered: agent slots beyond the open-session
    /// count are dropped (no blank panes); the remaining panes absorb the
    /// space. BUG-174: the list is floored at one pane — an empty
    /// derivation (all-agent layout, zero sessions) renders a single
    /// full-width TRANSIENT Board pane instead (see
    /// `recompute_effective_panes`).
    pub fn effective_panes(&self) -> &[MuxPaneKind] {
        &self.rendered_panes
    }

    /// The session id rendered in the focused pane, if it is an agent pane.
    pub fn focused_session_id(&self) -> Option<SessionId> {
        let kind = self.rendered_panes.get(self.focus)?;
        if *kind != MuxPaneKind::Agent {
            return None;
        }
        let agent_idx = self
            .rendered_panes
            .iter()
            .take(self.focus)
            .filter(|k| **k == MuxPaneKind::Agent)
            .count();
        self.sessions.get(self.window_start + agent_idx).cloned()
    }

    /// MUX-004 R-CHIPS / chip-click selection: rotate the agent window and
    /// move pane focus so the agent pane rendering `session` is the focused
    /// pane. This is what makes a clicked/entered chip *show* its selection
    /// — the chip's selected-highlight (MENU-010) paints only on the focused
    /// agent pane's window session, so without landing pane focus there the
    /// click would be reverted by `sync_mux_focus_to_session` (which treats
    /// the focused agent pane as the source of truth) and the highlight would
    /// stay on the previously focused pane's session.
    ///
    /// Returns the newly focused pane index. `None` when there is no rendered
    /// agent pane at all (an all-Board/Files/Ckpts grid) or the session is
    /// not in the open-session list (closed / Cleared — no chip to select).
    pub fn focus_agent_pane_for_session(&mut self, session: &SessionId) -> Option<usize> {
        // The rendered agent pane indices, in rendered_panes order. The
        // Nth of these renders `sessions[window_start + N]`.
        let agent_indices: Vec<usize> = self
            .rendered_panes
            .iter()
            .enumerate()
            .filter_map(|(i, k)| (k == &MuxPaneKind::Agent).then_some(i))
            .collect();
        let n_slots = agent_indices.len();
        if n_slots == 0 {
            return None;
        }
        let pos = self.sessions.iter().position(|s| s == session)?;
        // Rotate the window so `pos` is visible in an agent slot. This is the
        // SAME bound `sync_window` uses (n_slots == min(agent_slot_count,
        // sessions.len()) in every case), so the next render's re-derivation
        // keeps the window where we put it (the focused session stays in
        // `window_session_ids()` and no re-advance happens). A pure rotation
        // does not change the session count, so the rendered pane list (and
        // the focus index below) stays valid — no `recompute_effective_panes`
        // needed.
        let max_start = self.sessions.len().saturating_sub(n_slots);
        let window_start = pos.min(max_start);
        self.window_start = window_start;
        // The agent slot rendering `pos` is `pos - window_start`; focus the
        // rendered pane at that slot. `bump_focus` re-arms the flash (the
        // focus genuinely moved).
        let slot = pos - window_start;
        let pane = *agent_indices.get(slot)?;
        self.bump_focus(pane);
        Some(pane)
    }

    /// True iff a new-agent prompt was requested by the last `shift_right`
    /// (consumed by [`Self::note_session_created`]).
    pub fn pending_new_agent(&self) -> bool {
        self.pending_new_agent
    }

    /// A session was created while a mux new-agent prompt was pending:
    /// advance the window so the new (last) session lands in the last agent
    /// slot and move focus to that agent pane.
    pub fn note_session_created(&mut self) {
        if !self.pending_new_agent {
            return;
        }
        self.pending_new_agent = false;
        let max_start = self.sessions.len().saturating_sub(self.agent_slot_count());
        self.window_start = max_start;
        // Focus the last rendered agent pane (the slot the new session
        // just filled). MUX-006: re-arm the flash iff the focus moved.
        if let Some(idx) = self
            .rendered_panes
            .iter()
            .rposition(|k| *k == MuxPaneKind::Agent)
        {
            self.bump_focus(idx);
        }
    }

    /// The index in `panes` of the Nth agent slot (`0-based`; the last
    /// agent slot when `n` exceeds the count). MUX-002: the new-agent
    /// prompt's window rotation lands the new session in the LAST agent
    /// slot, which the focused pane must be.
    pub fn agent_pane_index(&self, panes: &[MuxPaneKind], n: usize) -> usize {
        panes
            .iter()
            .enumerate()
            .filter(|(_, k)| **k == MuxPaneKind::Agent)
            .map(|(i, _)| i)
            .nth(n.saturating_sub(1))
            .unwrap_or(0)
    }

    fn recompute_effective_panes(&mut self) {
        // BUG-183: user-closed panes (Esc on a lazy pane) are hidden from
        // the LIVE grid — the stored layout is untouched, so `/mux off` →
        // `/mux on` restores them (`closed_panes` clears on every
        // (re)layout). BUG-174: an all-agent layout with zero open
        // sessions draws a single TRANSIENT Board pane so every key path
        // stays alive; it is live-only and never stored. MUX-010: the
        // derivation lives in `rects.rs` (`refresh_visibility`).
        self.refresh_visibility();
        let n = self.rendered_panes.len().max(1);
        // MUX-006: a window clamp that moves the focus re-arms the flash.
        self.bump_focus(self.focus.min(n - 1));
    }
}
