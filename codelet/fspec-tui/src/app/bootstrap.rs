//! `App::bootstrap` + subscriber-task spawn helpers (RPC-009 + RPC-012).
//!
//! RPC-012 rule [8]: `bootstrap` no longer pre-creates a session. It
//! ONLY calls `backend.list_work_units()` and seeds the BoardStore via
//! `Action::WorkUnitsLoaded`, then spawns the three subscriber tasks
//! (work_units_rx / chunks_rx / logs_rx). Session creation is deferred
//! until the user enters AgentView for the first time (lazy creation
//! inside `App::dispatch` on `Action::EnterWorkUnit`).

use anyhow::Result;
use tokio::sync::broadcast;
use tracing::debug;

use crate::components::Action;

use super::state::App;

impl App {
    /// RPC-012 bootstrap: seed BoardStore via `backend.list_work_units()`
    /// and spawn the three subscriber tasks. Lazy session creation is
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

    /// PROV-120: restore the TS-parity "first-available" startup model before
    /// the lazy `create_session` (the AgentView-mount analogue of TS
    /// `initializeModels`). Reads the persisted `tui.lastUsedModel` (legacy
    /// `default-model.json` back-compat), lists providers, resolves
    /// restore-persisted → first-available, and commits the result via
    /// `set_default_model`. Mirroring TS, the commit is UNCONDITIONAL
    /// (idempotent; corrects a stale value) and there is NO hardcoded
    /// anthropic/claude fallback — a `None` resolution leaves the default
    /// unset so the PROV-101 decline path is preserved. Best-effort: any
    /// failure is surfaced to tracing and never blocks bootstrap.
    async fn initialize_startup_model(&mut self) {
        let persisted =
            codelet_sessions::last_used_model_persistence::load_persisted_model_string();
        let sections = match self.backend.list_providers().await {
            Ok(sections) => sections,
            Err(err) => {
                debug!("bootstrap: backend.list_providers() failed: {err}");
                return;
            }
        };
        if let Some(resolved) = codelet_sessions::startup_model_resolution::resolve_startup_model(
            &sections,
            persisted.as_deref(),
        ) {
            if let Err(err) = self.backend.set_default_model(resolved.model_string).await {
                debug!("bootstrap: backend.set_default_model() failed: {err}");
            }
        }
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

    /// Spawn the three subscriber tasks per RPC-009 architecture note [2].
    /// Each task runs on the host runtime (`tokio::spawn`, NEVER
    /// `tokio::runtime::Builder` or `Runtime::new()` per RPC-005 Q9).
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
    }
}
