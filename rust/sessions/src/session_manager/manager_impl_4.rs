//! SessionManager inherent methods (part 4).

use super::*;

impl SessionManager {
    /// RPC-422: Create a BackgroundSession from an existing manifest WITHOUT
    /// persisting a blank manifest to disk.
    ///
    /// Used by `resume_session` to restore a session that was persisted on disk
    /// but is not currently in memory. This avoids the bug where
    /// `create_session_with_id` overwrites the manifest with 0 messages.
    pub async fn create_session_from_manifest(
        &self,
        manifest: &codelet_core::persistence::SessionManifest,
        model: &str,
    ) -> Result<(), String> {
        let uuid = manifest.id;

        tracing::info!(
            session_id = %uuid,
            model = %model,
            manifest_name = %manifest.name,
            manifest_message_count = manifest.messages.len(),
            "create_session_from_manifest: starting session creation from existing manifest"
        );

        {
            let sessions = self.sessions.read().expect("sessions lock poisoned");
            if sessions.len() >= MAX_SESSIONS {
                return Err(format!("Maximum sessions ({}) reached", MAX_SESSIONS));
            }
            if sessions.contains_key(&uuid) {
                drop(sessions);
                tracing::info!(
                    session_id = %uuid,
                    "create_session_from_manifest: session already exists in memory, reactivating"
                );
                self.set_active_session(uuid);
                return Ok(());
            }
        }

        // RPC-422: DO NOT save a blank manifest to disk.
        // The manifest already exists on disk with the correct message references.
        // We only need to create the in-memory BackgroundSession.

        // RPC-424: Parse model string using shared helper
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
        let project_path = manifest.project.clone();
        // BUG-411: a resolution failure (a missing credential is not one)
        // fails creation instead of creating a session without them.
        crate::credentials::resolve_and_set_env_var(
            registry_provider,
            Some(project_path.as_path()),
        )
        .map_err(|error| {
            format!("cannot resolve credentials for provider {registry_provider}: {error}")
        })?;

        let name = manifest.name.clone();
        let project = manifest.project.to_string_lossy().to_string();

        // RPC-425: Use shared session creation helper
        let parsed_model = ParsedModelInfo {
            model,
            registry_provider,
            is_profile_model,
            is_codex_model,
            is_custom_model,
        };

        // WT-007: re-read the codelet-git session manifest so a
        // formerly-isolated session comes back isolated. `resolve`
        // verifies the worktree (and its git admin dir) still exist and,
        // if not, marks the strayed manifest terminated (WT-005 prune).
        let resume_isolation = crate::resume_isolation::resolve(&uuid, project_path.as_path());
        let params = SessionCreationParams {
            uuid,
            name: &name,
            project: &project,
            project_path: project_path.as_path(),
            parsed_model,
            provider_id,
            model_id,
            worktree_path: resume_isolation.worktree_path.clone(),
            base_commit: resume_isolation.base_commit.clone(),
            isolation: resume_isolation.context.clone(),
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
        self.restore_activity_focus(&session, manifest)?;
        // BUG-601: the rules this session turned off stay off after resume.
        codelet_tools::blocklist::set_session_disabled_rules(
            uuid,
            manifest.blocklist_disabled_rules.clone(),
        );
        session.prepare_manifest_restore();
        let input_rx = result.input_rx;
        let mcp_injection_rx = result.mcp_injection_rx;

        // RPC-386: stamp the owning-manager back-reference BEFORE spawning the
        // agent loop, so the AgentManager handler the loop registers binds to
        // THIS manager.
        session.set_owning_manager(self.self_weak.clone());

        // RPC-040: agent_loop spawning is delegated to the hooks impl
        self.hooks()
            .spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx);

        // RPC-385: capture the SessionInfo BEFORE the insert
        let created_info = session.get_info();

        tracing::info!(
            session_id = %uuid,
            "create_session_from_manifest: inserting session into in-memory map"
        );
        self.sessions
            .write()
            .expect("sessions lock poisoned")
            .insert(uuid, session);

        // RPC-385: fire the session-created broadcast
        let _ = self.session_created_tx.send(created_info);

        self.set_default_model(model)?;
        self.set_active_session(uuid);
        self.maybe_start_scheduler(&project);

        // RPC-041: Emit IsolationStateChange directly on the manager-owned chunks_tx.
        // WT-007: a re-isolated resume mirrors the live isolated-create
        // path — IsolationStateChange(true, worktree, base); a
        // non-isolated resume keeps the historical (false, None) emission.
        let session_id_str = uuid.to_string();
        if let Some(resume_wt) = &resume_isolation.worktree_path {
            let _ = self.chunks_tx.send((
                codelet_rpc_types::SessionId::from(session_id_str.clone()),
                codelet_rpc_types::StreamChunk::isolation_state_change_with_base(
                    true,
                    Some(resume_wt.to_string_lossy().to_string()),
                    resume_isolation.base_commit.clone(),
                ),
            ));
        } else {
            let _ = self.chunks_tx.send((
                codelet_rpc_types::SessionId::from(session_id_str.clone()),
                codelet_rpc_types::StreamChunk::isolation_state_change(false, None),
            ));
        }

        // TUI-091: footer poller via the hooks. WT-007: a re-isolated
        // resume polls the worktree cwd (mirroring the live path).
        self.hooks().spawn_footer_poller(
            session_id_str,
            resume_isolation
                .worktree_path
                .clone()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| project.clone()),
            resume_isolation
                .worktree_path
                .clone()
                .map(|p| p.to_string_lossy().to_string()),
        );

        codelet_tools::broadcast_metadata_update();

        tracing::info!(
            session_id = %uuid,
            model = %model,
            project = %project,
            "create_session_from_manifest: session creation complete (manifest preserved on disk)"
        );

        Ok(())
    }
}
