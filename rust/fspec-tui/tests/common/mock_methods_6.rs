//! MockBackend behaviour for one group of FspecBackend methods.

use super::*;

impl MockBackend {
    pub async fn set_debug_directory(&self, path: String) -> Result<()> {
        self.controls
            .set_debug_directory_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.controls.last_set_debug_directory.lock() = Some(path);
        if let Some(msg) = self.controls.set_debug_directory_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }
    // ── RPC-430 debug hydration / propagation ──────────────────────────

    pub async fn get_debug_enabled(&self, _session_id: SessionId) -> Result<bool> {
        self.controls
            .get_debug_enabled_calls
            .fetch_add(1, Ordering::SeqCst);
        match self.controls.get_debug_enabled_result.lock().clone() {
            Ok(val) => Ok(val),
            Err(msg) => Err(anyhow::anyhow!("{msg}")),
        }
    }
    pub async fn set_debug_enabled(&self, session_id: SessionId, enabled: bool) -> Result<()> {
        self.controls
            .set_debug_enabled_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.controls.last_set_debug_enabled.lock() = Some((session_id, enabled));
        Ok(())
    }
    pub async fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>> {
        self.controls
            .blocklist_list_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self.controls.blocklist_list_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(self.controls.blocklist_rules.lock().clone())
    }
    pub async fn merge_session_worktree(
        &self,
        _session_id: SessionId,
        _strategy: codelet_rpc_types::MergeStrategy,
    ) -> Result<codelet_rpc_types::MergeOutcome> {
        self.controls
            .merge_session_worktree_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self.controls.merge_session_worktree_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(self.controls.merge_outcome.lock().clone())
    }
    pub async fn discard_session_worktree(&self, _session_id: SessionId) -> Result<()> {
        self.controls
            .discard_session_worktree_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self.controls.discard_session_worktree_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }
    pub async fn prune_orphaned_worktrees(&self) -> Result<Vec<String>> {
        self.controls
            .prune_orphaned_worktrees_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self
            .orchestration
            .prune_orphaned_worktrees_error
            .lock()
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(self.orchestration.pruned_sessions.lock().clone())
    }
    pub async fn list_session_worktrees(
        &self,
    ) -> Result<Vec<codelet_rpc_types::SessionWorktreeInfo>> {
        self.orchestration
            .list_session_worktrees_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(self.orchestration.session_worktrees.lock().clone())
    }
    pub async fn inspect_session_changes(
        &self,
        _session_id: SessionId,
    ) -> Result<codelet_rpc_types::SessionChangesSummary> {
        self.orchestration
            .inspect_session_changes_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(self.orchestration.session_changes_summary.lock().clone())
    }
    pub async fn session_card_changes(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<codelet_rpc_types::SessionCardChange>> {
        self.orchestration
            .session_card_changes_calls
            .fetch_add(1, Ordering::SeqCst);
        let answer = self.orchestration.session_card_changes.lock().clone();
        match answer {
            Some(answer) => answer(&session_id).map_err(|e| anyhow::anyhow!("{e}")),
            None => Ok(Vec::new()),
        }
    }
    pub async fn move_session_cards_back(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<codelet_rpc_types::CardMoveBack>> {
        self.orchestration
            .move_session_cards_back_calls
            .fetch_add(1, Ordering::SeqCst);
        let answer = self.orchestration.move_session_cards_back.lock().clone();
        match answer {
            Some(answer) => answer(&session_id).map_err(|e| anyhow::anyhow!("{e}")),
            None => Ok(Vec::new()),
        }
    }
}
