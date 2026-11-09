//! TUI-125: grow the agent window for a session the daemon announced (a
//! spawned sub-agent, another client's session) without moving the focus.
//!
//! Feature: spec/features/sessions-the-user-did-not-create-never-take-focus.feature
//!
//! [`MultiplexLayout::sync_window`] keeps the focus INDEX; a pane added
//! before the focused one would hand the focus to a different pane. This
//! variant keeps the focused pane itself: an agent pane keeps showing its
//! session, any other pane stays the same pane.

use codelet_rpc_types::SessionId;

use super::{MultiplexLayout, MuxPaneKind};

impl MultiplexLayout {
    /// [`Self::sync_window`] to `sessions`, keeping the focused pane.
    pub fn sync_window_keeping_focus(&mut self, sessions: &[SessionId]) {
        let before = self.focus;
        let focused_session = self.focused_session_id();
        let focused_kind = self.rendered_panes.get(before).copied();
        let ordinal = self
            .rendered_panes
            .iter()
            .take(before)
            .filter(|kind| Some(**kind) == focused_kind)
            .count();
        let flash = (self.flash_pane, self.flash_clock_ms);
        self.sync_window(sessions);
        let target = match (focused_session, focused_kind) {
            (Some(session), _) => self.agent_pane_showing(&session),
            (None, Some(kind)) => self.nth_pane_of_kind(kind, ordinal),
            (None, None) => None,
        };
        if let Some(index) = target {
            self.focus = index;
            self.flash_pane = flash
                .0
                .map(|pane| if pane == before { index } else { pane });
            self.flash_clock_ms = flash.1;
        }
    }

    /// The rendered pane index of the agent slot showing `session`.
    fn agent_pane_showing(&self, session: &SessionId) -> Option<usize> {
        let slot = self
            .window_session_ids()
            .iter()
            .position(|s| s == session)?;
        self.nth_pane_of_kind(MuxPaneKind::Agent, slot)
    }

    fn nth_pane_of_kind(&self, kind: MuxPaneKind, n: usize) -> Option<usize> {
        self.rendered_panes
            .iter()
            .enumerate()
            .filter(|(_, pane)| **pane == kind)
            .nth(n)
            .map(|(index, _)| index)
    }
}
