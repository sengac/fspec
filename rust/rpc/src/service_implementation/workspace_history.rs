macro_rules! compose_rpc_workspace_history {
    ($($collected:tt)*) => {
        compose_rpc_providers! {
            $($collected)*
    async fn move_work_unit_up(
        self,
        _ctx: Context,
        workspace: String,
        id: String,
    ) -> Result<(), String> {
        // RPC-017: delegate to the shared work-units write helper.
        // RPC-450: on the named workspace (no start-workspace default).
        let cwd = self.workspace_cwd(&workspace)?;
        codelet_core::work_units_write::move_work_unit(
            &cwd,
            &id,
            codelet_core::work_units_write::Direction::Up,
        )
        .map_err(|e| format!("{e:#}"))
    }

    async fn move_work_unit_down(
        self,
        _ctx: Context,
        workspace: String,
        id: String,
    ) -> Result<(), String> {
        let cwd = self.workspace_cwd(&workspace)?;
        codelet_core::work_units_write::move_work_unit(
            &cwd,
            &id,
            codelet_core::work_units_write::Direction::Down,
        )
        .map_err(|e| format!("{e:#}"))
    }

    async fn get_model_info(self, _ctx: Context, session_id: SessionId) -> Result<ModelInfo, String> {
        // RPC-018: delegate via the optional SessionManagerHandle. The
        // trait carries a default impl that returns `ModelInfo::default()`,
        // so callers without an attached handle (test fixtures, embedded
        // hosts without a session manager) get the safe sentinel.
        match self.inner.session_manager() {
            Some(handle) => handle.get_model_info(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn get_thinking_level(self, _ctx: Context, session_id: SessionId) -> Result<ThinkingLevel, String> {
        // RPC-018: mirror of `get_model_info`. Default = `ThinkingLevel::Off`.
        match self.inner.session_manager() {
            Some(handle) => handle.get_thinking_level(&session_id),
            None => Err(crate::NO_SESSION_MANAGER.to_string()),
        }
    }

    async fn get_workspace_info(self, _ctx: Context) -> WorkspaceInfo {
        // RPC-018: build the workspace snapshot from the attached cwd +
        // `codelet_git::status::get_current_branch`. When no cwd is
        // attached we fall back to `std::env::current_dir()` for the
        // cwd string but DELIBERATELY skip the git probe so the
        // SessionFooter degrades to a bare-cwd render (no `[⎇ branch]`).
        //
        // WT-002: a detached-HEAD cwd is still a git repo — the footer
        // shows a "(detached)" indicator (the dispatch layer derives it
        // from `git_branch == Some("(detached)")`) rather than a blank
        // branch.
        match self.inner.cwd() {
            Some(cwd_buf) => {
                let cwd_buf = cwd_buf.clone();
                let git_branch = match codelet_git::status::get_current_branch(&cwd_buf) {
                    Ok(branch) => branch.or(Some("(detached)".to_string())),
                    Err(_) => None,
                };
                WorkspaceInfo {
                    cwd: cwd_buf.to_string_lossy().into_owned(),
                    git_branch,
                }
            }
            None => {
                let cwd_buf = std::env::current_dir().unwrap_or_default();
                WorkspaceInfo {
                    cwd: cwd_buf.to_string_lossy().into_owned(),
                    git_branch: None,
                }
            }
        }
    }

    async fn search_files(
        self,
        _ctx: Context,
        prefix: String,
        limit: u32,
    ) -> Result<Vec<String>, String> {
        // RPC-020: delegate to `codelet_core::file_search::search` in the
        // attached workspace cwd. BUG-465: without a cwd the search is
        // refused rather than answered with "no matches".
        let cwd = self.inner.cwd().ok_or_else(|| "no workspace directory is attached to this fspec service".to_string())?;
        if prefix.is_empty() {
            return Ok(Vec::new());
        }
        Ok(codelet_core::file_search::search(cwd, &prefix, limit))
    }

    async fn persistence_add_history(
        self,
        _ctx: Context,
        session: SessionId,
        text: String,
    ) -> Result<(), String> {
        // RPC-025: recorded under the attached workspace. Without one the
        // entry is recorded unscoped (empty project) and read back by the
        // unscoped history — explicitly specified by the RPC-025 scenario
        // "FspecServiceImpl falls back to a None project filter when the
        // shared service has no workspace cwd attached".
        let project = match self.inner.cwd() {
            Some(cwd) => cwd.clone(),
            None => std::path::PathBuf::new(),
        };
        crate::history_scope::add(project, session, text)
    }

    async fn persistence_get_history(
        self,
        _ctx: Context,
        _session: SessionId,
        limit: u32,
    ) -> Result<Vec<String>, String> {
        // RPC-025: scoped to the attached workspace; a daemon without one
        // reads every project's history (the documented unscoped read).
        crate::history_scope::get(self.inner.cwd().map(std::path::PathBuf::as_path), limit)
    }

    async fn persistence_search_history(
        self,
        _ctx: Context,
        query: String,
    ) -> Result<Vec<HistoryMatch>, String> {
        // RPC-025: scoped like `persistence_get_history`.
        crate::history_scope::search(&query, self.inner.cwd().map(std::path::PathBuf::as_path))
    }

    async fn persistence_delete_session(
        self,
        _ctx: Context,
        session: SessionId,
    ) -> Result<(), String> {
        // RPC-026: parse the SessionId string as a Uuid and delegate to
        // the lifted core helper. Non-UUID SessionIds fall back to
        // Uuid::nil() — same parse-or-nil pattern as
        // persistence_add_history — so synthetic test ids (e.g.
        // "s-1") still round-trip through the call without panicking
        // at the boundary. The core helper is idempotent for unknown
        // ids so the worst case is a silent no-op.
        let uuid = uuid::Uuid::parse_str(&session.value).unwrap_or_else(|_| uuid::Uuid::nil());
        codelet_core::persistence::delete_session(uuid)
    }
        }
    };
}
