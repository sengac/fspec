//! Scripted OAuth device-code flow — PROV-088.
//!
//! Pure-Rust helpers for the NAPI `custom_oauth_device_*` bindings.
//! They delegate to the `auth_start` / `auth_poll` functions (falling
//! back to the legacy PROV-060 names `build_authorization_request` /
//! `poll_for_token`) and persist tokens through the existing
//! `custom_oauth::write_stored_tokens` helper when a poll succeeds.

use anyhow::{anyhow, Result};
use rhai::{Dynamic, Map, Scope};

use super::custom_oauth::write_stored_tokens;
use super::script_provider::ScriptedOAuthProvider;
use super::script_provider_aliases::auth_start_or_legacy;

/// Wrapper around a `ScriptedOAuthProvider` that surfaces the two
/// device-code operations the NAPI layer needs: `start` and `poll`.
pub struct ScriptedDeviceFlow<'a> {
    provider: &'a ScriptedOAuthProvider,
}

impl<'a> ScriptedDeviceFlow<'a> {
    pub fn new(provider: &'a ScriptedOAuthProvider) -> Self {
        Self { provider }
    }

    /// Invoke the script's `auth_start` (or legacy
    /// `build_authorization_request`) and return the authorization
    /// map as-is — typically containing `device_code`, `user_code`,
    /// `verification_uri`, and `interval`.
    pub async fn start(&self) -> Result<Map> {
        auth_start_or_legacy(self.provider).await
    }

    /// Invoke the script's `auth_poll(config, device_data)` — or the
    /// legacy `poll_for_token` — and return the map as-is. The map
    /// must contain a `status` field; callers are responsible for
    /// persisting tokens when the status is `"success"` using
    /// [`persist_on_success`].
    pub async fn poll(&self, device_data: Map) -> Result<Map> {
        if self.provider.has_fn("auth_poll") {
            let engine = self.provider.engine_arc();
            let ast = self.provider.compiled_ast().clone();
            let config = self.provider.config_as_dynamic();
            tokio::task::spawn_blocking(move || -> Result<Map> {
                let mut scope = Scope::new();
                let dd = Dynamic::from_map(device_data);
                let result: Dynamic = engine
                    .call_fn(&mut scope, &ast, "auth_poll", (config, dd))
                    .map_err(|e| anyhow!("auth_poll failed: {e}"))?;
                result
                    .try_cast::<Map>()
                    .ok_or_else(|| anyhow!("auth_poll must return a Map"))
            })
            .await
            .map_err(|e| anyhow!("spawn_blocking failed: {e}"))?
        } else {
            self.provider.poll_for_token(device_data).await
        }
    }
}

/// Inspect a poll-result `Map`: when `status == "success"`, strip the
/// status field and persist the remaining token fields under
/// `provider_name`. Any other status (including missing) is a no-op.
pub fn persist_on_success(provider_name: &str, result: &Map) -> Result<()> {
    let status = result
        .get("status")
        .and_then(|d| d.clone().into_string().ok())
        .unwrap_or_default();
    if status != "success" {
        return Ok(());
    }
    let mut tokens = result.clone();
    tokens.remove("status");
    write_stored_tokens(provider_name, &tokens)
}
