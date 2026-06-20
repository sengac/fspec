//! RPC-093 + RPC-095 — per-frame animation tick for `AgentView`, split into
//! a sibling module so `views/agent.rs` stays under the 300-LoC source-shape
//! ceiling while keeping canonical rustfmt formatting.

use std::time::Instant;

use codelet_rpc_types::SessionStatus;

use super::transition_driver;
use super::AgentView;
use crate::store::AgentViewStore;

impl AgentView {
    /// RPC-095 + RPC-093: per-frame animation tick. Returns
    /// `(session_status, is_loading)`.
    pub(super) fn tick_animation(
        &mut self,
        store: &AgentViewStore,
        sid: Option<&codelet_rpc_types::SessionId>,
    ) -> (Option<SessionStatus>, bool) {
        let session_status = sid.and_then(|s| store.session_status_for(s).copied());
        let is_busy = matches!(
            session_status,
            Some(SessionStatus::Running) | Some(SessionStatus::Compacting)
        );
        if is_busy && self.spinner_started_at.is_none() {
            self.spinner_started_at = Some(Instant::now());
        } else if !is_busy {
            self.spinner_started_at = None;
        }
        let is_loading = matches!(session_status, Some(SessionStatus::Running));
        self.animation_clock_ms = self.animation_clock_ms.saturating_add(16);
        let elapsed_ms = self
            .spinner_started_at
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);
        self.input_transition_state = transition_driver::advance_transition(
            session_status,
            &self.input_transition_state,
            self.last_spinner_line.as_deref(),
            elapsed_ms,
            self.animation_clock_ms,
        );
        (session_status, is_loading)
    }
}
