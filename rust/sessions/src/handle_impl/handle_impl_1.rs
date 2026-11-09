macro_rules! compose_handle_impl_1 {
    ($($collected:tt)*) => {
        compose_handle_impl_2! {
            $($collected)*
    fn reconcile_work_units(&self, project: &std::path::Path) -> Result<(), String> {
        SessionManager::reconcile_work_units(self, project).map_err(|error| error.to_string())
    }
    fn list_sessions(&self, project_path: &str) -> Result<Vec<SessionInfo>, String> {
        SessionManager::list_sessions(self, project_path)
    }
    /// Create a new session, returning its freshly-minted `SessionId`.
    ///
    /// **Runtime requirement (RPC-070):** bridges sync→async via
    /// `tokio::task::block_in_place(|| Handle::current().block_on(...))`.
    /// MUST be invoked from a multi-thread tokio runtime — the nested
    /// `block_on` is only legal because `block_in_place` first detaches
    /// the worker from the scheduler.
    fn create_session_checked(&self, role: Option<String>) -> Result<SessionId, String> {
        crate::handle_creation::create_session_checked(self, role)
    }
    fn create_session_in_workspace(
        &self,
        workspace: &std::path::Path,
        role: Option<String>,
    ) -> Result<SessionId, String> {
        self.create_session_in_workspace_checked(workspace, role)
    }
    /// RPC-422: Resume a persisted session by first creating it in memory,
    /// then restoring messages and token state.
    ///
    /// This override ensures the BackgroundSession exists in the in-memory
    /// session map before `restore_session_messages` tries to look it up.
    fn resume_session(&self, session_id: &SessionId) -> Result<(), String> {
        let uuid = uuid::Uuid::parse_str(&session_id.value)
            .map_err(|e| format!("invalid session id: {e}"))?;

        tracing::info!(
            session_id = %uuid,
            "resume_session: starting session resume"
        );

        // Load the manifest from persistence
        let manifest = codelet_core::persistence::load_session(uuid)
            .map_err(|e| format!("Failed to load session manifest: {e}"))?;

        tracing::info!(
            session_id = %uuid,
            manifest_provider = %manifest.provider,
            manifest_name = %manifest.name,
            manifest_message_count = manifest.messages.len(),
            "resume_session: loaded manifest from disk"
        );

        // Check if the session already exists in memory
        let already_exists = self.get_session(&session_id.value).is_ok();

        tracing::info!(
            session_id = %uuid,
            already_exists,
            "resume_session: checking if session exists in memory"
        );

        // If not in memory, create it from the existing manifest WITHOUT overwriting it
        if !already_exists {
            let model = if manifest.provider.is_empty() {
                self.get_default_model()
                    .ok_or("No default model set and manifest has no provider")?
            } else {
                manifest.provider.clone()
            };
            tracing::info!(
                session_id = %uuid,
                model = %model,
                manifest_message_count = manifest.messages.len(),
                "resume_session: session not in memory, creating from existing manifest"
            );
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    SessionManager::create_session_from_manifest(self, &manifest, &model).await
                })
            })?;
            tracing::info!(
                session_id = %uuid,
                "resume_session: session created in memory from manifest"
            );
        }

        if !self
            .get_session(&session_id.value)?
            .requires_model_restore()
        {
            return Ok(());
        }

        // Now restore messages and token state
        tracing::info!(
            session_id = %uuid,
            "resume_session: restoring messages and token state"
        );
        self.get_session(&session_id.value)?
            .restore_archival_transcript()
            .map_err(|error| error.to_string())?;
        let envelopes = codelet_core::persistence::get_session_message_envelopes(uuid)?;
        tracing::info!(
            session_id = %uuid,
            envelope_count = envelopes.len(),
            "resume_session: loaded message envelopes"
        );
        let state = TokenRestoreState {
            current_context: manifest.token_usage.current_context_tokens as i64,
            cumulative_billed_output: manifest.token_usage.cumulative_billed_output as i64,
            cache_read: manifest.token_usage.cache_read_tokens as i64,
            cache_creation: manifest.token_usage.cache_creation_tokens as i64,
            cumulative_billed_input: manifest.token_usage.cumulative_billed_input as i64,
            cumulative_billed_output_second: manifest.token_usage.cumulative_billed_output as i64,
        };
        self.restore_session_messages(session_id, envelopes)?;
        self.restore_session_token_state(session_id, state)?;
        self.get_session(&session_id.value)?
            .complete_manifest_restore();

        tracing::info!(
            session_id = %uuid,
            "resume_session: session resume complete"
        );
        Ok(())
    }
    fn send_input(&self, session_id: &SessionId, text: String) -> Result<(), String> {
        self.live_session(session_id)?.send_input(text, None)
    }
    fn send_input_with_thinking(
        &self,
        session_id: &SessionId,
        text: String,
        thinking: Option<ThinkingConfig>,
    ) -> Result<(), String> {
        let session = self.live_session(session_id)?;
        let thinking_json = thinking
            .map(|t| serde_json::to_string(&t))
            .transpose()
            .map_err(|error| format!("thinking config is not serializable: {error}"))?;
        session.send_input(text, thinking_json)
    }
    fn interrupt(&self, session_id: &SessionId) -> Result<(), String> {
        // BUG-465: an unknown session is the lookup error, not a no-op.
        self.get_session(&session_id.value)?.interrupt();
        Ok(())
    }
    fn get_session_status(&self, session_id: &SessionId) -> Result<SessionStatus, String> {
        // BUG-445: an unknown session is the lookup error, not Idle.
        Ok(self.get_session(&session_id.value)?.get_status())
    }
    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        SessionManager::chunks_tx(self).subscribe()
    }
    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        SessionManager::logs_tx(self).subscribe()
    }
    fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)> {
        SessionManager::chunks_tx(self).clone()
    }
    fn logs_tx(&self) -> broadcast::Sender<LogRecord> {
        SessionManager::logs_tx(self).clone()
    }
    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        SessionManager::status_changes_tx(self).subscribe()
    }
    fn status_changes_tx(&self) -> broadcast::Sender<(SessionId, SessionStatus)> {
        SessionManager::status_changes_tx(self).clone()
    }
    fn session_created_rx(&self) -> broadcast::Receiver<SessionInfo> {
        SessionManager::session_created_tx(self).subscribe()
    }
    /// RPC-433: subscribe to the session-removed broadcast. The WS
    /// server's per-connection fan-out task drains this and wraps each
    /// id in `Envelope::SessionEvent(SessionEvent::Removed(_))`.
    fn session_removed_rx(&self) -> broadcast::Receiver<SessionId> {
        SessionManager::session_removed_tx(self).subscribe()
    }
    /// RPC-436: subscribe to the session-cleared broadcast. The WS
    /// server's per-connection fan-out task drains this and wraps each
    /// id in `Envelope::SessionEvent(SessionEvent::Cleared(_))`.
    fn session_cleared_rx(&self) -> broadcast::Receiver<SessionId> {
        SessionManager::session_cleared_tx(self).subscribe()
    }
    /// RPC-433: the in-memory live session map as a snapshot — the
    /// connect-time replay source for `Envelope::SessionSnapshot`
    /// (rule R3: no disk merge, no project filter).
    fn live_session_infos(&self) -> Vec<SessionInfo> {
        SessionManager::live_session_infos(self)
    }
    fn get_session_tokens(&self, session_id: &SessionId) -> Result<SessionTokens, String> {
        // BUG-445: an unknown session is the lookup error, not zeros.
        let (input, output, _reasoning) = self.get_session(&session_id.value)?.get_tokens();
        Ok(SessionTokens {
            input_tokens: input as i64,
            output_tokens: output as i64,
        })
    }
    fn get_session_model(&self, session_id: &SessionId) -> Result<SessionModel, String> {
        // BUG-445: an unknown session is the lookup error, not a blank model.
        let s = self.get_session(&session_id.value)?;
        // BUG-464: a poisoned lock or an unset id is reported, not "".
        let (provider_id, model_id) = s.selected_model_ids()?;
        let limits = s.model_limits();
        Ok(SessionModel {
            provider_id,
            model_id,
            context_window: limits.context_window.map(i64::from),
            max_output_tokens: limits.max_output_tokens.map(i64::from),
            compaction_threshold: limits.compaction_threshold.map(i64::from),
        })
    }
        }
    };
}
