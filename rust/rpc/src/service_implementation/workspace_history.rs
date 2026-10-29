macro_rules! compose_rpc_workspace_history {
    ($($collected:tt)*) => {
        compose_rpc_providers! {
            $($collected)*
    async fn move_work_unit_up(self, _ctx: Context, id: String) -> Result<(), String> {
        // RPC-017: delegate to the shared work-units write helper.
        // Errors are stringified at the RPC boundary so both transports
        // see the same diagnostic text.
        match self.inner.cwd() {
            Some(cwd) => codelet_core::work_units_write::move_work_unit(
                cwd,
                &id,
                codelet_core::work_units_write::Direction::Up,
            )
            .map_err(|e| format!("{e:#}")),
            None => Err(
                "move_work_unit_up requires a workspace cwd; SharedFspecService was constructed without with_cwd"
                    .to_string(),
            ),
        }
    }

    async fn move_work_unit_down(self, _ctx: Context, id: String) -> Result<(), String> {
        match self.inner.cwd() {
            Some(cwd) => codelet_core::work_units_write::move_work_unit(
                cwd,
                &id,
                codelet_core::work_units_write::Direction::Down,
            )
            .map_err(|e| format!("{e:#}")),
            None => Err(
                "move_work_unit_down requires a workspace cwd; SharedFspecService was constructed without with_cwd"
                    .to_string(),
            ),
        }
    }

    async fn get_model_info(self, _ctx: Context, session_id: SessionId) -> ModelInfo {
        // RPC-018: delegate via the optional SessionManagerHandle. The
        // trait carries a default impl that returns `ModelInfo::default()`,
        // so callers without an attached handle (test fixtures, embedded
        // hosts without a session manager) get the safe sentinel.
        match self.inner.session_manager() {
            Some(handle) => handle.get_model_info(&session_id),
            None => ModelInfo::default(),
        }
    }

    async fn get_thinking_level(self, _ctx: Context, session_id: SessionId) -> ThinkingLevel {
        // RPC-018: mirror of `get_model_info`. Default = `ThinkingLevel::Off`.
        match self.inner.session_manager() {
            Some(handle) => handle.get_thinking_level(&session_id),
            None => ThinkingLevel::Off,
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

    async fn search_files(self, _ctx: Context, prefix: String, limit: u32) -> Vec<String> {
        // RPC-020: delegate to `codelet_core::file_search::search` when
        // a workspace cwd is attached. Without a cwd we return an empty
        // Vec — both transports surface the same "no matches" signal
        // and the @file popup degrades gracefully.
        if prefix.is_empty() {
            return Vec::new();
        }
        match self.inner.cwd() {
            Some(cwd) => codelet_core::file_search::search(cwd, &prefix, limit),
            None => Vec::new(),
        }
    }

    async fn persistence_add_history(
        self,
        _ctx: Context,
        session: SessionId,
        text: String,
    ) -> Result<(), String> {
        // RPC-025: delegate to the lifted core helper. Preserve the
        // original SessionId string via `session_id_str` so non-UUID
        // SessionIds round-trip back through `HistoryMatch` unchanged;
        // the Uuid field is best-effort parsed-or-nil for legacy NAPI
        // / JSONL compatibility.
        let session_uuid =
            uuid::Uuid::parse_str(&session.value).unwrap_or_else(|_| uuid::Uuid::nil());
        let project = self
            .inner
            .cwd()
            .cloned()
            .unwrap_or_else(|| std::path::PathBuf::from(""));
        let entry = codelet_core::persistence::HistoryEntry::with_session_id_str(
            text,
            project,
            session_uuid,
            session.value,
        );
        codelet_core::persistence::history::add(entry)
    }

    async fn persistence_get_history(
        self,
        _ctx: Context,
        _session: SessionId,
        limit: u32,
    ) -> Result<Vec<String>, String> {
        // RPC-025: scope to the workspace cwd if attached; otherwise return
        // entries across all projects. The current contract returns `Vec<String>`
        // (display values only) — search returns richer HistoryMatch values.
        let project = self.inner.cwd().cloned();
        let proj_ref = project.as_deref();
        codelet_core::persistence::history::get(proj_ref, Some(limit as usize))
            .map(|entries| entries.into_iter().map(|e| e.display).collect())
    }

    async fn persistence_search_history(
        self,
        _ctx: Context,
        query: String,
    ) -> Result<Vec<HistoryMatch>, String> {
        // RPC-025: scope to workspace cwd if attached; convert each
        // matching HistoryEntry into a HistoryMatch with an RFC3339
        // timestamp so non-Rust consumers don't need chrono.
        let project = self.inner.cwd().cloned();
        let proj_ref = project.as_deref();
        codelet_core::persistence::history::search(&query, proj_ref).map(|entries| {
            entries
                .iter()
                .map(codelet_core::persistence::HistoryEntry::to_history_match)
                .collect()
        })
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
