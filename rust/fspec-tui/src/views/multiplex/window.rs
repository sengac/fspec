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
//! - Shift+Right at the rightmost pane (any kind) prompts to create a new
//!   agent UNLESS the focused pane is the rightmost AGENT pane and the
//!   window can advance (then it rotates forward).
//! - Shift+Left on the rightmost agent pane rotates backward when
//!   `window_start > 0`; otherwise it falls through to normal focus
//!   movement. Shift+Left at the first pane STOPS (no wrap-around).

use codelet_rpc_types::SessionId;

use super::{MultiplexLayout, MuxPaneKind};

impl MultiplexLayout {
    /// Number of agent slots in the configured pane list.
    pub fn agent_slot_count(&self) -> usize {
        self.config
            .panes
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

    /// Focus the previous pane (Shift+Left fall-through). STOPS at the
    /// first pane — no wrap-around (MUX-002 rule 5). MUX-006: a focus
    /// move re-arms the flash; stopping at the first pane is a
    /// no-op (no re-flash).
    pub fn focus_prev(&mut self) {
        self.bump_focus(self.focus.saturating_sub(1));
    }

    /// Focus the next pane (Shift+Right fall-through). STOPS at the last
    /// rendered pane — no wrap-around. MUX-006: a focus move re-arms the
    /// flash; stopping at the last pane is a no-op (no re-flash).
    pub fn focus_next(&mut self) {
        let n = self.rendered_panes.len().max(1);
        if self.focus + 1 < n {
            self.bump_focus(self.focus + 1);
        }
    }

    /// Shift+Right (MUX-002 rule 5/6/10):
    ///
    /// - focused pane is the rightmost AGENT pane (no agent slot follows,
    ///   non-agent panes may): rotate the window forward when it can
    ///   advance; otherwise prompt for a new agent.
    /// - focused pane is the rightmost pane of ANY kind: prompt for a new
    ///   agent (regardless of kind).
    /// - otherwise: move focus one pane to the right (no wrap).
    ///
    /// Returns `true` when a new-agent prompt was requested.
    pub fn shift_right(&mut self) -> bool {
        if self.is_focused_pane_last_agent() {
            if self.window_can_advance() {
                self.window_start += 1;
                return false;
            }
            self.pending_new_agent = true;
            return true;
        }
        if self.is_rightmost_pane() {
            self.pending_new_agent = true;
            return true;
        }
        self.focus_next();
        false
    }

    /// Shift+Left. On the rightmost AGENT pane it rotates the window
    /// backward when `window_start > 0`; otherwise it falls through to
    /// normal focus movement (stops at the first pane).
    pub fn shift_left(&mut self) {
        if self.is_focused_pane_last_agent() && self.window_start > 0 {
            self.window_start -= 1;
            return;
        }
        self.focus_prev();
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

    /// True iff the agent window can advance one step.
    pub fn window_can_advance(&self) -> bool {
        self.window_start + self.agent_slot_count() < self.sessions.len()
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

    /// True iff the focus is on the last rendered pane.
    pub fn is_rightmost_pane(&self) -> bool {
        self.focus + 1 == self.rendered_panes.len()
    }

    /// True iff the focused pane is an agent pane AND no agent pane
    /// follows it in the rendered list (the "rightmost agent pane" —
    /// non-agent panes may sit to its right).
    pub fn is_focused_pane_last_agent(&self) -> bool {
        let Some(kind) = self.rendered_panes.get(self.focus) else {
            return false;
        };
        if *kind != MuxPaneKind::Agent {
            return false;
        }
        !self.rendered_panes[self.focus + 1..].contains(&MuxPaneKind::Agent)
    }

    fn recompute_effective_panes(&mut self) {
        let available = self.sessions.len();
        let mut agent_seen = 0usize;
        self.rendered_panes = self
            .config
            .panes
            .iter()
            .filter(|k| {
                if **k == MuxPaneKind::Agent {
                    let keep = agent_seen < available;
                    agent_seen += 1;
                    keep
                } else {
                    true
                }
            })
            .copied()
            .collect();
        // BUG-174: floor the rendered list at one pane. When the
        // derivation is EMPTY (an all-agent layout with zero open
        // sessions), render a single full-width TRANSIENT Board pane
        // instead: the grid never collapses to "footer only" and every
        // key path stays alive (Esc → BUG-165 exit dialog, Shift+Right
        // → new-agent prompt, Enter → session from the board). The
        // floor is LIVE-ONLY — `config.panes` is untouched, so the
        // saved all-agent layout restores verbatim once a session
        // fills an agent slot.
        if self.rendered_panes.is_empty() {
            self.rendered_panes = vec![MuxPaneKind::Board];
        }
        let n = self.rendered_panes.len().max(1);
        // MUX-006: a window clamp that moves the focus re-arms the flash.
        self.bump_focus(self.focus.min(n - 1));
    }
}
