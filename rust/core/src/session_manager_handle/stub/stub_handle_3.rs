macro_rules! compose_stub_handle_3 {
    ($($collected:tt)*) => {
        compose_stub_handle_4! {
            $($collected)*
    fn send_hitl_response(
        &self,
        session_id: &SessionId,
        request_id: &str,
        _response: HitlResponse,
    ) -> Result<(), String> {
        let mut guard = stub_lock(&self.hitl_request, "hitl_request");
        match guard.get(session_id) {
            Some(pending) if pending.route.request_id == request_id => {
                guard.remove(session_id);
                Ok(())
            }
            _ => Err("Stale or already answered interaction".to_string()),
        }
    }
    fn get_pause_state(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<PendingRequest<PauseState>>, String> {
        let guard = stub_lock(&self.pause_state, "pause_state");
        Ok(guard.get(session_id).cloned())
    }
    fn get_hitl_request(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<PendingRequest<HitlRequest>>, String> {
        let guard = stub_lock(&self.hitl_request, "hitl_request");
        Ok(guard.get(session_id).cloned())
    }
    fn send_fspec_result(
        &self,
        _session_id: &SessionId,
        _result: FspecResult,
    ) -> Result<(), String> {
        Ok(())
    }
    fn create_isolated_session(&self, role: Option<String>) -> Result<IsolatedSessionInfo, String> {
        let seq = self.next_iso_id.fetch_add(1, Ordering::SeqCst);
        let id = SessionId::new(format!("stub-iso-{seq}"));
        let worktree_path = format!("/tmp/stub-wt-{seq}");
        let record = SessionRecord {
            id: id.clone(),
            role,
            status: SessionStatus::Idle,
            is_isolated: true,
            worktree_path: Some(worktree_path.clone()),
        };
        {
            let mut sessions = stub_lock(&self.sessions, "sessions");
            sessions.push(record.clone());
        }
        // RPC-433: isolated sessions are sessions too — fire the
        // session-created event so remote clients learn about them.
        let _ = self.session_created_tx.send(Self::record_info(&record));
        Ok(IsolatedSessionInfo {
            session_id: id,
            worktree_path,
            base_commit: "abc1234".to_string(),
        })
    }
    fn status_changes_rx(&self) -> broadcast::Receiver<(SessionId, SessionStatus)> {
        self.status_changes_tx.subscribe()
    }
    fn status_changes_tx(&self) -> broadcast::Sender<(SessionId, SessionStatus)> {
        self.status_changes_tx.clone()
    }
    /// RPC-433: subscribe to the stub's session-created events.
    fn session_created_rx(&self) -> broadcast::Receiver<SessionInfo> {
        self.session_created_tx.subscribe()
    }
    /// RPC-433: subscribe to the stub's session-removed events.
    fn session_removed_rx(&self) -> broadcast::Receiver<SessionId> {
        self.session_removed_tx.subscribe()
    }
    /// RPC-436: subscribe to the stub's session-cleared events.
    fn session_cleared_rx(&self) -> broadcast::Receiver<SessionId> {
        self.session_cleared_tx.subscribe()
    }
    fn destroy_session(&self, session_id: &SessionId) -> Result<(), String> {
        let existed = {
            let mut sessions = stub_lock(&self.sessions, "sessions");
            let before = sessions.len();
            sessions.retain(|r| r.id != *session_id);
            sessions.len() != before
        };
        // RPC-433: fire the session-removed event ONLY when the session
        // actually existed (idempotent destroy of an unknown id emits
        // nothing — mirrors the real `SessionManager::destroy_session`).
        if existed {
            let _ = self.session_removed_tx.send(session_id.clone());
        }
        // Clean up keyed state.
        {
            let mut g = stub_lock(&self.tokens, "tokens");
            g.remove(session_id);
        }
        {
            let mut g = stub_lock(&self.models, "models");
            g.remove(session_id);
        }
        {
            let mut g = stub_lock(&self.work_unit_ctx, "work_unit_ctx");
            g.remove(session_id);
        }
        {
            let mut g = stub_lock(&self.pending_input, "pending_input");
            g.remove(session_id);
        }
        {
            let mut g = stub_lock(&self.debug_enabled, "debug_enabled");
            g.remove(session_id);
        }
        {
            let mut g = stub_lock(&self.pause_state, "pause_state");
            g.remove(session_id);
        }
        {
            let mut g = stub_lock(&self.hitl_request, "hitl_request");
            g.remove(session_id);
        }
        {
            let mut g = stub_lock(&self.active_session, "active_session");
            if g.as_ref() == Some(session_id) {
                *g = None;
            }
        }
        Ok(())
    }
    // ========================================================================
    // RPC-054: Provider credentials surface — deterministic in-memory
    // implementation used by cross-transport parity tests. The
    // `provider_credentials` map persists across get/set/delete so the
    // embedded + WebSocket transports observe the same state.
    // ========================================================================

    fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialInfo>, String> {
        Ok({
        self.list_provider_credentials_calls
            .fetch_add(1, Ordering::SeqCst);
        let guard = stub_lock(&self.provider_credentials, "provider_credentials");
        let mut out: Vec<ProviderCredentialInfo> = guard.values().cloned().collect();
        out.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
        out
    })
    }
    fn get_provider_credential(&self, provider_id: &str) -> Result<Option<ProviderCredentialInfo>, String> {
        Ok({
        {
            let guard = stub_lock(&self.provider_credentials, "provider_credentials");
            guard.get(provider_id).cloned()
        }
    })
    }
    fn set_provider_credentials(
        &self,
        provider_id: &str,
        creds: ProviderCredentialInput,
    ) -> Result<(), String> {
        self.set_provider_credentials_calls
            .fetch_add(1, Ordering::SeqCst);
        // Light validation: each kind requires its primary field.
        match creds.kind.as_str() {
            "api_key" => {
                if creds.api_key.as_deref().unwrap_or("").is_empty() {
                    return Err("api_key input requires a non-empty api_key".to_string());
                }
            }
            "oauth" => {
                if creds.oauth_token.as_deref().unwrap_or("").is_empty() {
                    return Err("oauth input requires a non-empty oauth_token".to_string());
                }
            }
            "custom" => {
                if creds.custom_endpoint.as_deref().unwrap_or("").is_empty() {
                    return Err("custom input requires a non-empty custom_endpoint".to_string());
                }
            }
            other => return Err(format!("unknown credential kind: {other}")),
        }
        {
            let mut guard = stub_lock(&self.provider_credentials, "provider_credentials");
            let existing = guard
                .get(provider_id)
                .cloned()
                .unwrap_or(ProviderCredentialInfo {
                    provider_id: provider_id.to_string(),
                    display_name: provider_id.to_string(),
                    configured: false,
                    credential_type: creds.kind.clone(),
                    model_count: 0,
                    masked_key: None,
                    source: None,
                });
            guard.insert(
                provider_id.to_string(),
                ProviderCredentialInfo {
                    configured: true,
                    credential_type: creds.kind,
                    ..existing
                },
            );
        }
        Ok(())
    }
    fn delete_provider_credentials(&self, provider_id: &str) -> Result<(), String> {
        self.delete_provider_credentials_calls
            .fetch_add(1, Ordering::SeqCst);
        {
            let mut guard = stub_lock(&self.provider_credentials, "provider_credentials");
            if let Some(existing) = guard.get(provider_id).cloned() {
                guard.insert(
                    provider_id.to_string(),
                    ProviderCredentialInfo {
                        configured: false,
                        model_count: 0,
                        ..existing
                    },
                );
            }
        }
        Ok(())
    }
    fn test_provider_connection(&self, _provider_id: &str) -> Result<TestConnectionResult, String> {
        self.test_provider_connection_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(TestConnectionResult {
            success: true,
            error: None,
            latency_ms: 7,
        })
    }
    fn refresh_models_cache(
        &self,
        provider_id: &str,
    ) -> Result<Vec<codelet_rpc_types::ModelEntry>, String> {
        self.refresh_models_cache_calls
            .fetch_add(1, Ordering::SeqCst);
        // Bump the model_count on the matching row so downstream
        // assertions can observe the refresh landing on the stub.
        {
            let mut guard = stub_lock(&self.provider_credentials, "provider_credentials");
            if let Some(existing) = guard.get(provider_id).cloned() {
                guard.insert(
                    provider_id.to_string(),
                    ProviderCredentialInfo {
                        model_count: existing.model_count.saturating_add(1),
                        ..existing
                    },
                );
            }
        }
        Ok(Vec::new())
    }
    fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>, String> {
        Ok({
        self.blocklist_list_calls.fetch_add(1, Ordering::SeqCst);
        {
            let g = stub_lock(&self.blocklist_rules, "blocklist_rules");
            g.clone()
        }
    })
    }
    // ========================================================================
    // RPC-057: Merge/worktree surface — deterministic stub overrides used
    // by cross-transport parity tests. Each method increments a per-call
    // counter and returns the seeded value (or a safe default).
    // ========================================================================
        }
    };
}
