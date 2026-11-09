//! SessionManager inherent methods (part 5).

use super::*;

impl SessionManager {
    /// GIT-028: Create an isolated session with a git worktree.
    pub async fn create_isolated_session_with_id(
        &self,
        id: &str,
        model: &str,
        project: &str,
        name: &str,
    ) -> Result<codelet_rpc_types::IsolatedSessionInfo, String> {
        let uuid = Uuid::parse_str(id).map_err(|e| format!("Invalid session ID: {}", e))?;

        {
            let sessions = self.sessions.read().expect("sessions lock poisoned");
            if sessions.len() >= MAX_SESSIONS {
                return Err(format!("Maximum sessions ({}) reached", MAX_SESSIONS));
            }
            if sessions.contains_key(&uuid) {
                return Err(format!("Session {} already exists", id));
            }
        }

        let worktree_result = codelet_git::create_worktree(project, id)
            .map_err(|e| format!("Failed to create worktree: {}", e))?;

        let worktree_path = worktree_result.info.path.clone();
        let base_commit = worktree_result.base_commit.clone();

        codelet_git::create_session_manifest(
            id,
            project,
            Some(worktree_path.clone()),
            Some(base_commit.clone()),
        )
        .map_err(|e| format!("Failed to create session manifest: {}", e))?;

        // Isolation-specific env load (kept out of the shared helper — the two
        // non-isolated paths do not run it): loads the process-CWD `.env` so
        // provider credentials are visible to credential resolution below.
        let _ = dotenvy::dotenv();

        // RPC-424: Parse model string using shared helper (the provider-manager
        // funnel, WT-012, re-parses internally for its branch selection).
        let parsed = crate::model_parsing::parse_model_string(model)?;
        let registry_provider = parsed.registry_provider;
        let model_part = parsed.model_part;

        let (provider_id, model_id) = (
            Some(registry_provider.to_string()),
            Some(model_part.to_string()),
        );

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

        // WT-001: the isolation context the shared helper injects into the
        // environment reminder (worktree path relative to the project root,
        // base commit the worktree was forked from).
        let isolation = codelet_cli::session::context_gathering::IsolationContext {
            is_isolated: true,
            worktree_path: Some(
                worktree_path
                    .strip_prefix(&project_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| worktree_path.to_string_lossy().to_string()),
            ),
            base_commit: Some(base_commit.clone()),
        };

        // WT-012: the shared bootstrap (model selection, vision/max-images
        // seeding, Session construction + PROV-143 preserve-thinking seed,
        // context-reminder injection, lifecycle-hook load, BackgroundSession
        // construction, TUI-002 thinking-level seed, PROV-142 auto-continue
        // seed, model limits, pre-tool hook registration, MCP init) runs in
        // the RPC-425 helper — the isolated path no longer re-implements it.
        let parsed_model = ParsedModelInfo {
            model,
            registry_provider,
            is_profile_model: parsed.is_profile_model,
            is_codex_model: parsed.is_codex_model,
            is_custom_model: parsed.is_custom_model,
        };
        let params = SessionCreationParams {
            uuid,
            name,
            project,
            project_path: project_path.as_path(),
            parsed_model,
            provider_id,
            model_id,
            worktree_path: Some(worktree_path.clone()),
            base_commit: Some(base_commit.clone()),
            isolation: Some(isolation),
            chunks_tx: self.chunks_tx.clone(),
            status_changes_tx: self.status_changes_tx.clone(),
            session_cleared_tx: self.session_cleared_tx.clone(),
        };

        // WT-012: shared provider-manager funnel — single source of truth in
        // model_resolution::resolve_provider_manager.
        let provider_manager = crate::model_resolution::resolve_provider_manager(model).await?;

        let result = create_background_session_inner(params, provider_manager).await?;
        let session = result.session;
        let input_rx = result.input_rx;
        let mcp_injection_rx = result.mcp_injection_rx;

        // RPC-386: stamp the owning-manager back-reference for isolated sessions
        // too (before spawn_agent_loop), so spawned isolated subordinates bind
        // their AgentManager handler to THIS manager rather than the singleton.
        session.set_owning_manager(self.self_weak.clone());

        // WT-005: persist the session manifest to disk so /resume parity
        // holds for isolated sessions exactly like the plain
        // `create_session_with_id` path. Best-effort — a persistence
        // failure never blocks session creation (the in-memory session is
        // fully functional and the git-session manifest already exists).
        let mut isolated_persist_manifest =
            codelet_core::persistence::SessionManifest::with_provider(
                name,
                project_path.clone(),
                model,
            );
        isolated_persist_manifest.id = uuid;
        if let Err(e) = codelet_core::persistence::save_session(&isolated_persist_manifest) {
            tracing::warn!(
                session_id = %uuid,
                error = %e,
                "create_isolated_session_with_id: failed to persist session manifest (best-effort)"
            );
        }

        self.hooks()
            .spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx);

        // RPC-385: capture the SessionInfo BEFORE the insert moves `session`.
        let created_info = session.get_info();

        self.sessions
            .write()
            .expect("sessions lock poisoned")
            .insert(uuid, session);

        self.set_active_session(uuid);

        // RPC-385: fire the session-created broadcast for isolated/worktree
        // sessions too so spawned isolated subordinates are equally visible.
        let _ = self.session_created_tx.send(created_info);

        // RPC-041: Emit IsolationStateChange directly on the manager-owned
        // chunks_tx (the previous `hooks().emit_isolation_state_change(...)`
        // delegation is gone — the hook has been removed from the trait).
        // WT-001: use the 3-arg constructor so the chunk carries the base
        // commit SHA the worktree was forked from (RPC-036 wire contract).
        let _ = self.chunks_tx.send((
            codelet_rpc_types::SessionId::from(id.to_string()),
            codelet_rpc_types::StreamChunk::isolation_state_change_with_base(
                true,
                Some(worktree_path.to_string_lossy().to_string()),
                Some(base_commit.clone()),
            ),
        ));

        self.hooks().spawn_footer_poller(
            id.to_string(),
            worktree_path.to_string_lossy().to_string(),
            Some(worktree_path.to_string_lossy().to_string()),
        );

        codelet_tools::broadcast_metadata_update();

        Ok(codelet_rpc_types::IsolatedSessionInfo {
            session_id: codelet_rpc_types::SessionId::new(id.to_string()),
            worktree_path: worktree_path.to_string_lossy().to_string(),
            base_commit,
        })
    }
    /// SCHED-004: Spawn a session triggered by the scheduler.
    #[allow(clippy::too_many_arguments)]
    pub async fn spawn_scheduled_session(
        &self,
        id: &str,
        model: &str,
        project: &str,
        name: &str,
        schedule_name: &str,
        role: Option<&str>,
        prompt: &str,
    ) -> Result<(), String> {
        self.create_session_with_id(id, model, project, name)
            .await?;

        let uuid = Uuid::parse_str(id).map_err(|e| format!("Invalid session ID: {}", e))?;
        let sessions = self.sessions.read().expect("sessions lock poisoned");
        if let Some(session) = sessions.get(&uuid) {
            session
                .schedule_triggered
                .store(true, std::sync::atomic::Ordering::Relaxed);
            *session.schedule_name.write().expect("schedule_name lock") =
                Some(schedule_name.to_string());

            if let Some(role_str) = role {
                if !role_str.is_empty() {
                    session.set_role(role_str.to_string());
                }
            }

            session.send_input(prompt.to_string(), None)?;
        }

        Ok(())
    }
}
