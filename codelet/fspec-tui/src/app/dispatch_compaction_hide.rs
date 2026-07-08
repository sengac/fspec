//! RPC-417 — 10-second COMPACTED-badge auto-hide timer.
//!
//! Feature: spec/features/agentview-compaction-badge-auto-hide.feature
//!
//! Mirrors the idiomatic `dispatch_reconnect.rs` arm/handle timer pattern
//! (spawn → sleep → self-addressed Action + seq-guard). On a
//! `StreamChunk::CompactionComplete` the store's per-session
//! compaction-reduction seq is bumped and `arm_compaction_hide` spawns a
//! `sleep(10s) → Action::ClearCompactionReduction { session_id, seq }`
//! task. When that action is later dispatched, `handle_clear_compaction_reduction`
//! clears the badge ONLY if the fired seq still matches the session's
//! current seq — a newer compaction or a `/clear` bumps the seq, turning
//! any stale fire into a silent no-op.
//!
//! Arming is runtime-guarded (`tokio::runtime::Handle::try_current`) so
//! synchronous `#[test]` paths (e.g. the RPC-100 tests) that dispatch
//! `CompactionComplete` without a tokio runtime do NOT panic — the badge
//! simply persists (pre-existing behaviour) under no runtime.

use std::time::Duration;

use codelet_rpc_types::SessionId;

use crate::components::Action;

use super::state::App;

/// TS TUI-044 parity: 10-second auto-hide window.
const COMPACTION_HIDE_DELAY: Duration = Duration::from_secs(10);

impl App {
    /// RPC-417: arm (or re-arm) the per-session 10-second auto-hide timer.
    /// Runtime-guarded so it's a no-op outside a tokio runtime. Aborts any
    /// prior timer for `session_id` before spawning the fresh one.
    pub(crate) fn arm_compaction_hide(&mut self, session_id: SessionId, seq: u64) {
        if tokio::runtime::Handle::try_current().is_err() {
            return;
        }
        if let Some(prev) = self.compaction_hide_handles.remove(&session_id) {
            prev.abort();
        }
        let action_tx = self.action_tx.clone();
        let sid = session_id.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(COMPACTION_HIDE_DELAY).await;
            let _ = action_tx.send(Action::ClearCompactionReduction {
                session_id: sid,
                seq,
            });
        });
        self.compaction_hide_handles.insert(session_id, handle);
    }

    /// RPC-417: on the fired auto-hide `Action::ClearCompactionReduction`,
    /// clear the session's COMPACTED badge — but ONLY when the fired seq
    /// still matches the session's current seq. A newer compaction or a
    /// `/clear` bumped the seq, so a stale timer is a silent no-op and the
    /// newer badge (or the cleared state) survives.
    pub(crate) fn handle_clear_compaction_reduction(&mut self, session_id: &SessionId, seq: u64) {
        if self
            .agent_view_store
            .compaction_reduction_seq_for(session_id)
            != seq
        {
            return;
        }
        self.agent_view_store.clear_compaction_reduction(session_id);
        self.compaction_hide_handles.remove(session_id);
    }
}
