//! Side-specific handling of inbound envelopes: the [`InboundHandler`]
//! hook plus its server ([`ServerInbound`]) and client ([`ClientInbound`])
//! implementations used by [`super::run_envelope_pump`].

use codelet_rpc_types::{
    LogRecord, SessionEvent, SessionId, SessionInfo, SessionStatus, StreamChunk, WorkUnitInfo,
    WorkspaceGitState, WorkspaceWorkUnits,
};

/// Side-specific behaviour for envelopes that the local side does not
/// know how to act on by itself.
///
/// On the server side, reserved variants (and inbound `WorkUnitsUpdate`
/// — which a client should never push) are counted in
/// [`crate::ServerStats`] and a tracing warning is emitted. RPC-007:
/// inbound `Event` and `LogEvent` from a client are also unexpected
/// (they only flow server → client), so the server-side handler routes
/// them through `on_reserved` exactly the same way. On the client side,
/// `WorkUnitsUpdate(payload)` is demultiplexed onto a
/// `tokio::sync::watch` sender, `Event` payloads are demultiplexed onto
/// the chunks broadcast, and `LogEvent` payloads onto the logs
/// broadcast; everything else is debug-logged.
pub trait InboundHandler: Send + 'static {
    /// Called when a non-`Rpc`, non-`WorkUnitsUpdate`, non-`Event`,
    /// non-`LogEvent` envelope variant arrives — i.e. one of the
    /// reserved-and-rejected variants on the server side, or any
    /// unexpected payload-bearing variant on the client side.
    fn on_reserved(&self, variant: &'static str);

    /// Called when an `Envelope::WorkUnitsUpdate(payload)` arrives.
    ///
    /// Default implementation defers to [`Self::on_reserved`] — that's
    /// the right behaviour on the server side, where a client should
    /// never push a snapshot. Client-side handlers override this to
    /// forward the payload to their broadcast sender.
    fn on_work_units_update(&self, _payload: Vec<WorkUnitInfo>) {
        self.on_reserved("WorkUnitsUpdate");
    }

    /// RPC-445: an `Envelope::WorkspaceWorkUnitsUpdate` arrived. Server
    /// side rejects it as reserved; clients forward it.
    fn on_workspace_work_units(&self, _payload: WorkspaceWorkUnits) {
        self.on_reserved("WorkspaceWorkUnitsUpdate");
    }

    /// RPC-454: an `Envelope::WorkspaceGitStateUpdate` arrived. Server side
    /// rejects it as reserved; clients forward it.
    fn on_workspace_git_state(&self, _payload: WorkspaceGitState) {
        self.on_reserved("WorkspaceGitStateUpdate");
    }

    /// Called when an `Envelope::Event { session_id, chunk }` arrives
    /// (RPC-007). Default implementation defers to [`Self::on_reserved`]
    /// — chunks only flow server → client, so a client-pushed `Event`
    /// is rejected on the server side. Client-side handlers override
    /// this to forward the payload to their chunks broadcast.
    fn on_event(&self, _session_id: SessionId, _chunk: StreamChunk) {
        self.on_reserved("Event");
    }

    /// Called when an `Envelope::LogEvent(record)` arrives (RPC-007).
    /// Default implementation defers to [`Self::on_reserved`] — logs
    /// only flow server → client, so a client-pushed `LogEvent` is
    /// rejected on the server side. Client-side handlers override this
    /// to forward the payload to their logs broadcast.
    fn on_log_event(&self, _record: LogRecord) {
        self.on_reserved("LogEvent");
    }

    /// Called when an `Envelope::StatusUpdate { session_id, status }`
    /// arrives (RPC-037). Default implementation defers to
    /// [`Self::on_reserved`] — status updates only flow server →
    /// client, so a client-pushed `StatusUpdate` is rejected on the
    /// server side. Client-side handlers override this to forward the
    /// payload to their status broadcast.
    fn on_status_update(&self, _session_id: SessionId, _status: SessionStatus) {
        self.on_reserved("StatusUpdate");
    }

    /// Called when an `Envelope::SessionEvent(event)` arrives
    /// (RPC-433). Default implementation defers to
    /// [`Self::on_reserved`] — session lifecycle events only flow
    /// server → client, so a client-pushed `SessionEvent` is rejected on
    /// the server side (counted in `rejected_envelopes`, named in
    /// `rejected_variants` — rule R7). Client-side handlers override this
    /// to forward the payload to their session-events broadcast.
    fn on_session_event(&self, _event: SessionEvent) {
        self.on_reserved("SessionEvent");
    }

    /// Called when an `Envelope::SessionSnapshot(payload)` arrives
    /// (RPC-433). Default implementation defers to
    /// [`Self::on_reserved`] — the connect-time snapshot only flows
    /// server → client, so a client-pushed `SessionSnapshot` is rejected
    /// on the server side (rule R7). Client-side handlers override this
    /// to forward the payload to their snapshot watch.
    fn on_session_snapshot(&self, _payload: Vec<SessionInfo>) {
        self.on_reserved("SessionSnapshot");
    }
}

/// Server-side reserved-variant handler — increments a counter and warns.
pub struct ServerInbound {
    pub stats: crate::ServerStats,
}

impl InboundHandler for ServerInbound {
    fn on_reserved(&self, variant: &'static str) {
        self.stats
            .rejected_envelopes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut log) = self.stats.rejected_variants.lock() {
            log.push(variant);
        }
        tracing::warn!(
            variant = variant,
            "unsupported envelope variant; ignoring frame"
        );
    }
}

/// Client-side handler — stores `WorkUnitsUpdate` payloads in a
/// `tokio::sync::watch` sender and forwards `Event`/`LogEvent` payloads
/// onto their respective `tokio::sync::broadcast` senders so that
/// `FspecWsClient::chunks_rx()` / `logs_rx()` subscribers see them.
///
/// `WorkUnitsUpdate` uses a watch (RPC-006 rule [9] + cross-transport
/// parity scenario): subscribers created AFTER an initial frame has
/// been forwarded by the server still observe the latest snapshot.
/// Broadcast was insufficient there because broadcast receivers only
/// see messages sent *after* they call `subscribe()`, which raced
/// against the server's initial-snapshot push during connection
/// handshake.
///
/// `Event` and `LogEvent` use plain broadcast: chunks/logs are
/// individual events (not snapshot replacements), and the embedded
/// path's matching `chunks_rx`/`logs_rx` already returns a broadcast
/// receiver — keeping the same surface preserves transport-agnostic
/// UI code.
pub struct ClientInbound {
    pub work_units_tx: tokio::sync::watch::Sender<Option<Vec<WorkUnitInfo>>>,
    pub chunks_tx: tokio::sync::broadcast::Sender<(SessionId, StreamChunk)>,
    pub logs_tx: tokio::sync::broadcast::Sender<LogRecord>,
    /// RPC-037: status updates broadcast forwarded onto
    /// `FspecWsClient::status_changes_rx()` subscribers.
    pub status_tx: tokio::sync::broadcast::Sender<(SessionId, SessionStatus)>,
    /// RPC-433: session lifecycle events (Created / Removed) forwarded
    /// onto `FspecWsClient::session_events_rx()` subscribers. ONE stream
    /// for both variants preserves created/removed ordering per session.
    pub session_events_tx: tokio::sync::broadcast::Sender<SessionEvent>,
    /// RPC-433: connect-time open-session snapshot. WATCH (not
    /// broadcast): this is the exact fix documented for the work-units
    /// initial-snapshot race (the doc on `work_units_tx` above) — the
    /// TUI subscribes to session streams only AFTER bootstrap's initial
    /// RPCs, so a plain broadcast snapshot would be lost.
    pub session_snapshot_tx: tokio::sync::watch::Sender<Option<Vec<SessionInfo>>>,
    /// RPC-445: workspace-keyed work-unit snapshots forwarded onto
    /// `FspecWsClient::workspace_work_units_rx()` subscribers.
    pub workspace_work_units_tx: tokio::sync::broadcast::Sender<WorkspaceWorkUnits>,
    /// RPC-454: workspace-keyed git-state pushes.
    pub workspace_git_state_tx: tokio::sync::broadcast::Sender<WorkspaceGitState>,
}

impl InboundHandler for ClientInbound {
    fn on_reserved(&self, variant: &'static str) {
        tracing::debug!(
            variant = variant,
            "unexpected non-Rpc envelope from server; ignoring"
        );
    }

    fn on_work_units_update(&self, payload: Vec<WorkUnitInfo>) {
        // `send_replace` always succeeds (does not require subscribers).
        // We deliberately collapse intermediate updates: the watcher's
        // semantics are "latest snapshot wins", which matches the
        // RPC-006 broadcast-payload contract (architecture note 12).
        let _ = self.work_units_tx.send_replace(Some(payload));
    }

    fn on_event(&self, session_id: SessionId, chunk: StreamChunk) {
        // Broadcast to all subscribers. `send` returns `Err` only if
        // there are no active receivers — that's fine, the next
        // subscribe() call will see the next chunk.
        let _ = self.chunks_tx.send((session_id, chunk));
    }

    fn on_log_event(&self, record: LogRecord) {
        let _ = self.logs_tx.send(record);
    }

    fn on_status_update(&self, session_id: SessionId, status: SessionStatus) {
        // Broadcast to all subscribers. `send` returns `Err` only if
        // there are no active receivers — that's fine, the next
        // subscribe() call will see the next status update.
        let _ = self.status_tx.send((session_id, status));
    }

    fn on_session_event(&self, event: SessionEvent) {
        // Broadcast to all subscribers. `send` returns `Err` only if
        // there are no active receivers — that's fine, the next
        // subscribe() call will see the next session event.
        let _ = self.session_events_tx.send(event);
    }

    fn on_session_snapshot(&self, payload: Vec<SessionInfo>) {
        // `send_replace` always succeeds (does not require subscribers).
        // Watch semantics ("latest snapshot wins") match the
        // connect-time contract: a subscriber created after the
        // handshake frame is still replayed the cached snapshot via
        // `FspecWsClient::session_snapshot_rx`'s forwarder.
        let _ = self.session_snapshot_tx.send_replace(Some(payload));
    }

    fn on_workspace_work_units(&self, payload: WorkspaceWorkUnits) {
        let _ = self.workspace_work_units_tx.send(payload);
    }

    fn on_workspace_git_state(&self, payload: WorkspaceGitState) {
        // No subscriber is not an error: the push is best-effort.
        let _ = self.workspace_git_state_tx.send(payload);
    }
}
