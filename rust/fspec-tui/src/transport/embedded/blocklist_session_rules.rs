macro_rules! compose_embedded_blocklist_session_rules {
    ($($collected:tt)*) => {
        compose_embedded_finish! {
            $($collected)*
    // BUG-601: the session's turned-off blocklist rules.
    async fn blocklist_disabled_rules(&self, session_id: SessionId) -> Result<Vec<String>> {
        self.client
            .blocklist_disabled_rules(context::current(), session_id)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
    // BUG-601: turn a blocklist rule off or on for the session.
    async fn blocklist_set_rule_enabled(
        &self,
        session_id: SessionId,
        rule_id: String,
        enabled: bool,
    ) -> Result<Vec<String>> {
        self.client
            .blocklist_set_rule_enabled(context::current(), session_id, rule_id, enabled)
            .await?
            .map_err(|error| anyhow::anyhow!(error))
    }
        }
    };
}
