//! `App::bootstrap` + subscriber-task spawn helpers (RPC-009 + RPC-012).
//!
//! RPC-012 rule [8]: `bootstrap` no longer pre-creates a session. It
//! ONLY calls `backend.list_work_units()` and seeds the BoardStore via
//! `Action::WorkUnitsLoaded`, then spawns the subscriber tasks
//! (work_units_rx / chunks_rx / logs_rx / status_changes_rx /
//! session_created_rx / checkpoints_progress_rx / git_state /
//! session_removed — RPC-434 added the last). Session creation is
//! deferred
//! until the user enters AgentView for the first time (lazy creation
//! inside `App::dispatch` on `Action::EnterWorkUnit`).

use anyhow::Result;
use tokio::sync::broadcast;
use tracing::debug;

use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-012 bootstrap: seed BoardStore via `backend.list_work_units()`
    /// and spawn the five subscriber tasks. Lazy session creation is
    /// deferred to the first `Action::EnterWorkUnit` / `OpenAgentView`.
    ///
    /// RPC-015 extension: additionally fire `backend.checkpoint_counts()`
    /// and dispatch `Action::CheckpointCountsLoaded(counts)` so the
    /// BoardView header paints the live counts on the very first frame.
    pub async fn bootstrap(&mut self) -> Result<()> {
        let units = self.backend.list_work_units().await?;
        self.dispatch(Action::WorkUnitsLoaded(units));
        // RPC-015: best-effort — failure is non-fatal because the header
        // already paints the default `Checkpoints: None` until the call
        // succeeds. We still surface the error to the tracing layer so
        // misconfigured cwds don't go silently undiagnosed.
        match self.backend.checkpoint_counts().await {
            Ok(counts) => {
                self.dispatch(Action::CheckpointCountsLoaded(counts));
            }
            Err(err) => {
                debug!("bootstrap: backend.checkpoint_counts() failed: {err}");
            }
        }
        // RPC-018: best-effort workspace fetch — failure is non-fatal
        // because the SessionFooter just hides the right-side content
        // until the real value arrives via a later refresh.
        match self.backend.get_workspace_info().await {
            Ok(info) => {
                self.dispatch(Action::WorkspaceInfoLoaded(info));
            }
            Err(err) => {
                debug!("bootstrap: backend.get_workspace_info() failed: {err}");
            }
        }
        self.spawn_subscriber_tasks();
        self.initialize_startup_model().await;
        self.initialize_default_thinking_level();
        // MUX-001 R6: load the persisted mux config (shared
        // fspec-config.json, tui.mux) so a restart restores the saved
        // grid; missing key → default preset.
        self.load_mux_config();
        // RPC-434: replay the connect-time open-session snapshot (the
        // server-pushed `SessionSnapshot` frame / embedded live map)
        // BEFORE the BUG-206 restore so the auto-restore re-asserts
        // the daemon active pointer AFTER the tabs exist (R4).
        self.replay_session_snapshot().await;
        // BUG-206: auto-restore the last active session — MEMORY-ONLY.
        // Resolution: the daemon's in-memory active-session pointer,
        // verified LIVE via list_live_sessions (never persisted
        // manifests). Fire-and-forget — no create fallback: the RPC-012
        // lazy create-on-enter-work-unit path is the only session
        // creation path on a fresh start. Nothing restorable → the
        // client lands on the board.
        self.spawn_session_restore(None);
        self.start_viewer_server().await;
        Ok(())
    }

    /// RPC-373: start the RPC-372 attachment viewer server bound to the current
    /// working directory. Best-effort / non-fatal — mirrors the
    /// `checkpoint_counts` pattern: on success store the handle + port so the
    /// board `D` key can build the FOUNDATION.md URL; on failure log via
    /// `debug!` and leave `viewer_port = None` (the `D` key then no-ops). cwd is
    /// resolved from `std::env::current_dir()` (the same source the rest of the
    /// TUI uses at startup).
    async fn start_viewer_server(&mut self) {
        let cwd = match std::env::current_dir() {
            Ok(cwd) => cwd,
            Err(err) => {
                debug!("bootstrap: current_dir() failed; viewer disabled: {err}");
                return;
            }
        };
        match codelet_attachment_viewer::start_viewer(cwd).await {
            Ok(handle) => {
                self.viewer_port = Some(handle.port);
                self.viewer_handle = Some(handle);
            }
            Err(err) => {
                debug!("bootstrap: start_viewer() failed; D key will no-op: {err}");
            }
        }
    }

    /// PROV-120: restore the TS-parity "first-available" startup model
    /// before the lazy `create_session` (the AgentView-mount analogue of
    /// TS `initializeModels`). Body in app/bootstrap_model.rs
    /// (300-LoC ceiling).
    pub(crate) async fn initialize_startup_model(&mut self) {
        super::bootstrap_model::initialize_startup_model(self).await
    }

    /// TUI-093: restore the persisted default thinking level to the active
    /// session at startup — the thinking-level analogue of
    /// `initialize_startup_model` and parity with the TS
    /// `useDefaultThinkingLevel` "load on mount → apply to active session".
    /// Best-effort: with no active session yet (the lazy `create_session` path)
    /// this just logs; the per-session guarded apply then fires when the first
    /// session is created/resumed via `refresh_session_chrome`.
    pub(crate) fn initialize_default_thinking_level(&mut self) {
        let loaded =
            codelet_sessions::default_thinking_level_persistence::load_default_thinking_level_opt();
        debug!(
            "bootstrap: default thinking level = {:?}",
            loaded.map(|l| l as u8)
        );
        if let Some(session) = self.active_session_rx_snapshot() {
            let _ = self.apply_default_thinking_level_if_needed(&session);
        }
    }

    /// Spawn the subscriber tasks per RPC-009 architecture note [2]
    /// (work_units_rx / chunks_rx / logs_rx / status_changes_rx /
    /// session_created_rx / checkpoints_progress_rx / git_state /
    /// session_removed — RPC-434 added the last). Each task runs on the
    /// host runtime (`tokio::spawn`, NEVER `tokio::runtime::Builder` or
    /// `Runtime::new()` per RPC-005 Q9).
    pub(crate) fn spawn_subscriber_tasks(&mut self) {
        // (a) work_units_rx → Action::WorkUnitsLoaded
        let tx = self.action_tx.clone();
        let backend = self.backend.clone();
        let mut rx = self.backend.work_units_rx();
        let work_units_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(units) => {
                        let _ = tx.send(Action::WorkUnitsLoaded(units));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("work_units subscriber lagged by {n}; resyncing");
                        if let Ok(snapshot) = backend.list_work_units().await {
                            let _ = tx.send(Action::WorkUnitsLoaded(snapshot));
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.subscriber_tasks.push(work_units_task);

        // (b) chunks_rx → Action::ChunkReceived (RPC-045: no active-session
        //     filter — every (SessionId, StreamChunk) is forwarded so
        //     background sessions accumulate scrollback + per-session
        //     state regardless of focus).
        let tx = self.action_tx.clone();
        let mut rx = self.backend.chunks_rx();
        let chunks_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok((id, chunk)) => {
                        let _ = tx.send(Action::ChunkReceived(id, chunk));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("chunks subscriber lagged by {n}; continuing");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.subscriber_tasks.push(chunks_task);

        // (c) logs_rx → tracing layer.
        let mut rx = self.backend.logs_rx();
        let logs_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(record) => {
                        debug!("rpc log: {record:?}");
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("logs subscriber lagged by {n}; continuing");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.subscriber_tasks.push(logs_task);

        // (d) RPC-045: status_changes_rx → Action::SessionStatusChanged.
        //     Push-driven replacement for the polling get_session_status
        //     path. Subscribers exit cleanly on RecvError::Closed; lag
        //     is logged via tracing::warn but does NOT crash the loop.
        let tx = self.action_tx.clone();
        let mut rx = self.backend.status_changes_rx();
        let status_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok((id, status)) => {
                        tracing::debug!(
                            session_id = %id,
                            status = ?status,
                            "[compaction-status] TUI received status change"
                        );
                        let _ = tx.send(Action::SessionStatusChanged(id, status));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("status_changes subscriber lagged by {n}; continuing");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.subscriber_tasks.push(status_task);

        // (e) RPC-385: session_created_rx → Action::SessionCreated(id).
        //     Push-driven notification for sessions the TUI did NOT itself
        //     initiate (spawned subordinates via AgentManager). Each event
        //     carries the new session's SessionInfo; we fold its id into
        //     Action::SessionCreated, whose handler is idempotent so an id
        //     that already has a tab is a no-op. Mirrors the work_units_rx
        //     lag-recovery shape: Lagged → debug + continue, Closed → break.
        let tx = self.action_tx.clone();
        let mut rx = self.backend.session_created_rx();
        let session_created_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(info) => {
                        let _ = tx.send(Action::SessionCreated(codelet_rpc_types::SessionId::new(
                            info.id,
                        )));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("session_created subscriber lagged by {n}; continuing");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.subscriber_tasks.push(session_created_task);

        // (f) TUI-109: checkpoints_progress_rx → Action::CheckpointsProgress.
        //     Per-item checkpoint-enumeration progress frames fed by
        //     FspecServiceImpl::list_checkpoints on the embedded
        //     transport. Transports that don't forward the frames
        //     (websocket) return a closed receiver — this loop exits
        //     immediately on RecvError::Closed and the dialog degrades
        //     to spinner + stage label only (TUI-107 behavior, no
        //     timeout logic). The App fold stale-drops frames that
        //     arrive after the list stage flushed.
        let tx = self.action_tx.clone();
        let mut rx = self.backend.checkpoints_progress_rx();
        let checkpoints_progress_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(progress) => {
                        let _ = tx.send(Action::CheckpointsProgress(progress));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("checkpoints_progress subscriber lagged by {n}; continuing");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.subscriber_tasks.push(checkpoints_progress_task);

        // (g) BUG-182: git_state_changed_rx → Action::GitStateChanged —
        // the ONE centralized git poll stream (R3: folds onto the
        // existing CheckpointCountsLoaded / WorkspaceInfoLoaded writer
        // paths; R4: re-fetches the visible lazy views). Body + the
        // websocket closed-receiver degradation live in
        // app/bootstrap_git_state.rs (300-LoC ceiling).
        self.spawn_git_state_subscriber();

        // (h) RPC-434: session_removed_rx → Action::SessionRemoved —
        // daemon-side session destruction (any client). Body in
        // app/bootstrap_session_removed.rs (300-LoC ceiling).
        self.spawn_session_removed_subscriber();

        // (i) RPC-437: session_cleared_rx → Action::SessionCleared —
        // daemon-side session clear (any client, including the
        // NAPI/Telegram entry point; the session stays LIVE). Body in
        // app/bootstrap_session_cleared.rs (300-LoC ceiling).
        self.spawn_session_cleared_subscriber();
    }
}
