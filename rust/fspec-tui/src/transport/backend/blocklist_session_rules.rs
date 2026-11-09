macro_rules! compose_backend_blocklist_session_rules {
    ($($collected:tt)*) => {
        compose_backend_finish! {
            $($collected)*
    /// BUG-601: the blocklist rule ids the daemon has turned off for the
    /// session (sorted); a backend without the RPC refuses.
    async fn blocklist_disabled_rules(&self, _session_id: SessionId) -> Result<Vec<String>> {
        Err(BackendError::Unimplemented("blocklist_disabled_rules").into())
    }
    /// BUG-601: turn a blocklist rule off (`enabled` false) or on for the
    /// session in the daemon; answers the session's turned-off rule ids.
    async fn blocklist_set_rule_enabled(
        &self,
        _session_id: SessionId,
        _rule_id: String,
        _enabled: bool,
    ) -> Result<Vec<String>> {
        Err(BackendError::Unimplemented("blocklist_set_rule_enabled").into())
    }
        }
    };
}
