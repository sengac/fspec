//! Scripted OAuth Provider (PROV-060 + PROV-087)
//!
//! `ScriptedOAuthProvider` loads `.rhai` files that define custom OAuth flows.
//! Scripts may define up to five functions using the preferred names
//! introduced in PROV-087:
//! - `auth_start(config)` → Map with url, pkce_verifier, state
//! - `auth_exchange(config, code, pkce_verifier)` → Map with tokens
//! - `auth_refresh(config, current_tokens)` → Map with new tokens
//! - `auth_needs_refresh(tokens)` → bool
//! - `poll_for_token(config, device_data)` → Map with status + tokens
//!
//! For backward compatibility, the legacy PROV-060 names are still accepted
//! as fallbacks: `build_authorization_request`, `exchange_code`,
//! `refresh_token`, `needs_refresh`. Use the `auth_*_or_legacy` free
//! functions to invoke whichever name the script defines.

use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use rhai::{Dynamic, Engine, Map, AST};

use super::engine::build_default_engine;
use super::script_invoke::{call_script_bool, call_script_map};

/// Configuration for a scripted OAuth provider.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScriptProviderConfig {
    /// Provider name identifier
    pub name: String,
    /// Display name for the provider
    pub display_name: String,
    /// Path to the .rhai script file
    pub script: String,
    /// Authorization URL
    pub auth_url: Option<String>,
    /// Token URL
    pub token_url: Option<String>,
    /// OAuth client_id
    pub client_id: Option<String>,
    /// Redirect URI
    pub redirect_uri: Option<String>,
    /// OAuth scopes (space-separated)
    pub scopes: Option<String>,
    /// OAuth flow type (authorization_code, device_code)
    pub flow: Option<String>,
    /// Credential file name
    pub credential_file: Option<String>,
}

/// A scripted OAuth provider that executes Rhai scripts.
pub struct ScriptedOAuthProvider {
    engine: Arc<Engine>,
    ast: AST,
    config: ScriptProviderConfig,
}

impl ScriptedOAuthProvider {
    /// Load a scripted provider from a .rhai file.
    pub fn load(script_path: &Path, config: ScriptProviderConfig) -> Result<Self> {
        let engine = build_default_engine();
        let script_content = std::fs::read_to_string(script_path)
            .map_err(|e| anyhow!("Failed to read script {}: {e}", script_path.display()))?;
        let ast = engine.compile(&script_content).map_err(|e| {
            anyhow!(
                "Failed to compile script {}: {e}",
                script_path.display()
            )
        })?;
        Ok(Self {
            engine: Arc::new(engine),
            ast,
            config,
        })
    }

    /// Load from a script string (for testing).
    pub fn from_script(script: &str, config: ScriptProviderConfig) -> Result<Self> {
        let engine = build_default_engine();
        let ast = engine
            .compile(script)
            .map_err(|e| anyhow!("Failed to compile script: {e}"))?;
        Ok(Self {
            engine: Arc::new(engine),
            ast,
            config,
        })
    }

    /// Build the config Dynamic map from the provider config.
    fn config_map(&self) -> Dynamic {
        let mut map = Map::new();
        map.insert("name".into(), Dynamic::from(self.config.name.clone()));
        if let Some(ref v) = self.config.auth_url {
            map.insert("auth_url".into(), Dynamic::from(v.clone()));
        }
        if let Some(ref v) = self.config.token_url {
            map.insert("token_url".into(), Dynamic::from(v.clone()));
        }
        if let Some(ref v) = self.config.client_id {
            map.insert("client_id".into(), Dynamic::from(v.clone()));
        }
        if let Some(ref v) = self.config.redirect_uri {
            map.insert("redirect_uri".into(), Dynamic::from(v.clone()));
        }
        if let Some(ref v) = self.config.scopes {
            map.insert("scopes".into(), Dynamic::from(v.clone()));
        }
        Dynamic::from_map(map)
    }

    /// Call `build_authorization_request(config)` in the script.
    ///
    /// Runs synchronously inside `tokio::task::spawn_blocking`.
    pub async fn build_authorization_request(&self) -> Result<Map> {
        call_script_map(
            self.engine.clone(),
            self.ast.clone(),
            "build_authorization_request",
            (self.config_map(),),
        )
        .await
    }

    /// Call `exchange_code(config, code, pkce_verifier)` in the script.
    pub async fn exchange_code(&self, code: &str, pkce_verifier: &str) -> Result<Map> {
        call_script_map(
            self.engine.clone(),
            self.ast.clone(),
            "exchange_code",
            (self.config_map(), code.to_string(), pkce_verifier.to_string()),
        )
        .await
    }

    /// Call `refresh_token(config, current_tokens)` in the script.
    pub async fn refresh_token(&self, current_tokens: Map) -> Result<Map> {
        call_script_map(
            self.engine.clone(),
            self.ast.clone(),
            "refresh_token",
            (self.config_map(), Dynamic::from_map(current_tokens)),
        )
        .await
    }

    /// Call `poll_for_token(config, device_data)` in the script.
    pub async fn poll_for_token(&self, device_data: Map) -> Result<Map> {
        call_script_map(
            self.engine.clone(),
            self.ast.clone(),
            "poll_for_token",
            (self.config_map(), Dynamic::from_map(device_data)),
        )
        .await
    }

    /// Call `needs_refresh(tokens)` in the script.
    pub async fn needs_refresh(&self, tokens: Map) -> Result<bool> {
        call_script_bool(
            self.engine.clone(),
            self.ast.clone(),
            "needs_refresh",
            (Dynamic::from_map(tokens),),
        )
        .await
    }

    /// Get the provider config.
    pub fn config(&self) -> &ScriptProviderConfig {
        &self.config
    }
}

impl std::fmt::Debug for ScriptedOAuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptedOAuthProvider")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

// PROV-087: small accessors used by the `auth_*` alias dispatcher in
// `script_provider_aliases.rs`.
impl ScriptedOAuthProvider {
    pub(crate) fn engine_arc(&self) -> Arc<Engine> {
        self.engine.clone()
    }
    pub(crate) fn compiled_ast(&self) -> &AST {
        &self.ast
    }
    pub(crate) fn config_as_dynamic(&self) -> Dynamic {
        self.config_map()
    }
}
