impl SharedFspecService {
    /// Subscribe to the `(SessionId, StreamChunk)` broadcast (RPC-007).
    /// Both transports drain this same broadcast — the embedded transport
    /// returns the receiver directly to callers, the WS server's
    /// per-connection chunks_fanout task drains it and emits
    /// `Envelope::Event` frames.
    ///
    /// When a session manager is attached, subscribes to its
    /// per-process broadcast so all listeners — NAPI, embedded callers,
    /// WS fan-out — see the same chunks. Without a session manager,
    /// subscribes to a local broadcast that no producer publishes to
    /// (yields nothing).
    pub fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        match &self.session_manager {
            Some(handle) => handle.chunks_rx(),
            None => self.chunks_tx.subscribe(),
        }
    }

    /// Cloneable handle to the chunks broadcast sender — used by the
    /// session manager implementation (and the NAPI co-listener) to
    /// publish new chunks. Delegates to the session manager when
    /// attached so all subscribers see the same broadcast.
    pub fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)> {
        match &self.session_manager {
            Some(handle) => handle.chunks_tx(),
            None => self.chunks_tx.clone(),
        }
    }

    /// Subscribe to the `LogRecord` broadcast (RPC-007).
    ///
    /// Mirrors `chunks_rx` — when a session manager is attached, returns
    /// the session manager's own logs broadcast so all subscribers
    /// (NAPI co-listener, embedded callers, WS fan-out) see the same
    /// records.
    pub fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        match &self.session_manager {
            Some(handle) => handle.logs_rx(),
            None => self.logs_tx.subscribe(),
        }
    }

    /// Cloneable handle to the logs broadcast sender — used by the
    /// host's tracing::Layer to publish structured events. Delegates to
    /// the session manager when attached so the layer publishes onto
    /// the same broadcast that listeners observe via `logs_rx`.
    pub fn logs_tx(&self) -> broadcast::Sender<LogRecord> {
        match &self.session_manager {
            Some(handle) => handle.logs_tx(),
            None => self.logs_tx.clone(),
        }
    }

    /// TUI-109: subscribe to the checkpoint-enumeration progress
    /// broadcast. The embedded transport returns the receiver directly
    /// to the TUI; transports that don't forward the frames (e.g. the
    /// WebSocket transport) degrade to spinner-only automatically —
    /// no producer on the other side, no frames, no timeout logic.
    pub fn checkpoints_progress_rx(&self) -> broadcast::Receiver<CheckpointsProgress> {
        self.checkpoints_progress_tx.subscribe()
    }

    /// TUI-109: cloneable handle to the checkpoint-enumeration progress
    /// broadcast sender — `FspecServiceImpl::list_checkpoints` publishes
    /// frames here during enumeration.
    pub fn checkpoints_progress_tx(&self) -> broadcast::Sender<CheckpointsProgress> {
        self.checkpoints_progress_tx.clone()
    }

    /// BUG-182: subscribe to the git-state push — a fresh `GitState`
    /// snapshot broadcast on every debounced `.git` change AND every
    /// poll tick that produced a DIFFERENT snapshot (dedup; see
    /// `GitStateWatcher`). Returns a CLOSED receiver when no cwd (and
    /// therefore no watcher) is attached: embedded callers without a cwd
    /// get a receiver that immediately observes `RecvError::Closed`,
    /// mirroring `checkpoints_progress_rx`.
    pub fn git_state_changed_rx(&self) -> broadcast::Receiver<GitState> {
        match &self.git_state_watcher {
            Some(watcher) => watcher.subscribe(),
            None => {
                let (tx, rx) = broadcast::channel(DEFAULT_GIT_STATE_CAPACITY);
                drop(tx);
                rx
            }
        }
    }

    /// BUG-182: backfill snapshot of the most recent git state for the
    /// attached cwd (empty `GitState` when no cwd is attached). Pairs
    /// with [`Self::git_state_changed_rx`] for late-joining
    /// subscribers: the watcher only broadcasts frames emitted after
    /// they subscribe.
    pub fn git_state_snapshot(&self) -> GitState {
        match &self.git_state_watcher {
            Some(watcher) => watcher.snapshot(),
            None => GitState::default(),
        }
    }

    /// RPC-037: Subscribe to the `(SessionId, SessionStatus)` broadcast
    /// that carries push-driven session status updates. The embedded
    /// transport returns the receiver directly to callers; the WS
    /// server's per-connection `status_changes_fanout` task drains it
    /// and emits `Envelope::StatusUpdate` frames.
    ///
    /// When a session manager is attached, delegates to its
    /// per-process broadcast so all listeners — NAPI, embedded callers,
    /// WS fan-out — see the same status changes. Without a session
    /// manager, returns a degenerate receiver whose sender has been
    /// dropped (subscribers immediately observe `RecvError::Closed`).
    pub fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        match &self.session_manager {
            Some(handle) => handle.status_changes_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// Access the session manager handle, if one was provided.
    pub fn session_manager(&self) -> Option<&Arc<dyn SessionManagerHandle>> {
        self.session_manager.as_ref()
    }

    /// RPC-385: subscribe to session-created events. When a session manager is
    /// attached, delegates to its per-process broadcast so the embedded TUI
    /// sees every newly-created session (including spawned subordinates).
    /// Without a session manager, returns a degenerate receiver whose sender
    /// has been dropped (subscribers immediately observe `RecvError::Closed`).
    pub fn session_created_rx(&self) -> broadcast::Receiver<codelet_rpc_types::SessionInfo> {
        match &self.session_manager {
            Some(handle) => handle.session_created_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// RPC-433: subscribe to session-removed events. When a session manager is
    /// attached, delegates to its per-process broadcast so the WS server's
    /// per-connection fan-out task can wrap each id in
    /// `Envelope::SessionEvent(SessionEvent::Removed(_))` and push it to
    /// every connected client. Without a session manager, returns a
    /// degenerate receiver whose sender has been dropped (subscribers
    /// immediately observe `RecvError::Closed`).
    pub fn session_removed_rx(&self) -> broadcast::Receiver<codelet_rpc_types::SessionId> {
        match &self.session_manager {
            Some(handle) => handle.session_removed_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// RPC-436: subscribe to session-cleared events. When a session
    /// manager is attached, delegates to its per-process broadcast so the
    /// WS server's per-connection fan-out task can wrap each id in
    /// `Envelope::SessionEvent(SessionEvent::Cleared(_))` and push it to
    /// every connected client. Without a session manager, returns a
    /// degenerate receiver whose sender has been dropped (subscribers
    /// immediately observe `RecvError::Closed`).
    pub fn session_cleared_rx(&self) -> broadcast::Receiver<codelet_rpc_types::SessionId> {
        match &self.session_manager {
            Some(handle) => handle.session_cleared_rx(),
            None => {
                let (tx, rx) = broadcast::channel(1);
                drop(tx);
                rx
            }
        }
    }

    /// RPC-433: the in-memory live session map as a snapshot
    /// (`Vec<SessionInfo>`), the connect-time replay source for
    /// `Envelope::SessionSnapshot` (rule R3: no disk merge, no project
    /// filter — parity with the BUG-206 liveness gate). Delegates to the
    /// attached handle; empty when no session manager is attached (the
    /// degenerate no-manager daemon still sends an always-present,
    /// possibly-empty snapshot on connect).
    pub fn live_sessions_snapshot(&self) -> Vec<codelet_rpc_types::SessionInfo> {
        match &self.session_manager {
            Some(handle) => handle.live_session_infos(),
            None => Vec::new(),
        }
    }
}
