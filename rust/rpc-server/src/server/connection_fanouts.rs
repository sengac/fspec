//! Per-connection push fan-out tasks spawned by `handle_connection`
//! (RPC-006/007/011/037/433/436/445) and their teardown.

use super::fanouts::{
    log_events_fanout, status_changes_fanout, stream_chunks_fanout, work_units_fanout,
};
use super::session_fanouts::{
    session_cleared_fanout, session_created_fanout, session_removed_fanout,
};
use crate::envelope::Envelope;
use crate::ServerStats;
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::{SessionId, WorkUnitFocus};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc::UnboundedSender};
use tokio::task::JoinHandle;

/// Every fan-out task of one connection; aborted together on close.
pub(super) struct ConnectionFanouts(Vec<JoinHandle<()>>);

impl ConnectionFanouts {
    /// Spawn the fan-outs. `focus_rx` is subscribed by the caller BEFORE
    /// the connect-time snapshots are sent so no focus change is missed.
    pub(super) fn spawn(
        service: &Arc<SharedFspecService>,
        envelope_out_tx: &UnboundedSender<Envelope>,
        stats: &ServerStats,
        focus_rx: broadcast::Receiver<(SessionId, WorkUnitFocus)>,
    ) -> Self {
        let watcher_rx = service.watcher_rx();
        let envelope_out_tx_for_watcher = envelope_out_tx.clone();
        let lag_work_units = Arc::clone(&stats.lag_work_units);
        let last_watcher_event_at = Arc::clone(&stats.last_watcher_event_at);
        let watcher_fanout = tokio::spawn(work_units_fanout(
            watcher_rx,
            envelope_out_tx_for_watcher,
            lag_work_units,
            last_watcher_event_at,
        ));

        let chunks_rx = service.chunks_rx();
        let envelope_out_tx_for_chunks = envelope_out_tx.clone();
        let lag_chunks = Arc::clone(&stats.lag_chunks);
        let chunks_fanout = tokio::spawn(stream_chunks_fanout(
            chunks_rx,
            envelope_out_tx_for_chunks,
            lag_chunks,
        ));

        let logs_rx = service.logs_rx();
        let envelope_out_tx_for_logs = envelope_out_tx.clone();
        let lag_logs = Arc::clone(&stats.lag_logs);
        let logs_fanout = tokio::spawn(log_events_fanout(
            logs_rx,
            envelope_out_tx_for_logs,
            lag_logs,
        ));

        // RPC-037: per-connection fanout for push-driven session status
        // updates. Drains `SharedFspecService::status_changes_rx` (which
        // delegates to the attached `SessionManagerHandle`) and forwards
        // each `(SessionId, SessionStatus)` tuple as an
        // `Envelope::StatusUpdate` frame onto the per-connection
        // envelope-out channel — mirroring the chunks/logs fanout pattern.
        let status_rx = service.status_changes_rx();
        let envelope_out_tx_for_status = envelope_out_tx.clone();
        let lag_status = Arc::clone(&stats.lag_status);
        let status_fanout = tokio::spawn(status_changes_fanout(
            status_rx,
            envelope_out_tx_for_status,
            lag_status,
        ));

        // RPC-433: per-connection fanout for session lifecycle events
        // (rule R4). Two tasks mirror `status_changes_fanout`: one drains
        // `SharedFspecService::session_created_rx` and wraps each
        // `SessionInfo` in `Envelope::SessionEvent(Created)`, the other
        // drains `session_removed_rx` and wraps each id in
        // `Envelope::SessionEvent(Removed)`. Both fan out to EVERY
        // connected client — including the one that issued the RPC (no
        // per-client filtering, RPC-007 chunk fan-out precedent). They
        // share a single lag counter (`lag_session_events`), like
        // `lag_status` for the status fanout.
        let session_created_rx = service.session_created_rx();
        let envelope_out_tx_for_session_created = envelope_out_tx.clone();
        let lag_session_events = Arc::clone(&stats.lag_session_events);
        let session_created_fanout = tokio::spawn(session_created_fanout(
            session_created_rx,
            envelope_out_tx_for_session_created,
            lag_session_events,
        ));

        let session_removed_rx = service.session_removed_rx();
        let envelope_out_tx_for_session_removed = envelope_out_tx.clone();
        let session_removed_fanout = tokio::spawn(session_removed_fanout(
            session_removed_rx,
            envelope_out_tx_for_session_removed,
            Arc::clone(&stats.lag_session_events),
        ));

        let focus_fanout = tokio::spawn(crate::focus_fanout::run(
            focus_rx,
            envelope_out_tx.clone(),
            Arc::clone(service),
            Arc::clone(&stats.lag_session_events),
        ));

        // RPC-436: per-connection fanout for the session-cleared lifecycle
        // event (rule R2 of session-events-cleared-fan-out.feature). Mirrors
        // `session_created_fanout` / `session_removed_fanout`: drains
        // `SharedFspecService::session_cleared_rx` and wraps each id in
        // `Envelope::SessionEvent(SessionEvent::Cleared(_))`, fanning out to
        // EVERY connected client (no per-client filtering) and sharing the
        // `lag_session_events` counter with the created/removed fanouts.
        let session_cleared_rx = service.session_cleared_rx();
        let envelope_out_tx_for_session_cleared = envelope_out_tx.clone();
        let session_cleared_fanout = tokio::spawn(session_cleared_fanout(
            session_cleared_rx,
            envelope_out_tx_for_session_cleared,
            Arc::clone(&stats.lag_session_events),
        ));

        // RPC-445: pushes for workspaces opened besides the start workspace.
        let workspace_fanout = tokio::spawn(crate::workspace_fanout::run(
            service.workspace_work_units_rx(),
            envelope_out_tx.clone(),
            Arc::clone(&stats.lag_work_units),
            Envelope::WorkspaceWorkUnitsUpdate,
        ));
        // RPC-454: git-state pushes for the same opened workspaces.
        let workspace_git_fanout = tokio::spawn(crate::workspace_fanout::run(
            service.workspace_git_state_rx(),
            envelope_out_tx.clone(),
            Arc::clone(&stats.lag_work_units),
            Envelope::WorkspaceGitStateUpdate,
        ));

        Self(vec![
            watcher_fanout,
            chunks_fanout,
            logs_fanout,
            status_fanout,
            session_created_fanout,
            session_removed_fanout,
            focus_fanout,
            session_cleared_fanout,
            workspace_fanout,
            workspace_git_fanout,
        ])
    }

    /// Abort every task and wait for each to finish.
    pub(super) async fn shutdown(self) {
        for task in &self.0 {
            task.abort();
        }
        for task in self.0 {
            let _ = task.await;
        }
    }
}
