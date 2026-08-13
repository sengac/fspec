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

use super::custom_oauth::{
    read_stored_tokens, resolve_login_implementation, write_stored_tokens, LoginImplementation,
};
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
        let tokens = match read_stored_tokens(&self.provider_name)? {
            Some(t) => t,
            None => return Ok(false),
        };
        let needs = auth_needs_refresh_or_legacy(self.provider, tokens.clone()).await?;
        if !needs {
            return Ok(false);
        }
        let refreshed = auth_refresh_or_legacy(self.provider, tokens).await?;
        write_stored_tokens(&self.provider_name, &refreshed)?;
        Ok(true)
    }
}
