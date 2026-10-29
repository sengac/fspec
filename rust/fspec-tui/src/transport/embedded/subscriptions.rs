macro_rules! compose_embedded_subscriptions {
    ($($collected:tt)*) => {
        compose_embedded_finish! {
            $($collected)*
    /// RPC-037: subscribe to the session manager's status broadcast.
    /// On the embedded path we read it directly from the
    /// `SharedFspecService`'s attached `SessionManagerHandle` so the
    /// receiver bypasses tarpc entirely (zero-cost path per RPC-002 §5.1).
    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        match self.service.session_manager() {
            Some(handle) => handle.status_changes_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// RPC-385: subscribe to the session manager's session-created broadcast.
    /// Read directly from the `SharedFspecService`'s attached
    /// `SessionManagerHandle` so the embedded TUI is notified of every newly
    /// created session — including spawned subordinates that bypass the
    /// TUI-initiated creation paths — bypassing tarpc entirely (the same
    /// zero-cost path `status_changes_rx` uses).
    fn session_created_rx(&self) -> broadcast::Receiver<codelet_rpc_types::SessionInfo> {
        match self.service.session_manager() {
            Some(handle) => handle.session_created_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// RPC-434: subscribe to the session manager's session-removed
    /// broadcast (zero-cost embedded path — the same direct handle read
    /// `status_changes_rx` / `session_created_rx` use). A session
    /// destroyed on the daemon fires `SessionEvent::Removed` on every
    /// connected client; the embedded TUI observes it here.
    fn session_removed_rx(&self) -> broadcast::Receiver<codelet_rpc_types::SessionId> {
        match self.service.session_manager() {
            Some(handle) => handle.session_removed_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// RPC-437: subscribe to the session manager's session-cleared
    /// broadcast (zero-cost embedded path — the same direct handle read
    /// `status_changes_rx` / `session_created_rx` use). A session
    /// cleared on the daemon (from ANY connected machine, including
    /// the NAPI/Telegram entry point) fires `SessionEvent::Cleared`
    /// on every connected client; the embedded TUI observes it here.
    /// Unlike `session_removed_rx`, the session stays LIVE — clear is
    /// repeatable, so the receiver must never be treated as
    /// terminal.
    fn session_cleared_rx(&self) -> broadcast::Receiver<codelet_rpc_types::SessionId> {
        match self.service.session_manager() {
            Some(handle) => handle.session_cleared_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// RPC-434: the connect-time open-session snapshot — delegates to
    /// the shared service's in-memory live map (no disk merge, no
    /// project filter — parity with the BUG-206 liveness gate).
    async fn session_snapshot(&self) -> Vec<codelet_rpc_types::SessionInfo> {
        self.service.live_sessions_snapshot()
    }

    /// TUI-109: subscribe to the checkpoint-enumeration progress
    /// broadcast. The channel lives directly on the
    /// `SharedFspecService` (not the session manager), so the embedded
    /// path forwards it with zero cost — `FspecServiceImpl::
    /// list_checkpoints` publishes a frame per collected item while the
    /// final Vec still returns through the tarpc RPC.
    fn checkpoints_progress_rx(
        &self,
    ) -> broadcast::Receiver<codelet_rpc_types::CheckpointsProgress> {
        self.service.checkpoints_progress_rx()
    }

    /// BUG-182: forward the shared service's git-state push so live
    /// `GitState` frames (watcher-driven, cwd-attached) reach the App
    /// subscriber on the embedded transport.
    fn git_state_changed_rx(&self) -> broadcast::Receiver<GitState> {
        self.service.git_state_changed_rx()
    }
        }
    };
}
