macro_rules! compose_rpc_oauth_worktrees {
    ($($collected:tt)*) => {
        compose_rpc_schedule_loops! {
            $($collected)*
    async fn oauth_clear_tokens(self, _ctx: Context, provider_id: String) -> Result<(), String> {
        // PROV-112: providers-direct clear (NOT via session_manager) so the
        // disconnect works regardless of whether a session manager is attached.
        crate::oauth_disconnect::clear_oauth_tokens(&provider_id).await
    }

    async fn oauth_get_tokens(self, _ctx: Context, provider_id: String) -> Result<bool, String> {
        crate::oauth_disconnect::has_oauth_tokens(&provider_id).await
    }

    async fn oauth_browser_login(self, _ctx: Context, provider_id: String) -> Result<(), String> {
        crate::oauth_login::browser_login(&provider_id).await
    }

    async fn oauth_headless_start(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Result<OAuthHeadlessStart, String> {
        crate::oauth_login::headless_start(&provider_id)
    }

    async fn oauth_headless_complete(
        self,
        _ctx: Context,
        provider_id: String,
        code_with_state: String,
        pkce_verifier: String,
    ) -> Result<(), String> {
        crate::oauth_login::headless_complete(&provider_id, &code_with_state, &pkce_verifier).await
    }

    async fn oauth_device_start(
        self,
        _ctx: Context,
        provider_id: String,
    ) -> Result<OAuthDeviceStart, String> {
        crate::oauth_login::device_start(&provider_id).await
    }

    async fn oauth_device_poll(
        self,
        _ctx: Context,
        provider_id: String,
        device_auth_id: String,
        interval: u64,
    ) -> Result<(), String> {
        crate::oauth_login::device_poll(&provider_id, device_auth_id, interval).await
    }

    async fn oauth_copilot_device_start(
        self,
        _ctx: Context,
        enterprise_host: Option<String>,
    ) -> Result<OAuthDeviceStart, String> {
        crate::oauth_copilot::device_start(enterprise_host).await
    }

    async fn blocklist_list(self, _ctx: Context) -> Vec<BlocklistRuleInfo> {
        match self.inner.session_manager() {
            Some(handle) => handle.blocklist_list(),
            None => Vec::new(),
        }
    }

    async fn merge_session_worktree(
        self,
        _ctx: Context,
        session_id: SessionId,
        strategy: MergeStrategy,
    ) -> Result<MergeOutcome, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.merge_session_worktree(&session_id, strategy),
            None => Ok(MergeOutcome::default()),
        }
    }

    async fn discard_session_worktree(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.discard_session_worktree(&session_id),
            None => Ok(()),
        }
    }

    async fn prune_orphaned_worktrees(self, _ctx: Context) -> Result<Vec<String>, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.prune_orphaned_worktrees(),
            None => Ok(Vec::new()),
        }
    }

    async fn list_session_worktrees(self, _ctx: Context) -> Vec<SessionWorktreeInfo> {
        match self.inner.session_manager() {
            Some(handle) => handle.list_session_worktrees(),
            None => Vec::new(),
        }
    }

    async fn inspect_session_changes(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<SessionChangesSummary, String> {
        match self.inner.session_manager() {
            Some(handle) => handle.inspect_session_changes(&session_id),
            None => Ok(SessionChangesSummary::default()),
        }
    }

    async fn detach_session_worktree(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<(), String> {
        match self.inner.session_manager() {
            Some(handle) => handle.detach_session_worktree(&session_id),
            None => Err("no session manager available".to_string()),
        }
    }
        }
    };
}
