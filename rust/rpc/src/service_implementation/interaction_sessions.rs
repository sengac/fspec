macro_rules! compose_rpc_interaction_sessions {
    ($($collected:tt)*) => {
        compose_rpc_health_checkpoints! {
            $($collected)*
    async fn get_interaction_snapshot(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<String, String> {
        self.inner
            .session_manager()
            .ok_or("Session manager unavailable")?
            .interaction_snapshot(&session_id)
    }
    async fn reply_interaction_checked(
        self,
        _ctx: Context,
        session_id: SessionId,
        method: String,
        request_id: String,
        arguments: String,
    ) -> Result<(), String> {
        if arguments.len() > 65536 {
            return Err("Approval response exceeds input limit".into());
        }
        self.inner
            .session_manager()
            .ok_or("Session manager unavailable")?
            .reply_interaction_checked(&session_id, &method, &request_id, &arguments)
    }

    async fn read_work_units_bounded(
        self,
        _ctx: Context,
        detail: bool,
        arguments: String,
    ) -> Result<String, String> {
        if arguments.len() > 4096 {
            return Err("Arguments exceed limit".into());
        }
        let args: serde_json::Value =
            serde_json::from_str(&arguments).map_err(|_| "Invalid arguments")?;
        let args = args.as_object().ok_or("Arguments must be an object")?;
        // Validate before taking a snapshot. The watcher still owns a full local snapshot.
        let page = if detail {
            None
        } else {
            Some(work_unit_reads::Page::parse(Some(args))?)
        };
        let chunk = if detail {
            Some(work_unit_reads::Detail::parse(Some(args))?)
        } else {
            None
        };
        let snapshot = serde_json::to_value(self.inner.list_work_units_snapshot())
            .map_err(|_| "Snapshot failed")?;
        let mut result = match (page, chunk) {
            (Some(page), _) => page.apply(snapshot)?,
            (_, Some(chunk)) => chunk.apply(snapshot)?,
            _ => return Err("Invalid read".into()),
        };
        if result.is_null() {
            return Err("Invalid workspace IDs".into());
        }
        result["backend_fetch"] = serde_json::json!("bounded_rpc");
        let text = serde_json::to_string(&result).map_err(|_| "Serialization failed")?;
        if text.len() > 256 * 1024 {
            return Err("Response exceeds limit".into());
        }
        Ok(text)
    }

    async fn list_work_units(self, _ctx: Context) -> Vec<WorkUnitInfo> {
        self.inner.list_work_units_snapshot()
    }

    async fn list_sessions(self, _ctx: Context, project_path: String) -> Vec<SessionInfo> {
        match self.inner.session_manager() {
            Some(handle) => handle.list_sessions(&project_path),
            None => Vec::new(),
        }
    }

    async fn list_live_sessions(self, _ctx: Context) -> Vec<SessionId> {
        match self.inner.session_manager() {
            Some(handle) => handle.list_live_sessions(),
            None => Vec::new(),
        }
    }

    async fn create_session(self, _ctx: Context, role: Option<String>) -> SessionId {
        match self.inner.session_manager() {
            Some(handle) => handle.create_session(role),
            None => SessionId::new("rpc-no-session-manager"),
        }
    }

    async fn create_session_checked(
        self,
        _ctx: Context,
        role: Option<String>,
    ) -> Result<SessionId, String> {
        self.inner
            .session_manager()
            .ok_or("Session creation requires a session manager")?
            .create_session_checked(role)
    }

    async fn session_manifest_issues(
        self,
        _ctx: Context,
    ) -> Result<Vec<codelet_rpc_types::SessionManifestIssue>, String> {
        codelet_core::persistence::session_manifest_issues()
    }

    async fn send_input(self, _ctx: Context, session_id: SessionId, text: String) {
        if let Some(handle) = self.inner.session_manager() {
            handle.send_input(&session_id, text);
        }
    }

    async fn interrupt(self, _ctx: Context, session_id: SessionId) {
        if let Some(handle) = self.inner.session_manager() {
            handle.interrupt(&session_id);
        }
    }

    async fn get_session_status(self, _ctx: Context, session_id: SessionId) -> SessionStatus {
        match self.inner.session_manager() {
            Some(handle) => handle.get_session_status(&session_id),
            None => SessionStatus::Idle,
        }
    }
        }
    };
}
