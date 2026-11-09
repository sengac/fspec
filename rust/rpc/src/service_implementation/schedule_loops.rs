macro_rules! compose_rpc_schedule_loops {
    ($($collected:tt)*) => {
        compose_rpc_session_mcp! {
            $($collected)*
    // ─────────────────────────────────────────────────────────────────
    // RPC-058 — /schedule wiring.
    // ─────────────────────────────────────────────────────────────────

    async fn schedule_add(self, _ctx: Context, job: ScheduledJob) -> Result<ScheduledJob, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_add(job),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn schedule_list(self, _ctx: Context) -> Result<Vec<ScheduledJob>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_list(),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn schedule_pause(self, _ctx: Context, name: String) -> Result<ScheduledJob, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_pause(&name),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn schedule_resume(self, _ctx: Context, name: String) -> Result<ScheduledJob, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_resume(&name),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn schedule_remove(self, _ctx: Context, name: String) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.schedule_remove(&name),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // RPC-059 — /loop wiring.
    // ─────────────────────────────────────────────────────────────────

    async fn loop_add(
        self,
        _ctx: Context,
        session_id: SessionId,
        interval_seconds: u32,
        prompt: String,
    ) -> Result<RegisteredLoop, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.loop_add(&session_id, interval_seconds, prompt),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn loop_cancel(self, _ctx: Context, id: String) -> Result<bool, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.loop_cancel(&id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn loop_list(self, _ctx: Context, session_id: SessionId) -> Result<Vec<RegisteredLoop>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.loop_list(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    // BOARD-024: canonical full-record read. RPC-450: from the named
    // workspace, opened through the registry like list_workspace_work_units.
    async fn get_work_unit_details(
        self,
        _ctx: Context,
        workspace: String,
        id: String,
    ) -> Result<String, String> {
        let workspace = self
            .inner
            .workspace(Some(std::path::Path::new(&workspace)))
            .map_err(|error| format!("daemon workspace unavailable for work unit {id}: {error}"))?
            .cwd()
            .map(std::path::Path::to_path_buf);
        tokio::task::spawn_blocking(move || {
            crate::work_unit_details::load(workspace.as_deref(), &id)
        })
        .await
        .map_err(|error| format!("work unit details read task failed: {error}"))?
    }

    async fn session_card_changes(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<Vec<codelet_rpc_types::SessionCardChange>, String> {
        // WT-018: the session's card changes on its main checkout's board.
        let handle = self.inner.session_manager().ok_or(crate::NO_SESSION_MANAGER)?;
        handle.session_card_changes(&session_id)
    }

    async fn move_session_cards_back(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<Vec<codelet_rpc_types::CardMoveBack>, String> {
        // WT-018: the discard's "move back", run on the main checkout.
        let handle = self.inner.session_manager().ok_or(crate::NO_SESSION_MANAGER)?;
        handle.move_session_cards_back(&session_id)
    }

    async fn pause_human_action(
        self,
        _ctx: Context,
        session_id: SessionId,
        request_id: String,
        reply: codelet_rpc_types::HumanActionReply,
    ) -> Result<(), String> {
        let handle = self.inner.session_manager().ok_or(crate::NO_SESSION_MANAGER)?;
        handle.pause_human_action(&session_id, &request_id, reply)
    }

    async fn list_workspaces(self, _ctx: Context) -> Result<Vec<WorkspaceEntry>, String> {
        self.inner.workspace_entries().map_err(|error| error.to_string())
    }

    async fn list_workspace_work_units(
        self,
        _ctx: Context,
        workspace: String,
    ) -> Result<Vec<WorkUnitInfo>, String> {
        self.inner
            .workspace(Some(std::path::Path::new(&workspace)))
            .map(|scope| scope.work_units())
            .map_err(|error| error.to_string())
    }

    async fn create_session_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
        role: Option<String>,
    ) -> Result<SessionId, String> {
        let scope = self
            .inner
            .workspace(Some(std::path::Path::new(&workspace)))
            .map_err(|error| error.to_string())?;
        let project = scope.cwd().map(std::path::Path::to_path_buf);
        self.inner
            .session_manager()
            .ok_or("Session creation requires a session manager")?
            .create_session_in_workspace(
                project.as_deref().unwrap_or(std::path::Path::new(&workspace)),
                role,
            )
    }

    /// RPC-440: read the default model through the optional
    /// SessionManagerHandle; no handle means no default.
    async fn get_default_model(self, _ctx: Context) -> Option<String> {
        self.inner
            .session_manager()
            .and_then(|handle| handle.get_default_model())
    }

    /// RIG-016: route mode-carrying input through the SessionManagerHandle.
    async fn submit_input(
        self,
        _ctx: Context,
        session_id: SessionId,
        text: String,
        mode: codelet_rpc_types::InputMode,
    ) -> Result<(), String> {
        self.inner
            .session_manager()
            .ok_or("Input requires a session manager")?
            .submit_input(&session_id, text, mode)
    }
        }
    };
}
