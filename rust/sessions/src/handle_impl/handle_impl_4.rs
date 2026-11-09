macro_rules! compose_handle_impl_4 {
    ($($collected:tt)*) => {
        compose_handle_impl_5! {
            $($collected)*
    fn send_hitl_response(
        &self,
        session_id: &SessionId,
        request_id: &str,
        response: HitlResponse,
    ) -> Result<(), String> {
        // RPC-410: direct pass-through mapping — the wire payload
        // {cancelled, answers} is authoritative. AMGR-020: the reply names
        // the request id and is the user's.
        self.get_session(&session_id.value)?.answer_question(
            request_id,
            &codelet_rpc_types::RequestHolder::User,
            crate::hitl_mapping::wire_response_to_internal(response.cancelled, response.answers),
        )
    }
    fn get_pause_state(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<codelet_rpc_types::PendingRequest<PauseState>>, String> {
        Ok(self
            .get_session(&session_id.value)?
            .pending_pause()
            .map(|(route, state)| codelet_rpc_types::PendingRequest {
                route,
                payload: pause_state_to_rpc(state),
            }))
    }
    fn get_hitl_request(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<codelet_rpc_types::PendingRequest<HitlRequest>>, String> {
        // RPC-410: full pass-through — every question surfaces on the
        // wire in order (options: None → empty vec).
        Ok(self
            .get_session(&session_id.value)?
            .pending_question()
            .map(|(route, request)| codelet_rpc_types::PendingRequest {
                route,
                payload: crate::hitl_mapping::internal_request_to_wire(request),
            }))
    }
    /// TOOL-022 P2: snapshot of the active exec-stdin request, if any.
    /// Pure pass-through mapping (tools-internal → wire; identical
    /// fields, see `exec_stdin_mapping`).
    fn get_exec_stdin_request(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<codelet_rpc_types::ExecStdinRequest>, String> {
        // BUG-445: the lookup and a broken state lock (BUG-369) are errors.
        Ok(self
            .get_session(&session_id.value)?
            .get_exec_stdin_request()?
            .map(crate::exec_stdin_mapping::internal_request_to_wire))
    }
    /// TOOL-022 P2: write typed text to a live exec session's stdin.
    ///
    /// Bridges sync→async via `block_in_place` + `Handle::current().block_on`
    /// (the `loop_block_on` pattern) because `get_stdin_tx` / `send` are
    /// async store ops. MUST be invoked from a multi-thread tokio runtime.
    /// Unknown agent session → "Session not found"; unknown/exited exec
    /// session → a clean error naming the exec session id (no -1
    /// reaper-race noise).
    fn write_exec_stdin(
        &self,
        session_id: &SessionId,
        exec_session_id: &str,
        text: &str,
    ) -> Result<(), String> {
        match self.get_session(&session_id.value) {
            Ok(session) => {
                let store = codelet_tools::unified_exec::global_store();
                let exec_session_id_owned = exec_session_id.to_string();
                let payload = {
                    let mut p = text.to_string();
                    if !p.ends_with('\n') {
                        p.push('\n');
                    }
                    p
                };
                let result = loop_block_on(async move {
                    let tx = store
                        .get_stdin_tx(&exec_session_id_owned)
                        .await
                        .ok_or_else(|| {
                            format!("Unknown or exited exec session: {exec_session_id_owned}")
                        })?;
                    // BUG-362: success only once the bytes reached the command.
                    codelet_tools::unified_exec::write_stdin(
                        &tx,
                        &exec_session_id_owned,
                        payload.into_bytes(),
                    )
                    .await
                });
                // BUG-171: a successful submit answers the prompt — clear
                // the stored request so the push contract emits
                // ExecStdinRequestCleared and the TUI unmounts the
                // overlay (set_exec_stdin_request is the sole emission
                // point; Some→None → cleared chunk).
                result?;
                session.set_exec_stdin_request(None)
            }
            Err(error) => Err(error),
        }
    }
    fn send_fspec_result(&self, session_id: &SessionId, result: FspecResult) -> Result<(), String> {
        match self.get_session(&session_id.value) {
            Ok(session) => {
                session.send_fspec_result(result);
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
    /// Create an isolated (worktree-backed) session.
    ///
    /// **Runtime requirement (RPC-070):** bridges sync→async via
    /// `tokio::task::block_in_place(|| Handle::current().block_on(...))`.
    /// MUST be invoked from a multi-thread tokio runtime.
    fn create_isolated_session(&self, role: Option<String>) -> Result<IsolatedSessionInfo, String> {
        let project = self.default_project()?;
        crate::handle_creation::create_isolated_session_in_project(self, &project, role)
    }
    /// BUG-427: an isolated session cut from the named workspace.
    fn create_isolated_session_in_workspace(
        &self,
        workspace: &std::path::Path,
        role: Option<String>,
    ) -> Result<IsolatedSessionInfo, String> {
        let project = std::fs::canonicalize(workspace)
            .ok()
            .filter(|path| path.is_dir())
            .ok_or_else(|| format!("workspace path is not a directory: {}", workspace.display()))?;
        crate::handle_creation::create_isolated_session_in_project(self, &project, role)
    }
    fn set_thinking_level_default(
        &self,
        session_id: &SessionId,
        level: ThinkingLevel,
    ) -> Result<(), String> {
        // TUI-002: mirror `set_default_model`'s entry/success logging so the
        // default-thinking write path is observable in the combined log.
        tracing::info!(
            level = %level,
            session_id = %session_id.value.as_str(),
            "set_thinking_level_default: saving the level for the session's model"
        );
        // MODEL-012: the level is saved for the session's current model only,
        // and only a level that model offers (the model default removes the
        // model's saved level). A failed save is reported, and the session's
        // level changes only once the level is saved.
        let session = self.get_session(&session_id.value)?;
        let target = session.thinking_target()?;
        crate::thinking_options::accepted(&target, &level)?;
        crate::default_thinking_level_persistence::save_default_thinking_level(
            target.model(),
            level.clone(),
        )?;
        session.set_base_thinking_level(level);
        Ok(())
    }
    fn destroy_session(&self, session_id: &SessionId) -> Result<(), String> {
        // BUG-402: the manager's own reason, never relabelled.
        SessionManager::destroy_session(self, &session_id.value)
    }
    fn get_model_info(
        &self,
        session_id: &SessionId,
    ) -> Result<codelet_rpc_types::ModelInfo, String> {
        // TUI-001: build the models.dev registry once so `resolve_model_info`
        // can promote the raw slug to the friendly catalog name + capability
        // flags. `None` outside a multi-thread runtime / on cache failure →
        // graceful fallback to the raw slug (mirrors the TS fallback path).
        let registry = build_cloud_registry();
        let s = self.get_session(&session_id.value)?;
        // BUG-464: a poisoned lock or an unset id is reported, not "".
        let (provider_id, model_id) = s.selected_model_ids()?;
        Ok(crate::cloud_models::resolve_model_info(
            registry.as_ref(),
            &provider_id,
            &model_id,
            s.model_limits().context_window,
            s.model_limits().compaction_threshold,
        ))
    }
    fn get_thinking_level(&self, session_id: &SessionId) -> Result<ThinkingLevel, String> {
        // BUG-464: an unknown session is the lookup error, not Off.
        // MODEL-011: the level exactly as chosen.
        Ok(self.get_session(&session_id.value)?.get_base_thinking_level())
    }
    fn get_thinking_options(
        &self,
        session_id: &SessionId,
    ) -> Result<codelet_rpc_types::ThinkingOptions, String> {
        // MODEL-011: what the session's current model offers.
        let target = self.get_session(&session_id.value)?.thinking_target()?;
        Ok(crate::thinking_options::options_for(&target))
    }
        }
    };
}
