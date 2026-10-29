macro_rules! compose_backend_subscriptions {
    ($($collected:tt)*) => {
        compose_backend_finish! {
            $($collected)*
    /// RPC-037: subscribe to push-driven (SessionId, SessionStatus)
    /// transitions. Mirror of `chunks_rx` / `logs_rx` — each call
    /// returns a fresh receiver bound to the per-transport broadcast.
    /// The default returns a closed receiver so transports that don't
    /// yet wire push-status compile unchanged and gracefully degrade.
    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        let (tx, rx) = broadcast::channel(1);
        drop(tx);
        rx
    }

    /// RPC-385: subscribe to session-created events so the embedded TUI can
    /// append a tab for sessions it did not itself initiate (spawned
    /// subordinates via AgentManager). The payload is the new session's
    /// [`SessionInfo`] (its `.id` carries the SessionId). The embedded
    /// backend forwards the SessionManager's `session_created_tx().subscribe()`;
    /// the default returns a closed receiver so transports that do not yet
    /// wire the channel compile unchanged and gracefully degrade. Full remote
    /// (tarpc/websocket) parity is an explicit OUT-OF-SCOPE RPC follow-up.
    fn session_created_rx(&self) -> broadcast::Receiver<SessionInfo> {
        let (tx, rx) = broadcast::channel(1);
        drop(tx);
        rx
    }

    /// RPC-434: subscribe to session-removed events — a session was
    /// destroyed on the daemon (from this client or ANY other connected
    /// machine). The payload is the destroyed session's `SessionId`
    /// (nothing else: the client already holds all tab state). The
    /// embedded backend forwards the SessionManager's
    /// `session_removed_tx()`; the WebSocket backend splits the RPC-433
    /// `SessionEvent` wire stream onto this per-variant receiver. The
    /// default returns a closed receiver so transports that do not yet
    /// wire the channel compile unchanged (RPC-037 / PROV-109
    /// convention) — the App subscriber observes `RecvError::Closed`
    /// and exits (respawned on Reconnected per R5).
    fn session_removed_rx(&self) -> broadcast::Receiver<SessionId> {
        let (tx, rx) = broadcast::channel(1);
        drop(tx);
        rx
    }

    /// RPC-437: subscribe to session-cleared events — a session's
    /// history was cleared on the daemon (from this client or ANY other
    /// connected machine, including the NAPI/Telegram clear entry
    /// point). The payload is the cleared session's `SessionId` (the
    /// session stays LIVE — unlike `Removed`, nothing is destroyed).
    /// The embedded backend forwards the SessionManager's
    /// `session_cleared_tx()`; the WebSocket backend splits the RPC-433
    /// `SessionEvent` wire stream onto this per-variant receiver. The
    /// default returns a closed receiver so transports that do not yet
    /// wire the channel compile unchanged (RPC-037 / PROV-109
    /// convention) — the App subscriber observes `RecvError::Closed`
    /// and exits (respawned on Reconnected per R5).
    fn session_cleared_rx(&self) -> broadcast::Receiver<SessionId> {
        let (tx, rx) = broadcast::channel(1);
        drop(tx);
        rx
    }

    /// RPC-434: the connect-time open-session snapshot — the
    /// in-memory live session set the daemon holds at connect time
    /// (WS: the server-pushed `Envelope::SessionSnapshot` frame;
    /// embedded: `SharedFspecService::live_sessions_snapshot()`).
    /// `App::bootstrap` / `handle_reconnected` fold every entry
    /// through the existing `Action::SessionCreated` handler
    /// (idempotent append) BEFORE `spawn_session_restore`. The default
    /// returns an empty Vec so test doubles and degenerate backends
    /// compile unchanged — an empty snapshot seeds no tabs.
    async fn session_snapshot(&self) -> Vec<SessionInfo> {
        Vec::new()
    }

    /// TUI-109: subscribe to per-item checkpoint-enumeration progress
    /// frames (the `CheckpointsProgress` broadcast fed by
    /// `FspecServiceImpl::list_checkpoints`). The embedded transport
    /// forwards `SharedFspecService::checkpoints_progress_rx()`; the
    /// default returns a closed receiver so transports that don't
    /// forward the frames (e.g. WebSocket) degrade to spinner + stage
    /// label only — the subscriber observes `RecvError::Closed`
    /// immediately and the dialog never receives a counter (TUI-107
    /// behavior, no timeout logic needed).
    fn checkpoints_progress_rx(&self) -> broadcast::Receiver<CheckpointsProgress> {
        let (tx, rx) = broadcast::channel(1);
        drop(tx);
        rx
    }

    /// BUG-182: subscribe to the git-state push — fresh `GitState`
    /// snapshots broadcast by the shared layer's `GitStateWatcher` on
    /// every debounced `.git` change and every poll tick that produced a
    /// different snapshot (dedup). The embedded transport forwards
    /// `SharedFspecService::git_state_changed_rx()`; the default returns
    /// a closed receiver so transports that don't forward the frames
    /// (e.g. WebSocket) degrade gracefully — the App subscriber observes
    /// `RecvError::Closed` immediately and the header falls back to the
    /// bootstrap value + in-view `RefreshCheckpointCounts` re-poll
    /// (documented, like `checkpoints_progress_rx` for TUI-109).
    fn git_state_changed_rx(&self) -> broadcast::Receiver<GitState> {
        let (tx, rx) = broadcast::channel(1);
        drop(tx);
        rx
    }
        }
    };
}
