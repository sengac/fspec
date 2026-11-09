//! MockBackend scripting and recording helpers (part 5).

use super::*;

impl MockBackend {
    /// RPC-054: per-call counter for `test_provider_connection`.
    pub fn test_provider_connection_calls(&self) -> usize {
        self.controls
            .test_provider_connection_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-054: per-call counter for `refresh_models_cache`.
    pub fn refresh_models_cache_calls(&self) -> usize {
        self.controls
            .refresh_models_cache_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-054: capture of the last `(provider_id, input)` tuple passed
    /// to `set_provider_credentials`.
    pub fn last_set_provider_credentials(&self) -> Option<(String, ProviderCredentialInput)> {
        self.controls.last_set_provider_credentials.lock().clone()
    }
    /// RPC-054: capture of the last provider_id passed to
    /// `test_provider_connection`.
    pub fn last_test_provider_connection(&self) -> Option<String> {
        self.controls.last_test_provider_connection.lock().clone()
    }
    /// RPC-054: capture of the last provider_id passed to
    /// `refresh_models_cache`.
    pub fn last_refresh_models_cache(&self) -> Option<String> {
        self.controls.last_refresh_models_cache.lock().clone()
    }
    /// RPC-054: capture of the last provider_id passed to
    /// `delete_provider_credentials`.
    pub fn last_delete_provider_credentials(&self) -> Option<String> {
        self.controls
            .last_delete_provider_credentials
            .lock()
            .clone()
    }
    // ── RPC-055 debug-capture surface helpers ────────────────────────

    /// RPC-055: how many times `toggle_debug` was awaited.
    pub fn toggle_debug_calls(&self) -> usize {
        self.controls.toggle_debug_calls.load(Ordering::SeqCst)
    }
    /// RPC-055: the last `(SessionId, debug_dir)` tuple passed to
    /// `toggle_debug`.
    pub fn last_toggle_debug(&self) -> Option<(SessionId, String)> {
        self.controls.last_toggle_debug.lock().clone()
    }
    /// RPC-055: script the next `toggle_debug` call to return `Ok(path)`.
    pub fn set_toggle_debug_result_ok(&self, path: String) {
        *self.controls.toggle_debug_result.lock() = Ok(path);
    }
    /// RPC-055: script the next `toggle_debug` call to return
    /// `Err(message)` — exercises the error-notice branch.
    pub fn set_toggle_debug_result_err(&self, message: String) {
        *self.controls.toggle_debug_result.lock() = Err(message);
    }
    /// RPC-055: how many times `set_debug_directory` was awaited.
    pub fn set_debug_directory_calls(&self) -> usize {
        self.controls
            .set_debug_directory_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-055: the last path passed to `set_debug_directory`.
    pub fn last_set_debug_directory(&self) -> Option<String> {
        self.controls.last_set_debug_directory.lock().clone()
    }
    // ── RPC-430 debug hydration / propagation helpers ──────────────────

    /// RPC-430: how many times `get_debug_enabled` was awaited.
    pub fn get_debug_enabled_calls(&self) -> usize {
        self.controls.get_debug_enabled_calls.load(Ordering::SeqCst)
    }
    /// RPC-430: script the next `get_debug_enabled` call to return
    /// `Ok(val)`.
    pub fn set_get_debug_enabled_result_ok(&self, val: bool) {
        *self.controls.get_debug_enabled_result.lock() = Ok(val);
    }
    /// RPC-430: how many times `set_debug_enabled` was awaited.
    pub fn set_debug_enabled_calls(&self) -> usize {
        self.controls.set_debug_enabled_calls.load(Ordering::SeqCst)
    }
    /// RPC-430: the last `(SessionId, bool)` passed to `set_debug_enabled`.
    pub fn last_set_debug_enabled(&self) -> Option<(SessionId, bool)> {
        self.controls.last_set_debug_enabled.lock().clone()
    }
    // ── RPC-056 blocklist surface helpers ────────────────────────────

    /// RPC-056: replace the in-memory rule list returned by
    /// `blocklist_list`.
    pub fn seed_blocklist_rules(&self, rules: Vec<BlocklistRuleInfo>) {
        *self.controls.blocklist_rules.lock() = rules;
    }
    /// RPC-056: how many times `blocklist_list` was awaited.
    pub fn blocklist_list_calls(&self) -> usize {
        self.controls.blocklist_list_calls.load(Ordering::SeqCst)
    }
    /// RPC-056: force the next `blocklist_list` call to fail.
    #[allow(dead_code)]
    pub fn set_blocklist_list_error(&self, message: String) {
        *self.controls.blocklist_list_error.lock() = Some(message);
    }
    // ── RPC-057 /merge-worktree surface helpers ──────────────────────

    /// RPC-057: seed the `MergeOutcome` returned by `merge_session_worktree`.
    #[allow(dead_code)]
    pub fn seed_merge_outcome(&self, outcome: codelet_rpc_types::MergeOutcome) {
        *self.controls.merge_outcome.lock() = outcome;
    }
}
