//! MockBackend behaviour for one group of FspecBackend methods.

use super::*;

impl MockBackend {
    pub async fn oauth_clear_tokens(&self, provider_id: String) -> Result<()> {
        self.controls
            .oauth_clear_tokens_calls
            .fetch_add(1, Ordering::SeqCst);
        self.controls
            .oauth_clear_tokens_providers
            .lock()
            .push(provider_id.clone());
        if let Some(msg) = self.controls.oauth_clear_tokens_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        // Mirror the real clear: the provider's OAuth tokens go away, so a
        // follow-up `list_provider_credentials` reflects it as unconfigured
        // (projection then drops the oauth-status / Logout row). Idempotent.
        let mut store = self.controls.provider_credentials.lock();
        if let Some(row) = store.iter_mut().find(|p| p.provider_id == provider_id) {
            row.configured = false;
            row.masked_key = None;
            row.source = None;
        }
        Ok(())
    }
    pub async fn oauth_get_tokens(&self, provider_id: String) -> Result<bool> {
        self.controls
            .oauth_get_tokens_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(self
            .controls
            .oauth_get_tokens_results
            .lock()
            .get(&provider_id)
            .copied()
            .unwrap_or(false))
    }
    pub async fn oauth_headless_start(
        &self,
        _provider_id: String,
    ) -> Result<codelet_rpc_types::OAuthHeadlessStart> {
        self.controls
            .oauth_headless_start_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self.controls.oauth_headless_start_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let (authorize_url, pkce_verifier) =
            self.controls.oauth_headless_start_result.lock().clone();
        Ok(codelet_rpc_types::OAuthHeadlessStart {
            authorize_url,
            pkce_verifier,
        })
    }
    pub async fn oauth_headless_complete(
        &self,
        provider_id: String,
        code_with_state: String,
        pkce_verifier: String,
    ) -> Result<()> {
        self.controls
            .oauth_headless_complete_calls
            .fetch_add(1, Ordering::SeqCst);
        self.controls.oauth_headless_complete_args.lock().push((
            provider_id,
            code_with_state,
            pkce_verifier,
        ));
        if let Some(msg) = self.controls.oauth_headless_complete_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }
    pub async fn oauth_device_start(
        &self,
        provider_id: String,
    ) -> Result<codelet_rpc_types::OAuthDeviceStart> {
        self.controls
            .oauth_device_start_calls
            .fetch_add(1, Ordering::SeqCst);
        self.controls
            .oauth_device_start_providers
            .lock()
            .push(provider_id);
        if let Some(msg) = self.controls.oauth_device_start_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let (user_code, verification_url, device_auth_id, interval) =
            self.controls.oauth_device_start_result.lock().clone();
        Ok(codelet_rpc_types::OAuthDeviceStart {
            user_code,
            verification_url,
            device_auth_id,
            interval,
        })
    }
}
