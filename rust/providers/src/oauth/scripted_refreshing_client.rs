//! Scripted OAuth refresh middleware — PROV-088.
//!
//! When a Rhai shadow config is active for a provider, outbound HTTP
//! calls must auto-refresh expired tokens before each request.
//! [`ScriptedRefreshingClient::ensure_fresh_if_needed`] reads the
//! stored tokens, consults the script's `auth_needs_refresh` (falling
//! back to `needs_refresh`), and calls `auth_refresh` (falling back
//! to `refresh_token`) — persisting the new tokens before returning.
//!
//! The dispatcher for "which implementation handles a given provider
//! name" is [`super::custom_oauth::resolve_login_implementation`] —
//! shared with the `/login` flow so both entry points agree on
//! whether a custom script is active. A thin
//! [`resolve_refresh_middleware`] wrapper re-exports it for callers
//! that prefer the refresh-flavoured name.

use anyhow::Result;

use super::credential_store::CredentialStore;
use super::custom_oauth::{resolve_login_implementation, LoginImplementation};
use super::json_convert::{dynamic_to_json_value, json_value_to_dynamic};
use super::script_provider::ScriptedOAuthProvider;
use super::script_provider_aliases::{auth_needs_refresh_or_legacy, auth_refresh_or_legacy};

/// Which refresh implementation the dispatcher should activate.
///
/// This is a semantic alias for [`LoginImplementation`] — the refresh
/// flow and the login flow share the same "is there a shadow config"
/// question, so there is only one source of truth.
pub type RefreshMiddleware = LoginImplementation;

/// Dispatcher: is a shadow config present for `provider_name`?
///
/// Delegates to [`resolve_login_implementation`] — the two dispatchers
/// must agree for `/login <name>` and runtime refresh to route through
/// the same code path.
pub fn resolve_refresh_middleware(provider_name: &str) -> RefreshMiddleware {
    resolve_login_implementation(provider_name)
}

/// Auto-refresh helper bound to a scripted provider + provider name.
pub struct ScriptedRefreshingClient<'a> {
    provider: &'a ScriptedOAuthProvider,
    provider_name: String,
}

impl<'a> ScriptedRefreshingClient<'a> {
    pub fn new(provider: &'a ScriptedOAuthProvider, provider_name: &str) -> Self {
        Self {
            provider,
            provider_name: provider_name.to_string(),
        }
    }

    /// If stored tokens exist and the script says they need refresh,
    /// invoke `auth_refresh` and persist the result. Returns `true`
    /// when a refresh happened.
    pub async fn ensure_fresh_if_needed(&self) -> Result<bool> {
        let path = super::scripted_request_refresh::credential_path(&self.provider_name)?;
        self.ensure_fresh_at(&path).await
    }

    pub(crate) async fn ensure_fresh_at(&self, path: &std::path::Path) -> Result<bool> {
        super::scripted_request_refresh::check_root(&self.provider_name, path)?;
        let store = CredentialStore::<serde_json::Value>::new(path.to_owned());
        let tokens = match store.read_sync()? {
            Some(value) => json_value_to_dynamic(&value)
                .try_cast::<rhai::Map>()
                .ok_or_else(|| anyhow::anyhow!("stored tokens must be a JSON object"))?,
            None => return Ok(false),
        };
        let needs = auth_needs_refresh_or_legacy(self.provider, tokens.clone()).await?;
        if !needs {
            return Ok(false);
        }
        let refreshed = auth_refresh_or_legacy(self.provider, tokens).await?;
        super::scripted_request_refresh::check_root(&self.provider_name, path)?;
        store.write_sync(&dynamic_to_json_value(&rhai::Dynamic::from_map(refreshed)))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(true)
    }
}
