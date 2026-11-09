//! MockBackend scripting and recording helpers (part 5).

use super::*;

impl MockBackend {
    /// PROV-113: script the `(authorize_url, pkce_verifier)` returned by
    /// `oauth_headless_start`.
    pub fn seed_oauth_headless_start(&self, authorize_url: &str, pkce_verifier: &str) {
        *self.controls.oauth_headless_start_result.lock() =
            (authorize_url.to_string(), pkce_verifier.to_string());
    }
    /// PROV-113: per-call counter for `oauth_headless_start`.
    pub fn oauth_headless_start_calls(&self) -> usize {
        self.controls
            .oauth_headless_start_calls
            .load(Ordering::SeqCst)
    }
    /// PROV-113: per-call counter for `oauth_headless_complete`.
    pub fn oauth_headless_complete_calls(&self) -> usize {
        self.controls
            .oauth_headless_complete_calls
            .load(Ordering::SeqCst)
    }
    /// PROV-113: every `(provider, code, verifier)` passed to
    /// `oauth_headless_complete`, in call order.
    pub fn oauth_headless_complete_args(&self) -> Vec<(String, String, String)> {
        self.controls.oauth_headless_complete_args.lock().clone()
    }
    /// PROV-113: script the `(user_code, verification_url, device_auth_id,
    /// interval)` returned by `oauth_device_start`.
    pub fn seed_oauth_device_start(
        &self,
        user_code: &str,
        verification_url: &str,
        device_auth_id: &str,
        interval: u64,
    ) {
        *self.controls.oauth_device_start_result.lock() = (
            user_code.to_string(),
            verification_url.to_string(),
            device_auth_id.to_string(),
            interval,
        );
    }
    /// PROV-113: per-call counter for `oauth_device_start`.
    pub fn oauth_device_start_calls(&self) -> usize {
        self.controls
            .oauth_device_start_calls
            .load(Ordering::SeqCst)
    }
    /// PROV-113: per-call counter for `oauth_device_poll`.
    pub fn oauth_device_poll_calls(&self) -> usize {
        self.controls.oauth_device_poll_calls.load(Ordering::SeqCst)
    }
    /// PROV-114: script the `(user_code, verification_url, device_auth_id,
    /// interval)` returned by `oauth_copilot_device_start`.
    pub fn seed_oauth_copilot_device_start(
        &self,
        user_code: &str,
        verification_url: &str,
        device_auth_id: &str,
        interval: u64,
    ) {
        *self.controls.oauth_copilot_device_start_result.lock() = (
            user_code.to_string(),
            verification_url.to_string(),
            device_auth_id.to_string(),
            interval,
        );
    }
    /// PROV-114: force `oauth_copilot_device_start` to fail with `message`.
    pub fn set_oauth_copilot_device_start_error(&self, message: String) {
        *self.controls.oauth_copilot_device_start_error.lock() = Some(message);
    }
    /// PROV-114: per-call counter for `oauth_copilot_device_start`.
    pub fn oauth_copilot_device_start_calls(&self) -> usize {
        self.controls
            .oauth_copilot_device_start_calls
            .load(Ordering::SeqCst)
    }
    /// PROV-114: every `enterprise_host` passed to
    /// `oauth_copilot_device_start`, in call order.
    pub fn oauth_copilot_device_start_hosts(&self) -> Vec<Option<String>> {
        self.controls
            .oauth_copilot_device_start_hosts
            .lock()
            .clone()
    }
    /// PROV-109: per-call counter for `save_profile`.
    pub fn save_profile_calls(&self) -> usize {
        self.controls.save_profile_calls.load(Ordering::SeqCst)
    }
    /// PROV-109: per-call counter for `delete_profile`.
    pub fn delete_profile_calls(&self) -> usize {
        self.controls.delete_profile_calls.load(Ordering::SeqCst)
    }
    /// PROV-109: capture of the last `(provider_id, profile_name, definition)`
    /// tuple passed to `save_profile`.
    pub fn last_save_profile(
        &self,
    ) -> Option<(String, String, codelet_rpc_types::ProfileDefinition)> {
        self.controls.last_save_profile.lock().clone()
    }
    /// PROV-109: capture of the last `(provider_id, profile_name)` pair passed
    /// to `delete_profile`.
    pub fn last_delete_profile(&self) -> Option<(String, String)> {
        self.controls.last_delete_profile.lock().clone()
    }
    /// RPC-054: per-call counter for `list_provider_credentials`.
    pub fn list_provider_credentials_calls(&self) -> usize {
        self.controls
            .list_provider_credentials_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-054: per-call counter for `set_provider_credentials`.
    pub fn set_provider_credentials_calls(&self) -> usize {
        self.controls
            .set_provider_credentials_calls
            .load(Ordering::SeqCst)
    }
    /// RPC-054: per-call counter for `delete_provider_credentials`.
    pub fn delete_provider_credentials_calls(&self) -> usize {
        self.controls
            .delete_provider_credentials_calls
            .load(Ordering::SeqCst)
    }
}
