use super::*;

impl BackgroundSession {
    pub fn set_owning_manager(
        &self,
        manager: std::sync::Weak<crate::session_manager::SessionManager>,
    ) {
        crate::tool_admission::register(self.id, &self.project, manager.clone());
        if let Ok(mut guard) = self.owning_manager.write() {
            *guard = manager;
        }
    }

    pub fn owning_manager(&self) -> Option<Arc<crate::session_manager::SessionManager>> {
        self.owning_manager
            .read()
            .ok()
            .and_then(|guard| guard.upgrade())
    }

    pub fn effective_cwd(&self) -> PathBuf {
        let worktree_path = self
            .worktree_path
            .read()
            .expect("worktree_path lock poisoned");
        worktree_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(&self.project))
    }

    pub fn worktree_path(&self) -> Option<PathBuf> {
        self.worktree_path
            .read()
            .expect("worktree_path lock poisoned")
            .clone()
    }

    pub fn base_commit(&self) -> Option<String> {
        self.base_commit
            .read()
            .expect("base_commit lock poisoned")
            .clone()
    }

    pub fn clear_isolation(&self) {
        *self
            .worktree_path
            .write()
            .expect("worktree_path lock poisoned") = None;
        *self.base_commit.write().expect("base_commit lock poisoned") = None;
    }

    pub fn hook_context(&self) -> HookContext {
        HookContext {
            session_id: self.id.to_string(),
            cwd: self.effective_cwd().to_string_lossy().to_string(),
            transcript_path: format!(
                "{}/.fspec/sessions/{}/transcript.json",
                self.project, self.id
            ),
        }
    }

    pub(super) fn build_isolation_context(
        &self,
    ) -> Option<codelet_cli::session::context_gathering::IsolationContext> {
        let worktree_path = self.worktree_path()?;
        let project_root = PathBuf::from(&self.project);
        let relative_path = worktree_path
            .strip_prefix(&project_root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| worktree_path.to_string_lossy().to_string());
        Some(codelet_cli::session::context_gathering::IsolationContext {
            is_isolated: true,
            worktree_path: Some(relative_path),
            base_commit: self.base_commit(),
        })
    }

    pub fn checkpoint(&self, label: &str) -> std::result::Result<GhostCheckpoint, SessionError> {
        let worktree_path = self.worktree_path().ok_or(SessionError::NotIsolated)?;
        create_ghost_commit(&worktree_path, &self.id.to_string(), label).map_err(SessionError::from)
    }

    pub fn restore(&self, label: &str) -> std::result::Result<RestoreResult, SessionError> {
        let worktree_path = self.worktree_path().ok_or(SessionError::NotIsolated)?;
        restore_ghost_commit(&worktree_path, &self.id.to_string(), label, true)
            .map_err(SessionError::from)
    }

    pub fn list_checkpoints(&self) -> std::result::Result<Vec<String>, SessionError> {
        let worktree_path = self.worktree_path().ok_or(SessionError::NotIsolated)?;
        list_ghost_checkpoints(&worktree_path, &self.id.to_string()).map_err(SessionError::from)
    }
}
