//! MockBackend scripting and recording helpers (part 2).

use super::*;

impl MockBackend {
    /// RPC-017: counter + capture for `move_work_unit_up`.
    pub fn move_work_unit_up_calls(&self) -> usize {
        self.core.move_work_unit_up_calls.load(Ordering::SeqCst)
    }
    /// RPC-017: counter + capture for `move_work_unit_down`.
    pub fn move_work_unit_down_calls(&self) -> usize {
        self.core.move_work_unit_down_calls.load(Ordering::SeqCst)
    }
    /// RPC-017: the last id passed to `move_work_unit_up`.
    pub fn last_move_work_unit_up_id(&self) -> Option<String> {
        self.core.last_move_work_unit_up_id.lock().clone()
    }
    /// RPC-017: the last id passed to `move_work_unit_down`.
    pub fn last_move_work_unit_down_id(&self) -> Option<String> {
        self.core.last_move_work_unit_down_id.lock().clone()
    }
    /// RPC-018: preload the ModelInfo the next `get_model_info` call returns.
    pub fn set_model_info(&self, info: ModelInfo) {
        *self.core.model_info.lock() = info;
    }
    /// RPC-018: preload the ThinkingLevel the next `get_thinking_level` call returns.
    pub fn set_thinking_level(&self, level: ThinkingLevel) {
        *self.core.thinking_level.lock() = level;
    }
    /// RPC-018: preload the WorkspaceInfo the next `get_workspace_info` call returns.
    pub fn set_workspace_info(&self, info: WorkspaceInfo) {
        *self.core.workspace_info.lock() = info;
    }
    /// RPC-018: force the next `get_workspace_info` call to fail with the
    /// supplied message — exercises the bootstrap best-effort branch.
    pub fn set_workspace_info_error(&self, message: String) {
        *self.core.workspace_info_error.lock() = Some(message);
    }
    /// RPC-020: preload the file-search result list. `search_files`
    /// filters this Vec case-insensitively against its `prefix` arg.
    pub fn set_file_search_results(&self, paths: Vec<String>) {
        *self.core.file_search_results.lock() = paths;
    }
    /// RPC-026: how many times `persistence_delete_session` was awaited.
    pub fn delete_session_calls(&self) -> usize {
        self.core.delete_session_calls.load(Ordering::SeqCst)
    }
    /// RPC-026: the last id passed to `persistence_delete_session`.
    pub fn last_deleted_session(&self) -> Option<SessionId> {
        self.core.last_deleted_session.lock().clone()
    }
    /// RPC-026: preload the result list returned by
    /// `persistence_search_history`. The same Vec is returned for
    /// every query — tests script the desired outcome up front.
    pub fn set_history_search_results(&self, results: Vec<codelet_rpc_types::HistoryMatch>) {
        *self.core.history_search_results.lock() = results;
    }
    /// RPC-026: how many times `persistence_search_history` was awaited.
    pub fn search_history_calls(&self) -> usize {
        self.core.search_history_calls.load(Ordering::SeqCst)
    }
    /// RPC-026: the last query passed to `persistence_search_history`.
    pub fn last_history_query(&self) -> Option<String> {
        self.core.last_history_query.lock().clone()
    }
    /// RPC-026: seed the in-memory `sessions` list returned by
    /// `list_sessions`. Tests use this to script the resume picker.
    pub fn seed_sessions(&self, sessions: Vec<SessionInfo>) {
        *self.core.sessions.lock() = sessions;
    }
    /// RPC-022: preload the provider registry returned by
    /// `list_providers`.
    pub fn seed_providers(&self, providers: Vec<ProviderInfo>) {
        *self.core.providers.lock() = providers;
    }
    /// RPC-451: make `list_providers` fail with `message`.
    /// BUG-466: make every later `list_sessions` fail with `message`.
    pub fn fail_list_sessions(&self, message: &str) {
        *self.backend_state.list_sessions_error.lock() = Some(message.to_string());
    }
    pub fn fail_list_providers(&self, message: &str) {
        *self.core.list_providers_error.lock() = Some(message.to_string());
    }
    /// RPC-022: how many times `list_providers` was awaited.
    pub fn list_providers_calls(&self) -> usize {
        self.core.list_providers_calls.load(Ordering::SeqCst)
    }
    /// BUG-207: method names in await order (see `call_log` field).
    pub fn call_log(&self) -> Vec<String> {
        self.core.call_log.lock().clone()
    }
    /// RPC-022: how many times `set_session_model` was awaited.
    pub fn set_session_model_calls(&self) -> usize {
        self.core.set_session_model_calls.load(Ordering::SeqCst)
    }
    /// RPC-022: how many times `set_thinking_level` was awaited.
    pub fn set_thinking_level_calls(&self) -> usize {
        self.core.set_thinking_level_calls.load(Ordering::SeqCst)
    }
    /// RPC-022: how many times `get_session_role` was awaited.
    pub fn get_session_role_calls(&self) -> usize {
        self.core.get_session_role_calls.load(Ordering::SeqCst)
    }
    /// RPC-022: how many times `set_session_role` was awaited.
    pub fn set_session_role_calls(&self) -> usize {
        self.core.set_session_role_calls.load(Ordering::SeqCst)
    }
    /// RPC-022: the last `(SessionId, provider_id, model_id)` triple
    /// passed to `set_session_model`.
    pub fn last_set_session_model(&self) -> Option<(SessionId, String, String)> {
        self.core.last_set_session_model.lock().clone()
    }
    /// PROV-118: how many times `set_default_model` was awaited.
    pub fn set_default_model_calls(&self) -> usize {
        self.core.set_default_model_calls.load(Ordering::SeqCst)
    }
    /// PROV-118: the last model string passed to `set_default_model`.
    pub fn last_set_default_model(&self) -> Option<String> {
        self.core.last_set_default_model.lock().clone()
    }
}
