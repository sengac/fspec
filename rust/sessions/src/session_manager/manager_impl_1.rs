//! SessionManager inherent methods (part 1).

use super::*;

impl SessionManager {
    /// Create a new session manager with default no-op hooks. BUG-420: the
    /// manager only exists inside the `Arc` that holds its own self-reference
    /// (`Arc::new_cyclic`), so every session it creates can reach it.
    pub fn new() -> Arc<Self> {
        let (chunks_tx, _) = broadcast::channel(SUPERVISOR_BROADCAST_CAPACITY);
        let (logs_tx, _) = broadcast::channel(SUPERVISOR_BROADCAST_CAPACITY);
        let (status_changes_tx, _) = broadcast::channel(SUPERVISOR_BROADCAST_CAPACITY);
        // RPC-385: session-creation events use the supervisor broadcast
        // capacity because subscribers must tolerate bursts (e.g. the
        // lag-recovery path can flood many events at once). A lagged receiver
        // recovers via RecvError::Lagged rather than losing correctness: the
        // TUI's append is idempotent, so a dropped/replayed event never
        // produces a duplicate or missing tab.
        let (session_created_tx, _) = broadcast::channel(SUPERVISOR_BROADCAST_CAPACITY);
        // RPC-433: removal events share the created-events capacity and
        // lag-recovery contract — the TUI close path is idempotent, so a
        // lagged receiver recovers via RecvError::Lagged without producing
        // a wrong tab state (a missed removal is healed by the next
        // connect-time SessionSnapshot on reconnect).
        let (session_removed_tx, _) = broadcast::channel(SUPERVISOR_BROADCAST_CAPACITY);
        // RPC-436: cleared events share the same capacity + lag-recovery
        // contract — the TUI clear path is idempotent (resetting an
        // already-empty scrollback is a no-op), so a lagged receiver
        // recovers via RecvError::Lagged without producing a wrong state.
        let (session_cleared_tx, _) = broadcast::channel(SUPERVISOR_BROADCAST_CAPACITY);
        let (work_unit_focus_tx, _) = broadcast::channel(SUPERVISOR_BROADCAST_CAPACITY);
        let default_hooks: Arc<dyn SessionManagerHooks> = Arc::new(NoopSessionManagerHooks);
        Arc::new_cyclic(|self_weak| Self {
            sessions: RwLock::new(IndexMap::new()),
            chain_of_command: crate::chain_of_command::ChainOfCommand::new(),
            active_session_id: RwLock::new(None),
            scheduler_handle: RwLock::new(None),
            // BUG-463: construction never reads disk; the daemon restores the
            // persisted default with `restore_persisted_default_model`.
            default_model: RwLock::new(None),
            chunks_tx,
            logs_tx,
            status_changes_tx,
            session_created_tx,
            session_removed_tx,
            session_cleared_tx,
            work_unit_focus_tx,
            hooks: ArcSwap::from_pointee(default_hooks),
            self_weak: self_weak.clone(),
            default_project: OnceLock::new(),
        })
    }
    /// Replace the [`SessionManagerHooks`] implementation. Called by
    /// the napi side at startup to install `NapiSessionManagerHooks`.
    pub fn set_hooks(&self, hooks: Arc<dyn SessionManagerHooks>) {
        self.hooks.store(Arc::new(hooks));
    }
    /// WT-009: crate-visible hooks accessor — the `SessionManagerHandle`
    /// impl (handle_impl.rs) re-spawns the footer poller on detach.
    pub(crate) fn hooks(&self) -> Arc<dyn SessionManagerHooks> {
        let guard = self.hooks.load();
        (**guard).clone()
    }
    /// Accessor for the chunks broadcast sender.
    pub fn chunks_tx(&self) -> &broadcast::Sender<(SessionId, StreamChunk)> {
        &self.chunks_tx
    }
    /// Accessor for the logs broadcast sender.
    pub fn logs_tx(&self) -> &broadcast::Sender<LogRecord> {
        &self.logs_tx
    }
    /// Accessor for the status-changes broadcast sender.
    pub fn status_changes_tx(&self) -> &broadcast::Sender<(SessionId, SessionStatus)> {
        &self.status_changes_tx
    }
    /// RPC-385: accessor for the session-created broadcast sender. The
    /// embedded TUI transport subscribes to this so spawned subordinate
    /// sessions (created outside the TUI) become visible as tabs.
    pub fn session_created_tx(&self) -> &broadcast::Sender<SessionInfo> {
        &self.session_created_tx
    }
    /// RPC-433: accessor for the session-removed broadcast sender. The WS
    /// server's per-connection fan-out task subscribes to this and wraps
    /// each id in `Envelope::SessionEvent(SessionEvent::Removed(_))`.
    pub fn session_removed_tx(&self) -> &broadcast::Sender<SessionId> {
        &self.session_removed_tx
    }
    /// RPC-436: accessor for the session-cleared broadcast sender. The WS
    /// server's per-connection fan-out task subscribes to this and wraps
    /// each id in `Envelope::SessionEvent(SessionEvent::Cleared(_))`.
    pub fn session_cleared_tx(&self) -> &broadcast::Sender<SessionId> {
        &self.session_cleared_tx
    }
    /// RPC-433: the in-memory live session map as a snapshot
    /// (`Vec<SessionInfo>`), WITHOUT the persisted-manifest merge and
    /// WITHOUT project filtering that `list_sessions` applies. This is
    /// the connect-time replay source for
    /// `Envelope::SessionSnapshot` (rule R3 of
    /// session-events-daemon-fanout.feature): parity with the BUG-206
    /// liveness gate — it MUST NOT read disk.
    pub fn live_session_infos(&self) -> Vec<SessionInfo> {
        self.sessions
            .read()
            .expect("sessions lock poisoned")
            .values()
            .map(|s| s.get_info())
            .collect()
    }
    /// SCHED-004: Set the default model for scheduled session spawning.
    /// PROV-119: also persists the non-empty choice to disk
    /// (`<data_dir>/default-model.json`) so it survives a process restart. The
    /// write is best-effort: a failure is logged and never propagates, so
    /// session creation is never blocked by a persistence error.
    /// PROV-122: additionally persist the canonical `tui.lastUsedModel` in
    /// `fspec-config.json` alongside the legacy store (kept for back-compat) so
    /// the no-session selection path produces the key PROV-120's read path
    /// prefers. Both writes are best-effort and non-fatal.
    /// Set the default model and persist it (PROV-119 default-model file,
    /// PROV-122 fspec-config.json tui.lastUsedModel). BUG-411: an empty model
    /// string and a failed save are errors; the in-memory default is updated
    /// before persisting.
    pub fn set_default_model(&self, model: &str) -> Result<(), String> {
        if model.is_empty() {
            return Err("an empty model string is not a default model".to_string());
        }
        *self
            .default_model
            .write()
            .map_err(|_| "the default_model lock is poisoned".to_string())? =
            Some(model.to_string());
        crate::default_model_persistence::save_default_model(model)
            .map_err(|error| format!("cannot persist the default model {model}: {error}"))?;
        crate::last_used_model_persistence::save_persisted_model_string(model)
            .map_err(|error| format!("cannot persist the last used model {model}: {error}"))
    }
    /// BUG-613 (product owner, 2026-11-07): when the default model is one
    /// `deleted` names, clear it in memory, in default-model.json and in
    /// tui.lastUsedModel, and return the cleared model.
    pub fn clear_default_model_if(
        &self,
        deleted: impl Fn(&str) -> bool,
    ) -> Result<Option<String>, String> {
        let mut default = self
            .default_model
            .write()
            .map_err(|_| "the default_model lock is poisoned".to_string())?;
        let Some(model) = default.clone().filter(|model| deleted(model)) else {
            return Ok(None);
        };
        *default = None;
        drop(default);
        crate::default_model_persistence::clear_default_model()
            .map_err(|error| format!("cannot forget the default model {model}: {error}"))?;
        crate::last_used_model_persistence::forget_persisted_model_string(&model)
            .map_err(|error| format!("cannot forget the last used model {model}: {error}"))?;
        tracing::info!(%model, "cleared the default model: it was deleted");
        Ok(Some(model))
    }
    /// PROV-119 / BUG-463: load the persisted default model into this
    /// manager (without re-persisting it). A missing file leaves no default;
    /// an unreadable or malformed file, or an unresolvable data directory, is
    /// returned.
    pub fn restore_persisted_default_model(&self) -> Result<(), String> {
        let model = crate::default_model_persistence::load_default_model()?;
        *self
            .default_model
            .write()
            .map_err(|_| "the default_model lock is poisoned".to_string())? = model;
        Ok(())
    }
    /// SCHED-004: Get the default model string for scheduled session spawning.
    pub fn get_default_model(&self) -> Option<String> {
        self.default_model
            .read()
            .expect("default_model lock poisoned")
            .clone()
    }
    /// SCHED-006: Get the current number of sessions.
    pub async fn session_count(&self) -> usize {
        self.sessions.read().expect("sessions lock poisoned").len()
    }
    /// SCHED-006: Get all live session IDs for sweep detection.
    pub async fn live_session_ids(&self) -> Vec<Uuid> {
        self.sessions
            .read()
            .expect("sessions lock poisoned")
            .keys()
            .copied()
            .collect()
    }
    /// SCHED-006: Find a session by its schedule_name metadata.
    pub async fn find_session_by_schedule_name(&self, schedule_name: &str) -> Option<Uuid> {
        let sessions = self.sessions.read().expect("sessions lock poisoned");
        for (id, session) in sessions.iter() {
            let name = session.schedule_name.read().expect("schedule_name lock");
            if name.as_deref() == Some(schedule_name) {
                return Some(*id);
            }
        }
        None
    }
    /// Create a new background session (generates new UUID)
    pub async fn create_session(&self, model: &str, project: &str) -> Result<String, String> {
        let id = Uuid::new_v4();
        self.create_session_with_id(
            &id.to_string(),
            model,
            project,
            &format!("Session {}", &id.to_string()[..8]),
        )
        .await?;
        Ok(id.to_string())
    }
}
