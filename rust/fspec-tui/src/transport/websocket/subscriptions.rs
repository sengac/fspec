macro_rules! compose_websocket_subscriptions {
    ($($collected:tt)*) => {
        compose_websocket_workspace_checkpoints! {
            $($collected)*
    fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        // Best-effort: read the client slot synchronously via try_read.
        // If the supervisor is currently rebuilding the client this
        // returns a closed receiver — the App's subscriber task observes
        // RecvError::Closed and the supervisor's Action::Reconnected
        // triggers a fresh subscribe.
        match self.client.try_read() {
            Ok(guard) => match guard.as_ref() {
                Some(client) => client.work_units_rx(),
                None => empty_broadcast_rx(),
            },
            Err(_) => empty_broadcast_rx(),
        }
    }

    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        match self.client.try_read() {
            Ok(guard) => match guard.as_ref() {
                Some(client) => client.chunks_rx(),
                None => empty_broadcast_rx(),
            },
            Err(_) => empty_broadcast_rx(),
        }
    }

    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        match self.client.try_read() {
            Ok(guard) => match guard.as_ref() {
                Some(client) => client.logs_rx(),
                None => empty_broadcast_rx(),
            },
            Err(_) => empty_broadcast_rx(),
        }
    }

    /// RPC-037: subscribe to the WebSocket-pushed
    /// `Envelope::StatusUpdate` stream. When the client slot is `None`
    /// (disconnected) or the lock is contended, returns a degenerate
    /// receiver whose sender has been dropped — subscribers observe
    /// `RecvError::Closed` on the next `recv().await`, mirroring the
    /// chunks/logs degenerate-on-disconnect pattern.
    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        match self.client.try_read() {
            Ok(guard) => match guard.as_ref() {
                Some(client) => client.status_changes_rx(),
                None => empty_broadcast_rx(),
            },
            Err(_) => empty_broadcast_rx(),
        }
    }

    /// RPC-434: the WebSocket transport splits the single RPC-433
    /// session-event wire stream (`SessionEvent::Created` /
    /// `SessionEvent::Removed`) onto a per-subscriber
    /// `SessionInfo` broadcast. Each call grabs a fresh
    /// `session_events_rx()` receiver from the client, spawns a
    /// splitter task that forwards only the `Created` variant onto a
    /// per-subscriber channel, and returns that channel's receiver.
    /// The splitter exits when the wire stream closes (WS drop), and
    /// dropping its sender closes the per-subscriber broadcast —
    /// subscribers observe `RecvError::Closed` (RPC-010 rule [23]).
    /// When the client slot is `None` (disconnected) or the lock is
    /// contended, returns a closed receiver so the bootstrap
    /// subscriber exits immediately and is respawned on Reconnected
    /// (R2/R5). Replaces the never-firing RPC-385 OUT-OF-SCOPE stub.
    fn session_created_rx(&self) -> broadcast::Receiver<codelet_rpc_types::SessionInfo> {
        let (sub_tx, sub_rx) =
            broadcast::channel::<codelet_rpc_types::SessionInfo>(SESSION_EVENT_SPLITS_CAPACITY);
        let Some(mut events_rx) = self.client_session_events_rx() else {
            return empty_broadcast_rx();
        };
        tokio::spawn(async move {
            while let Ok(event) = events_rx.recv().await {
                if let codelet_rpc_types::SessionEvent::Created(info) = event {
                    if sub_tx.send(info).is_err() {
                        break;
                    }
                }
            }
            // `RecvError::Closed` — the wire stream closed (WS drop).
            // Dropping `sub_tx` here closes the per-subscriber channel.
        });
        sub_rx
    }

    /// RPC-434: sibling of `session_created_rx` — splits the RPC-433
    /// session-event wire stream onto a per-subscriber `SessionId`
    /// broadcast (the `Removed` variant only). Same closed-on-disconnect
    /// degradation (R2/R5).
    fn session_removed_rx(&self) -> broadcast::Receiver<SessionId> {
        let (sub_tx, sub_rx) = broadcast::channel::<SessionId>(SESSION_EVENT_SPLITS_CAPACITY);
        let Some(mut events_rx) = self.client_session_events_rx() else {
            return empty_broadcast_rx();
        };
        tokio::spawn(async move {
            while let Ok(event) = events_rx.recv().await {
                if let codelet_rpc_types::SessionEvent::Removed(id) = event {
                    if sub_tx.send(id).is_err() {
                        break;
                    }
                }
            }
            // `RecvError::Closed` — the wire stream closed (WS drop).
            // Dropping `sub_tx` here closes the per-subscriber channel.
        });
        sub_rx
    }

    /// RPC-437: sibling of `session_created_rx` / `session_removed_rx`
    /// — splits the RPC-433 session-event wire stream onto a
    /// per-subscriber `SessionId` broadcast (the `Cleared` variant
    /// only, the RPC-436 daemon-side fan-out frame). Same
    /// closed-on-disconnect degradation (R2/R5): when the client slot
    /// is `None` the closed receiver lets the bootstrap subscriber
    /// exit and it is respawned on Reconnected.
    fn session_cleared_rx(&self) -> broadcast::Receiver<SessionId> {
        let (sub_tx, sub_rx) = broadcast::channel::<SessionId>(SESSION_EVENT_SPLITS_CAPACITY);
        let Some(mut events_rx) = self.client_session_events_rx() else {
            return empty_broadcast_rx();
        };
        tokio::spawn(async move {
            while let Ok(event) = events_rx.recv().await {
                if let codelet_rpc_types::SessionEvent::Cleared(id) = event {
                    if sub_tx.send(id).is_err() {
                        break;
                    }
                }
            }
            // `RecvError::Closed` — the wire stream closed (WS drop).
            // Dropping `sub_tx` here closes the per-subscriber channel.
        });
        sub_rx
    }

    /// RPC-434: the connect-time open-session snapshot. Reads the
    /// fresh client's cached `SessionSnapshot` frame (the pump's
    /// watch-backed `session_snapshot_rx` replays it even when the
    /// subscriber arrived after the server pushed it); a bounded wait
    /// tolerates the initial frame arriving one tick after connect.
    /// Disconnected (client slot `None`) or frame-not-yet-received →
    /// empty Vec (seeds no tabs).
    async fn session_snapshot(&self) -> Vec<codelet_rpc_types::SessionInfo> {
        let guard = self.client.read().await;
        let Some(client) = guard.as_ref() else {
            return Vec::new();
        };
        let mut rx = client.session_snapshot_rx();
        match tokio::time::timeout(SNAPSHOT_WAIT, rx.recv()).await {
            Ok(Ok(snapshot)) => snapshot,
            _ => Vec::new(),
        }
    }
        }
    };
}
