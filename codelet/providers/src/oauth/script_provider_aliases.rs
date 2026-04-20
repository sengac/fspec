//! PROV-087: Scripted OAuth function-name aliases.
//!
//! The PROV-060 scripted OAuth provider required scripts to define
//! functions named `build_authorization_request` / `exchange_code` /
//! `refresh_token` / `needs_refresh`. PROV-087 introduces friendlier
//! names — `auth_start` / `auth_exchange` / `auth_refresh` /
//! `auth_needs_refresh` — and preserves the legacy names as fallbacks.
//!
//! Methods here delegate to [`script_invoke`](super::script_invoke) so
//! the spawn-blocking / call / cast boilerplate is centralised. Each
//! public method only holds the branch between the preferred and the
//! legacy function name.

use anyhow::Result;
use rhai::{Dynamic, Map};

use super::script_invoke::{call_script_bool, call_script_map};
use super::script_provider::ScriptedOAuthProvider;

impl ScriptedOAuthProvider {
    /// Return true when the compiled script defines a function named
    /// `name` at any arity. Used to prefer PROV-087 names and fall
    /// back to the legacy PROV-060 names when absent.
    pub fn has_fn(&self, name: &str) -> bool {
        self.compiled_ast().iter_functions().any(|f| f.name == name)
    }

    /// PROV-087: invoke `auth_start(config)` if defined, otherwise fall
    /// back to `build_authorization_request(config)`.
    pub async fn auth_start(&self) -> Result<Map> {
        if self.has_fn("auth_start") {
            call_script_map(
                self.engine_arc(),
                self.compiled_ast().clone(),
                "auth_start",
                (self.config_as_dynamic(),),
            )
            .await
        } else {
            self.build_authorization_request().await
        }
    }

    /// PROV-087: invoke `auth_exchange(config, code, verifier)` if
    /// defined, otherwise fall back to `exchange_code`.
    pub async fn auth_exchange(&self, code: &str, verifier: &str) -> Result<Map> {
        if self.has_fn("auth_exchange") {
            call_script_map(
                self.engine_arc(),
                self.compiled_ast().clone(),
                "auth_exchange",
                (
                    self.config_as_dynamic(),
                    code.to_string(),
                    verifier.to_string(),
                ),
            )
            .await
        } else {
            self.exchange_code(code, verifier).await
        }
    }

    /// PROV-087: invoke `auth_needs_refresh(tokens)` if defined,
    /// otherwise fall back to `needs_refresh`.
    pub async fn auth_needs_refresh(&self, tokens: Map) -> Result<bool> {
        if self.has_fn("auth_needs_refresh") {
            call_script_bool(
                self.engine_arc(),
                self.compiled_ast().clone(),
                "auth_needs_refresh",
                (Dynamic::from_map(tokens),),
            )
            .await
        } else {
            self.needs_refresh(tokens).await
        }
    }

    /// PROV-087: invoke `auth_refresh(config, tokens)` if defined,
    /// otherwise fall back to `refresh_token`.
    pub async fn auth_refresh(&self, tokens: Map) -> Result<Map> {
        if self.has_fn("auth_refresh") {
            call_script_map(
                self.engine_arc(),
                self.compiled_ast().clone(),
                "auth_refresh",
                (self.config_as_dynamic(), Dynamic::from_map(tokens)),
            )
            .await
        } else {
            self.refresh_token(tokens).await
        }
    }
}

/// PROV-087: Call `auth_start` with fallback to `build_authorization_request`.
pub async fn auth_start_or_legacy(provider: &ScriptedOAuthProvider) -> Result<Map> {
    provider.auth_start().await
}

/// PROV-087: Call `auth_exchange` with fallback to `exchange_code`.
pub async fn auth_exchange_or_legacy(
    provider: &ScriptedOAuthProvider,
    code: &str,
    verifier: &str,
) -> Result<Map> {
    provider.auth_exchange(code, verifier).await
}

/// PROV-087: Call `auth_needs_refresh` with fallback to `needs_refresh`.
pub async fn auth_needs_refresh_or_legacy(
    provider: &ScriptedOAuthProvider,
    tokens: Map,
) -> Result<bool> {
    provider.auth_needs_refresh(tokens).await
}

/// PROV-087: Call `auth_refresh` with fallback to `refresh_token`.
pub async fn auth_refresh_or_legacy(
    provider: &ScriptedOAuthProvider,
    tokens: Map,
) -> Result<Map> {
    provider.auth_refresh(tokens).await
}
