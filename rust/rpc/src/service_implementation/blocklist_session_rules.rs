macro_rules! compose_rpc_blocklist_session_rules {
    ($($collected:tt)*) => {
        compose_rpc_finish! {
            $($collected)*

    async fn blocklist_disabled_rules(
        self,
        _ctx: Context,
        session_id: SessionId,
    ) -> Result<Vec<String>, String> {
        // BUG-601: the rules the session turned off, as the daemon enforces.
        let handle = self.inner.session_manager().ok_or(crate::NO_SESSION_MANAGER)?;
        handle.blocklist_disabled_rules(&session_id)
    }

    async fn blocklist_set_rule_enabled(
        self,
        _ctx: Context,
        session_id: SessionId,
        rule_id: String,
        enabled: bool,
    ) -> Result<Vec<String>, String> {
        // BUG-601: turn the rule off or on for this session only.
        let handle = self.inner.session_manager().ok_or(crate::NO_SESSION_MANAGER)?;
        handle.blocklist_set_rule_enabled(&session_id, &rule_id, enabled)
    }
        }
    };
}
