//! SessionManager inherent methods (part 3).

use super::*;

impl SessionManager {
    /// Navigation: previous session
    pub fn get_prev_session(&self) -> Option<String> {
        let sessions = self.sessions.read().expect("sessions lock poisoned");
        let active = self
            .active_session_id
            .read()
            .expect("active_session lock poisoned");
        let nav_list = crate::navigation::build_navigation_list(&sessions, &self.chain_of_command);
        match crate::navigation::get_prev_target(&nav_list, *active) {
            crate::navigation::NavigationTarget::Session(id) => Some(id.to_string()),
            _ => None,
        }
    }
    /// Navigation: first session
    pub fn get_first_session(&self) -> Option<String> {
        let sessions = self.sessions.read().expect("sessions lock poisoned");
        let nav_list = crate::navigation::build_navigation_list(&sessions, &self.chain_of_command);
        nav_list.first().map(|id| id.to_string())
    }
    /// Get a session by ID
    pub fn get_session(&self, id: &str) -> Result<Arc<BackgroundSession>, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| format!("Invalid session ID {id}: {e}"))?;
        self.sessions
            .read()
            .expect("sessions lock poisoned")
            .get(&uuid)
            .cloned()
            .ok_or_else(|| format!("Session not found: {}", id))
    }
    /// Create a background session with a specific ID (for persistence integration).
    pub async fn create_session_with_id(
        &self,
        id: &str,
        model: &str,
        project: &str,
        name: &str,
    ) -> Result<(), String> {
        let uuid = Uuid::parse_str(id).map_err(|e| format!("Invalid session ID {id}: {e}"))?;

        tracing::info!(
            session_id = %uuid,
            model = %model,
            project = %project,
            name = %name,
            "create_session_with_id: starting session creation"
        );

        {
            let sessions = self.sessions.read().expect("sessions lock poisoned");
            tracing::debug!(
                session_count = sessions.len(),
                max_sessions = MAX_SESSIONS,
                "create_session_with_id: checking session limits"
            );
            if sessions.len() >= MAX_SESSIONS {
                return Err(format!("Maximum sessions ({}) reached", MAX_SESSIONS));
            }
            if sessions.contains_key(&uuid) {
                drop(sessions);
                tracing::info!(
                    session_id = %uuid,
                    "create_session_with_id: session already exists in memory, reactivating"
                );
                self.set_active_session(uuid);
                return Ok(());
            }
        }

        // RPC-424: Parse model string using shared helper. Done BEFORE
        // manifest persistence so a malformed model string does not leave
        // an orphaned manifest on disk with no in-memory session.
        let parsed = crate::model_parsing::parse_model_string(model)?;
        let registry_provider = parsed.registry_provider;
        let model_part = parsed.model_part;
        let is_profile_model = parsed.is_profile_model;
        let is_codex_model = parsed.is_codex_model;
        let is_custom_model = parsed.is_custom_model;

        let (provider_id, model_id) = (
            Some(registry_provider.to_string()),
            Some(model_part.to_string()),
        );

        // Resolve credentials internally using the lifted credentials module.
        let project_path = std::path::PathBuf::from(project);
        // BUG-411: a resolution failure (a missing credential is not one)
        // fails creation instead of creating a session without them.
        crate::credentials::resolve_and_set_env_var(
            registry_provider,
            Some(project_path.as_path()),
        )
        .map_err(|error| {
            format!("cannot resolve credentials for provider {registry_provider}: {error}")
        })?;

        // RPC-425: Use shared session creation helper
        let parsed_model = ParsedModelInfo {
            model,
            registry_provider,
            is_profile_model,
            is_codex_model,
            is_custom_model,
        };
        let params = SessionCreationParams {
            uuid,
            name,
            project,
            project_path: project_path.as_path(),
            parsed_model,
            provider_id,
            model_id,
            worktree_path: None,
            base_commit: None,
            isolation: None,
            chunks_tx: self.chunks_tx.clone(),
            status_changes_tx: self.status_changes_tx.clone(),
            session_cleared_tx: self.session_cleared_tx.clone(),
        };

        // WT-012: shared provider-manager funnel (registry selection vs
        // profile/codex construction) — single source of truth in
        // model_resolution::resolve_provider_manager.
        let provider_manager = crate::model_resolution::resolve_provider_manager(model).await?;

        let result = create_background_session_inner(params, provider_manager).await?;
        let session = result.session;
        let input_rx = result.input_rx;
        let mcp_injection_rx = result.mcp_injection_rx;

        // Persist the manifest ONLY after the session is fully constructed
        // in memory. This prevents orphaned manifests on disk when provider
        // resolution or session construction fails (the Linux bug where
        // `create_background_session_inner` returned Err after the manifest
        // was already written, leaving a disk-only session with no in-memory
        // counterpart and an empty SessionId returned to the caller).
        let project_path = std::path::PathBuf::from(project);
        let mut manifest = codelet_core::persistence::SessionManifest::with_provider(
            name,
            project_path.clone(),
            model,
        );
        manifest.id = uuid;
        tracing::info!(
            session_id = %uuid,
            manifest_provider = %manifest.provider,
            manifest_name = %manifest.name,
            manifest_project = %manifest.project.display(),
            "create_session_with_id: constructed manifest, about to persist to disk"
        );
        codelet_core::persistence::save_session(&manifest)
            .map_err(|e| format!("Failed to persist session manifest: {}", e))?;
        tracing::info!(
            session_id = %uuid,
            "create_session_with_id: manifest persisted to disk successfully"
        );

        // BUG-154: stamp the owning-manager back-reference BEFORE spawning the
        // agent loop, so the AgentManager handler the loop registers binds to
        // THIS manager instead of the global singleton. Mirrors the call in
        // create_session_from_manifest (line 840) and
        // create_isolated_session_with_id (line 1050).
        session.set_owning_manager(self.self_weak.clone());

        // RPC-040: agent_loop spawning is delegated to the hooks impl so
        // codelet-sessions has no transitive napi dependency.
        self.hooks()
            .spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx);

        // RPC-385: capture the SessionInfo BEFORE the insert moves `session`
        // into the map, so the session-created broadcast can carry it.
        let created_info = session.get_info();

        tracing::info!(
            session_id = %uuid,
            "create_session_with_id: inserting session into in-memory map"
        );
        self.sessions
            .write()
            .expect("sessions lock poisoned")
            .insert(uuid, session);

        // RPC-385: fire the session-created broadcast next to the existing
        // metadata-update fan-out so the embedded TUI can append a tab for
        // sessions it did not itself initiate (spawned subordinates). A send
        // error (no subscribers) is benign and ignored.
        let _ = self.session_created_tx.send(created_info);
        tracing::debug!(
            session_id = %uuid,
            "create_session_with_id: session-created broadcast sent"
        );

        self.set_default_model(model)?;
        self.set_active_session(uuid);
        self.maybe_start_scheduler(project);

        // RPC-041: Emit IsolationStateChange directly on the manager-owned
        // chunks_tx (the previous `hooks().emit_isolation_state_change(...)`
        // delegation is gone — the hook has been removed from the trait).
        let _ = self.chunks_tx.send((
            codelet_rpc_types::SessionId::from(id.to_string()),
            codelet_rpc_types::StreamChunk::isolation_state_change(false, None),
        ));

        // TUI-091: footer poller via the hooks.
        self.hooks()
            .spawn_footer_poller(id.to_string(), project.to_string(), None);

        codelet_tools::broadcast_metadata_update();

        tracing::info!(
            session_id = %uuid,
            model = %model,
            project = %project,
            name = %name,
            "create_session_with_id: session creation complete"
        );

        Ok(())
    }
}
