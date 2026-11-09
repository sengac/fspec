//! MockBackend scripting and recording helpers (part 4).

use super::*;

impl MockBackend {
    /// RPC-053: force the next `send_hitl_response` call to fail.
    pub fn set_send_hitl_response_error(&self, message: String) {
        *self.controls.send_hitl_response_error.lock() = Some(message);
    }
    // ── RPC-054 seed / mutator helpers ───────────────────────────────────

    /// RPC-054: replace the in-memory provider credential list returned
    /// by `list_provider_credentials`.
    pub fn seed_provider_credentials(&self, list: Vec<ProviderCredentialInfo>) {
        *self.controls.provider_credentials.lock() = list;
    }
    /// RPC-054: script the list returned by the very NEXT
    /// `list_provider_credentials` call after a successful save
    /// (one-shot — consumed by the call).
    pub fn set_post_save_provider_credentials(&self, list: Vec<ProviderCredentialInfo>) {
        *self.controls.provider_credentials_after_save.lock() = Some(list);
    }
    /// RPC-054: script the list returned by the very NEXT
    /// `list_provider_credentials` call after a successful delete.
    pub fn set_post_delete_provider_credentials(&self, list: Vec<ProviderCredentialInfo>) {
        *self.controls.provider_credentials_after_delete.lock() = Some(list);
    }
    /// RPC-054: script the list returned by the very NEXT
    /// `list_provider_credentials` call after a successful refresh.
    pub fn set_post_refresh_provider_credentials(&self, list: Vec<ProviderCredentialInfo>) {
        *self.controls.provider_credentials_after_refresh.lock() = Some(list);
    }
    /// RPC-054: script the result returned by
    /// `test_provider_connection` for the given provider_id.
    pub fn set_test_connection_result(
        &self,
        provider_id: impl Into<String>,
        result: TestConnectionResult,
    ) {
        self.controls
            .test_connection_results
            .lock()
            .insert(provider_id.into(), result);
    }
    /// RPC-054: script the model list returned by
    /// `refresh_models_cache` for the given provider_id.
    pub fn set_refresh_models_result(
        &self,
        provider_id: impl Into<String>,
        models: Vec<ModelEntry>,
    ) {
        self.controls
            .refresh_models_results
            .lock()
            .insert(provider_id.into(), models);
    }
    /// RPC-054: force the next `set_provider_credentials` call to fail.
    pub fn set_set_provider_credentials_error(&self, message: String) {
        *self.controls.set_provider_credentials_error.lock() = Some(message);
    }
    /// RPC-054: force the next `list_provider_credentials` call to fail.
    pub fn set_list_provider_credentials_error(&self, message: String) {
        *self.controls.list_provider_credentials_error.lock() = Some(message);
    }
    /// PROV-109: force the next `save_profile` call to fail.
    pub fn set_save_profile_error(&self, message: String) {
        *self.controls.save_profile_error.lock() = Some(message);
    }
    /// PROV-109: force the next `delete_profile` call to fail.
    pub fn set_delete_profile_error(&self, message: String) {
        *self.controls.delete_profile_error.lock() = Some(message);
    }
    /// PROV-112: force `oauth_clear_tokens` to fail with `message`.
    pub fn set_oauth_clear_tokens_error(&self, message: String) {
        *self.controls.oauth_clear_tokens_error.lock() = Some(message);
    }
    /// PROV-112: per-call counter for `oauth_clear_tokens`.
    pub fn oauth_clear_tokens_calls(&self) -> usize {
        self.controls
            .oauth_clear_tokens_calls
            .load(Ordering::SeqCst)
    }
    /// PROV-112: every `provider_id` passed to `oauth_clear_tokens`, in order.
    pub fn oauth_clear_tokens_providers(&self) -> Vec<String> {
        self.controls.oauth_clear_tokens_providers.lock().clone()
    }
    /// PROV-112: per-call counter for `oauth_get_tokens`.
    pub fn oauth_get_tokens_calls(&self) -> usize {
        self.controls.oauth_get_tokens_calls.load(Ordering::SeqCst)
    }
    /// PROV-112: seed the `has_tokens` answer for a provider's
    /// `oauth_get_tokens`.
    pub fn seed_oauth_get_tokens(&self, provider_id: &str, has_tokens: bool) {
        self.controls
            .oauth_get_tokens_results
            .lock()
            .insert(provider_id.to_string(), has_tokens);
    }
    /// BUG-600: force `oauth_headless_start` to fail with `message`.
    pub fn set_oauth_headless_start_error(&self, message: String) {
        *self.controls.oauth_headless_start_error.lock() = Some(message);
    }
}

// ── AMGR-020: pending requests and the request ids replies name ────────

fn user_route(request_id: String, session: &SessionId, kind: RequestKind) -> RequestRoute {
    RequestRoute {
        request_id,
        origin: session.clone(),
        kind,
        chain: vec![session.clone()],
        holder: RequestHolder::User,
    }
}

/// The approval route `script_pause_state` wraps its value in:
/// request id `pause-<session>`, held by the user.
pub fn pause_route(session: &SessionId) -> RequestRoute {
    user_route(
        format!("pause-{}", session.value),
        session,
        RequestKind::Approval,
    )
}

/// The question route `script_hitl_request` wraps its value in:
/// request id `question-<session>`, held by the user.
pub fn hitl_route(session: &SessionId) -> RequestRoute {
    user_route(
        format!("question-{}", session.value),
        session,
        RequestKind::Question,
    )
}

impl MockBackend {
    /// AMGR-020: seed the exact pending approval `get_pause_state` returns.
    pub fn script_pending_pause(
        &self,
        session: SessionId,
        value: Option<PendingRequest<PauseState>>,
    ) {
        self.controls
            .pause_state_store
            .lock()
            .insert(session, value);
    }
    /// AMGR-020: seed the exact pending question `get_hitl_request` returns.
    pub fn script_pending_hitl(
        &self,
        session: SessionId,
        value: Option<PendingRequest<HitlRequest>>,
    ) {
        self.controls
            .hitl_request_store
            .lock()
            .insert(session, value);
    }
    pub(super) fn record_reply(&self, method: &'static str, session: &SessionId, id: String) {
        self.controls
            .reply_request_ids
            .lock()
            .push((method, session.clone(), id));
    }
    /// AMGR-020: `(method, session, request_id)` of every `pause_human_action`,
    /// `pause_confirm`, `pause_triple` and `send_hitl_response`, in call order.
    pub fn reply_request_ids(&self) -> Vec<(&'static str, SessionId, String)> {
        self.controls.reply_request_ids.lock().clone()
    }
}
