//! MockBackend behaviour for one group of FspecBackend methods.

use super::*;

impl MockBackend {
    pub async fn oauth_device_poll(
        &self,
        _provider_id: String,
        _device_auth_id: String,
        _interval: u64,
    ) -> Result<()> {
        self.controls
            .oauth_device_poll_calls
            .fetch_add(1, Ordering::SeqCst);
        if let Some(msg) = self.controls.oauth_device_poll_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        Ok(())
    }
    pub async fn oauth_copilot_device_start(
        &self,
        enterprise_host: Option<String>,
    ) -> Result<codelet_rpc_types::OAuthDeviceStart> {
        self.controls
            .oauth_copilot_device_start_calls
            .fetch_add(1, Ordering::SeqCst);
        self.controls
            .oauth_copilot_device_start_hosts
            .lock()
            .push(enterprise_host);
        if let Some(msg) = self
            .controls
            .oauth_copilot_device_start_error
            .lock()
            .clone()
        {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let (user_code, verification_url, device_auth_id, interval) = self
            .controls
            .oauth_copilot_device_start_result
            .lock()
            .clone();
        Ok(codelet_rpc_types::OAuthDeviceStart {
            user_code,
            verification_url,
            device_auth_id,
            interval,
        })
    }
    pub async fn test_provider_connection(
        &self,
        provider_id: String,
    ) -> Result<TestConnectionResult> {
        self.controls
            .test_provider_connection_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.controls.last_test_provider_connection.lock() = Some(provider_id.clone());
        if let Some(msg) = self.controls.test_provider_connection_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let scripted = self
            .controls
            .test_connection_results
            .lock()
            .get(&provider_id)
            .cloned();
        Ok(scripted.unwrap_or(TestConnectionResult {
            success: true,
            error: None,
            latency_ms: 0,
        }))
    }
    pub async fn refresh_models_cache(&self, provider_id: String) -> Result<Vec<ModelEntry>> {
        self.controls
            .refresh_models_cache_calls
            .fetch_add(1, Ordering::SeqCst);
        self.core
            .call_log
            .lock()
            .push("refresh_models_cache".to_string());
        *self.controls.last_refresh_models_cache.lock() = Some(provider_id.clone());
        if let Some(msg) = self.controls.refresh_models_cache_error.lock().clone() {
            return Err(anyhow::anyhow!("{msg}"));
        }
        let scripted = self
            .controls
            .refresh_models_results
            .lock()
            .get(&provider_id)
            .cloned();
        Ok(scripted.unwrap_or_default())
    }
    // ── RPC-055 debug-capture surface ────────────────────────────────

    pub async fn toggle_debug(&self, session_id: SessionId, debug_dir: String) -> Result<String> {
        self.controls
            .toggle_debug_calls
            .fetch_add(1, Ordering::SeqCst);
        *self.controls.last_toggle_debug.lock() = Some((session_id, debug_dir));
        match self.controls.toggle_debug_result.lock().clone() {
            Ok(path) => Ok(path),
            Err(msg) => Err(anyhow::anyhow!("{msg}")),
        }
    }
    // TUI-122: scripted workspace registry edits.
    pub async fn add_workspace(&self, path: String) -> Result<codelet_rpc_types::WorkspaceEntry> {
        self.mock_add_workspace(&path)
            .map_err(|message| anyhow::anyhow!(message))
    }
    pub async fn remove_workspace(&self, path: String) -> Result<()> {
        self.mock_remove_workspace(&path);
        Ok(())
    }
}
