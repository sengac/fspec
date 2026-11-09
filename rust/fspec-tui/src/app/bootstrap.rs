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
use super::work_units_refetch::{refetch_work_units, send_or_log};

impl App {
    /// RPC-012 bootstrap: seed BoardStore via `backend.list_work_units()`
    /// and spawn the five subscriber tasks. Lazy session creation is
    /// deferred to the first `Action::EnterWorkUnit` / `OpenAgentView`.
    ///
    /// RPC-015 extension: the checkpoint counts and (RPC-018) the
    /// workspace info load too — BUG-657: in a background task, so the
    /// first frame draws the board without waiting on git.
    pub async fn bootstrap(&mut self) -> Result<()> {
        let units = self.backend.list_work_units().await?;
        self.dispatch(Action::WorkUnitsLoaded(units));
        self.spawn_git_values_load();
        self.spawn_subscriber_tasks();
        self.spawn_load_workspaces(); // RPC-445
        self.spawn_session_manifest_issues(true);
        self.initialize_startup_model().await;
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
        Ok(())
    }

    /// BUG-657: load the git-derived bootstrap values off the bootstrap
    /// path; each result arrives on the action bus. The task is tracked
    /// like every other bootstrap load (`next_pending_task`).
    fn spawn_git_values_load(&mut self) {
        let tx = self.action_tx.clone();
        // RPC-015 / RPC-453: the start workspace's counts (the workspace
        // list is not loaded yet); a failure is shown.
        let checkpoints = self.checkpoint_calls();
        let backend = self.backend.clone();
        self.pending_tasks.push(tokio::spawn(async move {
            send_or_log(
                &tx,
                match checkpoints.counts().await {
                    Ok(counts) => Action::CheckpointCountsLoaded(counts),
                    Err(err) => Action::SessionOperationFailed(format!(
                        "Checkpoint counts could not be loaded: {err:#}"
                    )),
                },
            );
            // RPC-018: best-effort workspace fetch — failure is non-fatal
            // because the SessionFooter just hides the right-side content
            // until the real value arrives via a later refresh.
            match backend.get_workspace_info().await {
                Ok(info) => send_or_log(&tx, Action::WorkspaceInfoLoaded(info)),
                Err(err) => debug!("bootstrap: backend.get_workspace_info() failed: {err}"),
            }
        }));
    }

    /// PROV-120: restore the TS-parity "first-available" startup model
    /// before the lazy `create_session` (the AgentView-mount analogue of
    /// TS `initializeModels`). Body in app/bootstrap_model.rs
    /// (300-LoC ceiling).
    pub(crate) async fn initialize_startup_model(&mut self) {
        super::bootstrap_model::initialize_startup_model(self).await
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
                    Ok(units) => send_or_log(&tx, Action::WorkUnitsLoaded(units)),
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("work_units subscriber lagged by {n}; resyncing");
                        // BUG-379: a failed resync is shown, never dropped.
                        refetch_work_units(backend.as_ref(), &tx, "after missed updates").await;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.subscriber_tasks.push(work_units_task);
        self.spawn_workspace_work_units_subscriber(); // RPC-445
        self.spawn_workspace_git_state_subscriber(); // RPC-454

        let tx = self.action_tx.clone();
        let mut rx = self.backend.chunks_rx();
        let chunks_task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok((id, chunk)) => {
                        let _ = tx.send(super::dispatch_transcript_live::transcript_push_action(
                            id, chunk,
                        ));
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        debug!("chunks subscriber lagged by {n}; resynchronizing transcripts");
                        let _ = tx.send(Action::Transcript(
                            super::transcript_sync::TranscriptAction::Poll,
                        ));
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
                        let _ = tx.send(Action::SessionObserved(info));
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
        self.spawn_work_unit_focus_subscriber();
    }
}
