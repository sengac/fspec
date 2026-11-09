macro_rules! compose_stub_handle_1 {
    ($($collected:tt)*) => {
        compose_stub_handle_2! {
            $($collected)*
    fn list_sessions(&self, _project_path: &str) -> Result<Vec<SessionInfo>, String> {
        Ok({
        let sessions = stub_lock(&self.sessions, "sessions");
        sessions.iter().map(Self::record_info).collect()
    })
    }
    /// RPC-433: the in-memory live session map as a snapshot — the
    /// connect-time replay source for `Envelope::SessionSnapshot`
    /// (in-memory only: no persisted-manifest merge, no project filter).
    fn live_session_infos(&self) -> Vec<SessionInfo> {
        let sessions = stub_lock(&self.sessions, "sessions");
        sessions.iter().map(Self::record_info).collect()
    }
    fn create_session_checked(&self, role: Option<String>) -> Result<SessionId, String> {
        let id = SessionId::new(format!(
            "stub-{}",
            self.next_id.fetch_add(1, Ordering::SeqCst)
        ));
        let record = SessionRecord {
            id: id.clone(),
            role,
            status: SessionStatus::Idle,
            is_isolated: false,
            worktree_path: None,
        };
        self.sessions
            .lock()
            .map_err(|error| format!("stub session map poisoned: {error}"))?
            .push(record.clone());
        // RPC-433: fire the session-created event (multi-client fan-out
        // tests drive Created through this sender; a lagged subscriber
        // recovers via RecvError::Lagged — the TUI append is idempotent).
        if self
            .session_created_tx
            .send(Self::record_info(&record))
            .is_err()
        {
            tracing::debug!(session_id = %id.value, "session created with no event subscribers");
        }
        Ok(id)
    }
    fn list_live_sessions(&self) -> Result<Vec<SessionId>, String> {
        Ok({
        stub_lock(&self.sessions, "sessions")
            .iter()
            .map(|r| r.id.clone())
            .collect()
    })
    }
    fn send_input(&self, session_id: &SessionId, _text: String) -> Result<(), String> {
        // BUG-328: like the real handle, input for an unknown session is
        // refused.
        let known = self
            .sessions
            .lock()
            .map_err(|_| "stub session registry poisoned".to_string())?
            .iter()
            .any(|record| &record.id == session_id);
        if !known {
            return Err(crate::session_manager_handle::session_not_live(session_id));
        }
        self.set_status(session_id, SessionStatus::Running);
        let _ = self
            .status_changes_tx
            .send((session_id.clone(), SessionStatus::Running));

        let chunks_tx = self.chunks_tx.clone();
        let status_tx = self.status_changes_tx.clone();
        let sid = session_id.clone();
        let sessions = Arc::clone(&self.sessions);
        let held = stub_lock(&self.turn_hold, "turn_hold").clone();
        tokio::spawn(async move {
            // Deterministic stub-provider sequence: [Text("hi back"), Done].
            let _ = chunks_tx.send((sid.clone(), StreamChunk::text("hi back".to_string())));
            // BUG-520: a held turn waits for its gate (released or dropped).
            if let Some(mut held) = held {
                let _ = held.wait_for(|released| *released).await;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            let _ = chunks_tx.send((sid.clone(), StreamChunk::done()));

            // Flip state back to Idle once the stream has completed.
            {
                let mut sessions = stub_lock(&sessions, "sessions");
                for record in sessions.iter_mut() {
                    if record.id == sid {
                        record.status = SessionStatus::Idle;
                        break;
                    }
                }
            }
            let _ = status_tx.send((sid, SessionStatus::Idle));
        });
        Ok(())
    }
    fn interrupt(&self, session_id: &SessionId) -> Result<(), String> {
        self.set_status(session_id, SessionStatus::Interrupted);
        let _ = self
            .chunks_tx
            .send((session_id.clone(), StreamChunk::interrupted(Vec::new())));
        Ok(())
    }
    fn get_session_status(&self, session_id: &SessionId) -> Result<SessionStatus, String> {
        Ok({
        let sessions = stub_lock(&self.sessions, "sessions");
        sessions
            .iter()
            .find(|r| r.id == *session_id)
            .map(|r| r.status)
            .unwrap_or(SessionStatus::Idle)
    })
    }
    fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
        self.chunks_tx.subscribe()
    }
    fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
        self.logs_tx.subscribe()
    }
    fn chunks_tx(&self) -> broadcast::Sender<(SessionId, StreamChunk)> {
        self.chunks_tx.clone()
    }
    fn logs_tx(&self) -> broadcast::Sender<LogRecord> {
        self.logs_tx.clone()
    }
    fn list_providers(&self) -> Result<Vec<ProviderInfo>, String> {
        Ok({
            let guard = stub_lock(&self.providers, "providers");
            guard.clone()
        })
    }
    // RPC-347: in-memory custom-model write overrides mirroring the
    // profile_sections append/replace/delete semantics. `provider_id` is
    // ignored (the stub does not enforce the openai-only guard — that lives
    // in the real persistence layer) so parity tests can drive any profile.
    fn add_custom_model(
        &self,
        _provider_id: &str,
        profile_name: &str,
        definition: &CustomModelDefinition,
    ) -> Result<(), String> {
        {
            let mut guard = stub_lock(&self.custom_models, "custom_models");
            guard
                .entry(profile_name.to_string())
                .or_default()
                .push(definition.clone());
        }
        Ok(())
    }
    fn update_custom_model(
        &self,
        _provider_id: &str,
        profile_name: &str,
        original_model_id: &str,
        definition: &CustomModelDefinition,
    ) -> Result<(), String> {
        {
            let mut guard = stub_lock(&self.custom_models, "custom_models");
            if let Some(entries) = guard.get_mut(profile_name) {
                if let Some(slot) = entries.iter_mut().find(|d| d.id == original_model_id) {
                    *slot = definition.clone();
                }
            }
        }
        Ok(())
    }
    fn delete_custom_model(
        &self,
        _provider_id: &str,
        profile_name: &str,
        model_id: &str,
    ) -> Result<(), String> {
        {
            let mut guard = stub_lock(&self.custom_models, "custom_models");
            if let Some(entries) = guard.get_mut(profile_name) {
                entries.retain(|d| d.id != model_id);
            }
        }
        Ok(())
    }
    // ========================================================================
    // RPC-037: deterministic stub overrides used by cross-transport parity
    // tests. Every override is small, synchronous, and idempotent so the
    // embedded and WebSocket paths produce byte-identical results.
    // ========================================================================

    fn send_input_with_thinking(
        &self,
        session_id: &SessionId,
        text: String,
        _thinking: Option<ThinkingConfig>,
    ) -> Result<(), String> {
        // Discard the thinking config (deterministic stub behaviour) and
        // delegate to the same chunk-emitting path as send_input.
        self.send_input(session_id, text)
    }
    fn get_session_tokens(&self, session_id: &SessionId) -> Result<SessionTokens, String> {
        Ok({
        {
            let guard = stub_lock(&self.tokens, "tokens");
            guard.get(session_id).cloned().unwrap_or(SessionTokens {
                input_tokens: 0,
                output_tokens: 0,
            })
        }
    })
    }
    fn get_session_model(&self, session_id: &SessionId) -> Result<SessionModel, String> {
        Ok({
        {
            let guard = stub_lock(&self.models, "models");
            guard.get(session_id).cloned().unwrap_or(SessionModel {
                provider_id: String::new(),
                model_id: String::new(),
                context_window: None,
                max_output_tokens: None,
                compaction_threshold: None,
            })
        }
    })
    }
    fn get_compaction_progress(&self, _session_id: &SessionId) -> Result<Option<CompactionProgress>, String> {
        Ok({
        None
    })
    }
    fn get_buffered_output(&self, _session_id: &SessionId, _limit: u32) -> Result<Vec<StreamChunk>, String> {
        Ok({
        Vec::new()
    })
    }
    fn clear_history(&self, session_id: &SessionId) -> Result<(), String> {
        // RPC-074 (TS parity): mirror `BackgroundSession::clear_history` —
        // emit a `SessionStateChange { state: Cleared }` chunk so any
        // subscriber (TUI, cross-transport parity tests) drives its UI
        // reset off the same chunk on both the stub and the real impl.
        // The previous `UserNotification { message: ... }` broadcast
        // for /clear was a Rust-side invention with no counterpart in the
        // TypeScript reference (`src/tui/components/AgentView.tsx:1554-1564`,
        // handleClearCommand → TUI-066 contract) and has been removed.
        let _ = self.chunks_tx.send((
            session_id.clone(),
            StreamChunk::session_state_change(SessionState::Cleared),
        ));
        // RPC-436: fire the session-cleared event ONLY when the session
        // actually exists (mirrors the `destroy_session` liveness gate) —
        // so multi-client fan-out tests observe exactly one Cleared event
        // per live clear, and an unknown-id clear emits nothing.
        let existed = stub_lock(&self.sessions, "sessions")
            .iter()
            .any(|r| &r.id == session_id);
        if existed {
            let _ = self.session_cleared_tx.send(session_id.clone());
        }
        Ok(())
    }
    fn compact_session(&self, session_id: &SessionId) -> Result<CompactionResult, String> {
        let canned = CompactionResult {
            original_tokens: 1000,
            compacted_tokens: 500,
            // RPC-420: percent of tokens removed [0,100] — 1000 → 500
            // is a 50% reduction, matching the real producer convention
            // (`compression_ratio(orig, compacted) * 100.0`).
            compression_ratio: 50.0,
            turns_summarized: 4,
            turns_kept: 2,
        };
        let _ = self.chunks_tx.send((
            session_id.clone(),
            StreamChunk::compaction_complete(canned.clone()),
        ));
        Ok(canned)
    }
        }
    };
}
