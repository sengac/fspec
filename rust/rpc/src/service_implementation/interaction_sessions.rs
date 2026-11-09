macro_rules! compose_rpc_interaction_sessions {
    ($($collected:tt)*) => {
        compose_rpc_health_checkpoints! {
            $($collected)*
    /// AMGR-020: the pending requests' ids and payloads, read with the
    /// same get_pause_state / get_hitl_request every client uses.
    async fn get_interaction_snapshot(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<String, String> {
        let handle = self
            .inner
            .session_manager()
            .ok_or("Session manager unavailable")?;
        let pause = handle.get_pause_state(&session_id)?;
        let hitl = handle.get_hitl_request(&session_id)?;
        let entry = |route: Option<&codelet_rpc_types::RequestRoute>, state: serde_json::Value| {
            serde_json::json!({"request_id": route.map(|route| route.request_id.clone()), "state": state})
        };
        let snapshot = serde_json::json!({
            "pause": entry(pause.as_ref().map(|p| &p.route), serde_json::to_value(pause.as_ref().map(|p| &p.payload)).map_err(|e| e.to_string())?),
            "hitl": entry(hitl.as_ref().map(|h| &h.route), serde_json::to_value(hitl.as_ref().map(|h| &h.payload)).map_err(|e| e.to_string())?),
        });
        serde_json::to_string(&snapshot).map_err(|e| e.to_string())
    }
    /// AMGR-020: one of the checked replies, named by `method`.
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
        let handle = self
            .inner
            .session_manager()
            .ok_or("Session manager unavailable")?;
        let args: serde_json::Value =
            serde_json::from_str(&arguments).map_err(|_| "Invalid response arguments")?;
        match method.as_str() {
            "pause_human_action" => handle.pause_human_action(
                &session_id,
                &request_id,
                serde_json::from_value(args["reply"].clone()).map_err(|_| "Invalid reply")?,
            ),
            "pause_confirm" => handle.pause_confirm(
                &session_id,
                &request_id,
                args["accept"].as_bool().ok_or("Missing accept")?,
            ),
            "pause_triple" => handle.pause_triple(
                &session_id,
                &request_id,
                serde_json::from_value(args["choice"].clone()).map_err(|_| "Invalid choice")?,
            ),
            "send_hitl_response" => handle.send_hitl_response(
                &session_id,
                &request_id,
                serde_json::from_value(args["response"].clone()).map_err(|_| "Invalid response")?,
            ),
            _ => Err("Unknown interaction response".into()),
        }
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
    async fn list_sessions(self, _ctx: Context, project_path: String) -> Result<Vec<SessionInfo>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.list_sessions(&project_path),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn list_live_sessions(self, _ctx: Context) -> Result<Vec<SessionId>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.list_live_sessions(),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    /// BUG-399: a creation failure, or a daemon without a session manager,
    /// is an error — never a placeholder id.
    async fn create_session(
        self,
        _ctx: Context,
        role: Option<String>,
    ) -> Result<SessionId, String> {
        self.inner
            .session_manager()
            .ok_or("Session creation requires a session manager")?
            .create_session_checked(role)
    }

    /// BUG-425: wire-compatibility alias of `create_session`; one code path.
    async fn create_session_checked(
        self,
        ctx: Context,
        role: Option<String>,
    ) -> Result<SessionId, String> {
        FspecService::create_session(self, ctx, role).await
    }
    async fn session_manifest_issues(
        self,
        _ctx: Context,
    ) -> Result<Vec<codelet_rpc_types::SessionManifestIssue>, String> {
        codelet_core::persistence::session_manifest_issues()
    }

    async fn send_input(
        self,
        _ctx: Context,
        session_id: SessionId,
        text: String,
    ) -> Result<(), String> {
        self.inner
            .session_manager()
            .ok_or("send_input requires a session manager")?
            .send_input(&session_id, text)
    }

    async fn interrupt(self, _ctx: Context, session_id: SessionId) -> Result<(), String> {
        // BUG-465: no session manager is refused, not a silent no-op.
        match self.inner.session_manager() {
            Some(handle) => handle.interrupt(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn get_session_status(self, _ctx: Context, session_id: SessionId) -> Result<SessionStatus, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.get_session_status(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }
        }
    };
}
