impl SharedFspecService {
    /// Construct the shared impl with a real workspace watcher (RPC-006).
    /// The session manager handle is left unset — calls to the session
    /// RPCs will return empty defaults until [`with_session_manager`] is
    /// used instead.
    pub fn new(watcher: Arc<WorkUnitsWatcher>) -> Self {
        let (chunks_tx, _) = broadcast::channel(DEFAULT_CHUNKS_CAPACITY);
        let (logs_tx, _) = broadcast::channel(DEFAULT_LOGS_CAPACITY);
        let (checkpoints_progress_tx, _) =
            broadcast::channel(DEFAULT_CHECKPOINTS_PROGRESS_CAPACITY);
        Self {
            watcher: ArcSwap::new(watcher),
            session_manager: None,
            focus_subscription: std::sync::Mutex::new(None),
            chunks_tx,
            logs_tx,
            list_work_units_calls: Arc::new(AtomicU64::new(0)),
            started_at: Instant::now(),
            stats: AsyncMutex::new(None),
            cwd: None,
            checkpoints_progress_tx,
            git_state_watcher: None,
            git_state_captures: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn work_unit_focus_rx(&self) -> broadcast::Receiver<(SessionId, WorkUnitFocus)> {
        match &self.session_manager {
            Some(handle) => handle.work_unit_focus_rx(),
            None => {
                let (sender, receiver) = broadcast::channel(1);
                drop(sender);
                receiver
            }
        }
    }

    /// Construct the shared impl with both a workspace watcher and a
    /// session manager handle (RPC-007). The host (rpc-server bin,
    /// EmbeddedTransport host, or rust/napi) constructs the concrete
    /// SessionManager and hands it here as `Arc<dyn SessionManagerHandle>`.
    pub fn with_session_manager(
        watcher: Arc<WorkUnitsWatcher>,
        session_manager: Arc<dyn SessionManagerHandle>,
    ) -> Self {
        let handle = Arc::downgrade(&session_manager);
        let subscription = codelet_core::work_unit_subscription::WorkUnitSubscription::new(
            &watcher,
            move |project| {
                if let Some(handle) = handle.upgrade() {
                    if let Err(error) = handle.reconcile_work_units(project) {
                        tracing::warn!(%error, "work-unit observation failed");
                    }
                }
            },
        );
        let (chunks_tx, _) = broadcast::channel(DEFAULT_CHUNKS_CAPACITY);
        let (logs_tx, _) = broadcast::channel(DEFAULT_LOGS_CAPACITY);
        let (checkpoints_progress_tx, _) =
            broadcast::channel(DEFAULT_CHECKPOINTS_PROGRESS_CAPACITY);
        Self {
            watcher: ArcSwap::new(watcher),
            session_manager: Some(session_manager),
            focus_subscription: std::sync::Mutex::new(Some(subscription)),
            chunks_tx,
            logs_tx,
            list_work_units_calls: Arc::new(AtomicU64::new(0)),
            started_at: Instant::now(),
            stats: AsyncMutex::new(None),
            cwd: None,
            checkpoints_progress_tx,
            git_state_watcher: None,
            git_state_captures: Arc::new(AtomicU64::new(0)),
        }
    }

    /// RPC-015: chainable builder method that attaches a workspace cwd
    /// to a freshly-constructed [`SharedFspecService`]. The cwd is
    /// passed into `codelet_git::ghost_commit::count_checkpoints` by
    /// `FspecService::checkpoint_counts`. When no cwd is attached,
    /// `checkpoint_counts()` returns the zero default.
    ///
    /// Example:
    /// ```ignore
    /// let service = Arc::new(
    ///     SharedFspecService::new(watcher).with_cwd(workspace_root.to_path_buf()),
    /// );
    /// ```
    pub fn with_cwd(mut self, cwd: PathBuf) -> Self {
        // BUG-182: attach the ONE centralized git-state watcher on the
        // same cwd — the debounced fs-watch covers every mutation source
        // (in-process agent tool calls, CLI in another terminal, raw
        // git update-ref) because every path lands under `.git`, and the
        // 10-second periodic poll catches working-tree changes that emit
        // no `.git` event.
        //
        // BUG-187: the capture hook bumps the per-service counter from
        // inside the blocking-pool capture, so tests (and future
        // observability) can count how often a capture actually ran.
        let captures = Arc::clone(&self.git_state_captures);
        self.git_state_watcher = Some(GitStateWatcher::with_captures(
            &cwd,
            Duration::from_secs(10),
            move || {
                captures.fetch_add(1, Ordering::SeqCst);
            },
        ));
        self.cwd = Some(cwd);
        self
    }

    /// RPC-015: borrow the workspace cwd attached via [`with_cwd`].
    /// Returns `None` when no cwd has been attached.
    pub fn cwd(&self) -> Option<&PathBuf> {
        self.cwd.as_ref()
    }

    /// RPC-011 rule [25]/[26]: atomically replace the watcher with a
    /// freshly-built one. Called by the daemon's SIGHUP handler so the
    /// workspace is re-walked without restarting the process or
    /// dropping any in-flight RPCs. Existing broadcast subscribers
    /// (tied to the OLD watcher's `subscribe()` receiver) stop seeing
    /// updates after the swap — they observe the silence as "watcher
    /// re-armed" and resync via `list_work_units_snapshot()`.
    pub fn rebuild_watcher(&self, new_watcher: WorkUnitsWatcher) {
        let mut subscription = self
            .focus_subscription
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *subscription = None;
        if let Some(manager) = &self.session_manager {
            let handle = Arc::downgrade(manager);
            *subscription = Some(
                codelet_core::work_unit_subscription::WorkUnitSubscription::new(
                    &new_watcher,
                    move |project| {
                        if let Some(handle) = handle.upgrade() {
                            if let Err(error) = handle.reconcile_work_units(project) {
                                tracing::warn!(%error, "replacement work-unit observation failed");
                            }
                        }
                    },
                ),
            );
        }
        self.watcher.store(Arc::new(new_watcher));
    }

    /// RPC-011: wire a `ServerStatsHandle` into the shared service so
    /// `health()` can return live counters. Called by the host
    /// transport (currently `bind_and_serve`) once it has constructed
    /// `ServerStats`. Idempotent — the last call wins.
    pub async fn set_stats(&self, stats: Arc<dyn ServerStatsHandle>) {
        let mut guard = self.stats.lock().await;
        *guard = Some(stats);
    }

    /// RPC-011: return the process startup instant. Exposed so the
    /// host transport (`bind_and_serve`) can pass the SAME instant
    /// into `ServerStats` so client-visible `uptime_secs` and any
    /// server-side bookkeeping agree.
    pub fn started_at(&self) -> Instant {
        self.started_at
    }

    /// Return the current snapshot from the watcher and increment the
    /// parity counter.
    pub fn list_work_units_snapshot(&self) -> Vec<WorkUnitInfo> {
        self.list_work_units_calls.fetch_add(1, Ordering::SeqCst);
        self.watcher.load().snapshot()
    }

    /// Read the list_work_units invocation counter.
    pub fn list_work_units_calls(&self) -> u64 {
        self.list_work_units_calls.load(Ordering::SeqCst)
    }

    /// BUG-187: how many GitState captures have run (on the blocking
    /// pool) since this service was constructed. Bumped by the capture
    /// hook wired in [`Self::with_cwd`].
    pub fn git_state_captures(&self) -> u64 {
        self.git_state_captures.load(Ordering::SeqCst)
    }

    /// Subscribe to live work-units updates from the underlying watcher.
    pub fn watcher_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
        self.watcher.load().subscribe()
    }

    /// Snapshot the current watcher state without incrementing the
    /// parity counter — used by the WS fan-out task on connect to send
    /// the initial snapshot frame.
    pub fn watcher_snapshot(&self) -> Vec<WorkUnitInfo> {
        self.watcher.load().snapshot()
    }
}
