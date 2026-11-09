macro_rules! compose_rpc_session_mcp {
    ($($collected:tt)*) => {
        compose_rpc_blocklist_session_rules! {
            $($collected)*
    /// MCP-003: the daemon-authenticated MCP configuration entry point.
    async fn configure_session_mcp_servers(
        self,
        _ctx: Context,
        session_id: SessionId,
        servers: Vec<codelet_rpc_types::SessionMcpServer>,
    ) -> Result<Vec<codelet_rpc_types::SessionMcpServerTools>, String> {
        self.require_bearer_for_mcp_configuration()?;
        self.inner
            .session_manager()
            .ok_or("MCP server configuration requires a session manager")?
            .configure_session_mcp_servers(&session_id, servers)
            .await
    }
    /// TOOL-027: forward an exec-session control to the session manager.
    async fn control_exec_stdin(
        self,
        _ctx: Context,
        session_id: SessionId,
        exec_session_id: String,
        control: codelet_rpc_types::ExecStdinControl,
    ) -> Result<(), String> {
        self.inner
            .session_manager()
            .ok_or("control_exec_stdin requires a session manager")?
            .control_exec_stdin(&session_id, &exec_session_id, control)
    }
    /// BUG-427: open the named workspace, then create the isolated session
    /// in its directory.
    async fn create_isolated_session_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
        role: Option<String>,
    ) -> Result<IsolatedSessionInfo, String> {
        let scope = self
            .inner
            .workspace(Some(std::path::Path::new(&workspace)))
            .map_err(|error| error.to_string())?;
        let project = scope
            .cwd()
            .ok_or_else(|| format!("workspace {workspace} has no directory"))?
            .to_path_buf();
        self.inner
            .session_manager()
            .ok_or("Isolated session creation requires a session manager")?
            .create_isolated_session_in_workspace(&project, role)
    }
    /// BUG-431: open the named workspace and read the attachment from its
    /// directory on this (the daemon's) machine.
    async fn read_card_attachment(
        self,
        _ctx: Context,
        workspace: String,
        path: String,
    ) -> Result<Vec<u8>, String> {
        let scope = self
            .inner
            .workspace(Some(std::path::Path::new(&workspace)))
            .map_err(|error| error.to_string())?;
        let root = scope
            .cwd()
            .ok_or_else(|| format!("workspace {workspace} has no directory"))?;
        codelet_attachment_viewer::read_workspace_attachment(root, &path)
    }
    /// RPC-452: list the named workspace's changed files.
    async fn changed_files_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
    ) -> Result<Vec<ChangedFile>, String> {
        let cwd = self.workspace_cwd(&workspace)?;
        collect_changed_files(&cwd).map_err(|error| error.to_string())
    }
    /// RPC-452: the diff of `path` in the named workspace.
    async fn file_diff_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
        path: String,
    ) -> Result<Option<String>, String> {
        let cwd = self.workspace_cwd(&workspace)?;
        codelet_git::get_file_diff(&cwd, &path).map_err(|error| error.to_string())
    }
    async fn checkpoint_counts_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
    ) -> Result<CheckpointCounts, String> {
        let cwd = self.workspace_cwd(&workspace)?;
        codelet_git::ghost_commit::count_checkpoints(&cwd).map_err(|error| error.to_string())
    }
    async fn list_checkpoints_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
    ) -> Result<Vec<CheckpointInfo>, String> {
        let cwd = self.workspace_cwd(&workspace)?;
        let tx = self.inner.checkpoints_progress_tx();
        checkpoints::collect_checkpoints_stream(&cwd, &mut |frame| {
            // Best-effort: no live subscribers is a no-op, never an error.
            let _ = tx.send(frame);
        })
        .map_err(|error| error.to_string())
    }
    async fn checkpoint_diff_files_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
        work_unit_id: String,
        name: String,
    ) -> Result<Vec<ChangedFile>, String> {
        let cwd = self.workspace_cwd(&workspace)?;
        checkpoints::collect_checkpoint_diff_files(&cwd, &work_unit_id, &name)
            .map_err(|error| error.to_string())
    }
    async fn checkpoint_file_diff_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<Option<String>, String> {
        let cwd = self.workspace_cwd(&workspace)?;
        checkpoints::checkpoint_file_diff(&cwd, &work_unit_id, &name, &path)
            .map_err(|error| error.to_string())
    }
    async fn restore_checkpoint_file_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
        work_unit_id: String,
        name: String,
        path: String,
    ) -> Result<(), String> {
        let cwd = self.workspace_cwd(&workspace)?;
        checkpoints::restore_file(&cwd, &work_unit_id, &name, &path).map_err(|error| error.to_string())
    }
    async fn restore_checkpoint_all_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
        work_unit_id: String,
        name: String,
    ) -> Result<(), String> {
        let cwd = self.workspace_cwd(&workspace)?;
        checkpoints::restore_all(&cwd, &work_unit_id, &name).map_err(|error| error.to_string())
    }
    async fn delete_checkpoint_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
        work_unit_id: String,
        name: String,
    ) -> Result<(), String> {
        let cwd = self.workspace_cwd(&workspace)?;
        checkpoints::delete_one(&cwd, &work_unit_id, &name).map_err(|error| error.to_string())
    }
    async fn delete_all_checkpoints_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
    ) -> Result<(), String> {
        let cwd = self.workspace_cwd(&workspace)?;
        checkpoints::delete_all(&cwd).map_err(|error| error.to_string())
    }
    async fn get_workspace_info_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
    ) -> Result<WorkspaceInfo, String> {
        let cwd = self.workspace_cwd(&workspace)?;
        // WT-002: a detached HEAD shows "(detached)"; a workspace that is
        // not a git repository has no branch; any other git error is
        // reported.
        let git_branch = match codelet_git::status::get_current_branch(&cwd) {
            Ok(branch) => Some(branch.unwrap_or_else(|| "(detached)".to_string())),
            Err(codelet_git::GitError::NotARepository { .. }) => None,
            Err(error) => return Err(error.to_string()),
        };
        Ok(WorkspaceInfo {
            cwd: cwd.to_string_lossy().into_owned(),
            git_branch,
        })
    }
    async fn git_state_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
    ) -> Result<GitState, String> {
        Ok(self
            .inner
            .workspace(Some(std::path::Path::new(&workspace)))
            .map_err(|error| error.to_string())?
            .git_state())
    }

    async fn search_files_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
        prefix: String,
        limit: u32,
    ) -> Result<Vec<String>, String> {
        let cwd = self.workspace_cwd(&workspace)?;
        if prefix.is_empty() {
            return Ok(Vec::new());
        }
        Ok(codelet_core::file_search::search(&cwd, &prefix, limit))
    }

    async fn persistence_add_history_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
        session: SessionId,
        text: String,
    ) -> Result<(), String> {
        let cwd = self.workspace_cwd(&workspace)?;
        crate::history_scope::add(cwd, session, text)
    }

    async fn persistence_get_history_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
        _session: SessionId,
        limit: u32,
    ) -> Result<Vec<String>, String> {
        let cwd = self.workspace_cwd(&workspace)?;
        crate::history_scope::get(Some(&cwd), limit)
    }

    async fn persistence_search_history_in_workspace(
        self,
        _ctx: Context,
        workspace: String,
        query: String,
    ) -> Result<Vec<HistoryMatch>, String> {
        let cwd = self.workspace_cwd(&workspace)?;
        crate::history_scope::search(&query, Some(&cwd))
    }

    async fn add_workspace(self, _ctx: Context, path: String) -> Result<WorkspaceEntry, String> {
        self.inner
            .add_workspace(std::path::Path::new(&path))
            .map_err(|error| error.to_string())
    }

    async fn remove_workspace(self, _ctx: Context, path: String) -> Result<(), String> {
        self.inner
            .forget_workspace(std::path::Path::new(&path))
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    async fn get_thinking_options(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<codelet_rpc_types::ThinkingOptions, String> {
        // MODEL-011: what the session's current model offers (appended, RPC-449).
        match self.inner.session_manager() {
            Some(handle) => handle.get_thinking_options(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn search_files_in_session(
        self,
        _ctx: Context,
        session_id: SessionId,
        prefix: String,
        limit: u32,
    ) -> Result<Vec<String>, String> {
        // BUG-541: the session resolves its own directory (its worktree when
        // isolated); an unresolvable one is an error, never the project root.
        let handle = self.inner.session_manager().ok_or(crate::NO_SESSION_MANAGER)?;
        let cwd = handle.get_effective_cwd(&session_id)?;
        if prefix.is_empty() {
            return Ok(Vec::new());
        }
        Ok(codelet_core::file_search::search(&cwd, &prefix, limit))
    }
        }
    };
}
