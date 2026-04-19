//! Scripted OAuth Provider (PROV-060)
//!
//! `ScriptedOAuthProvider` loads `.rhai` files that define custom OAuth flows.
//! Scripts must define up to five functions:
//! - `build_authorization_request(config)` → Map with url, pkce_verifier, state
//! - `exchange_code(config, code, pkce_verifier)` → Map with tokens
//! - `refresh_token(config, current_tokens)` → Map with new tokens
//! - `poll_for_token(config, device_data)` → Map with status + tokens
//! - `needs_refresh(tokens)` → bool

use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use rhai::{AST, Dynamic, Engine, Map, Scope};

use super::engine::build_default_engine;

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
        let engine = self.engine.clone();
        let ast = self.ast.clone();
        let config = self.config_map();

        tokio::task::spawn_blocking(move || -> Result<Map> {
            let mut scope = Scope::new();
            let result: Dynamic = engine
                .call_fn(&mut scope, &ast, "build_authorization_request", (config,))
                .map_err(|e| anyhow!("build_authorization_request failed: {e}"))?;
            result.try_cast::<Map>().ok_or_else(|| {
                anyhow!("build_authorization_request must return a Map")
            })
        })
        .await
        .map_err(|e| anyhow!("spawn_blocking failed: {e}"))?
    }

    /// Call `exchange_code(config, code, pkce_verifier)` in the script.
    pub async fn exchange_code(&self, code: &str, pkce_verifier: &str) -> Result<Map> {
        let engine = self.engine.clone();
        let ast = self.ast.clone();
        let config = self.config_map();
        let code = code.to_string();
        let verifier = pkce_verifier.to_string();

        tokio::task::spawn_blocking(move || -> Result<Map> {
            let mut scope = Scope::new();
            let result: Dynamic = engine
                .call_fn(
                    &mut scope,
                    &ast,
                    "exchange_code",
                    (config, code, verifier),
                )
                .map_err(|e| anyhow!("exchange_code failed: {e}"))?;
            result
                .try_cast::<Map>()
                .ok_or_else(|| anyhow!("exchange_code must return a Map"))
        })
        .await
        .map_err(|e| anyhow!("spawn_blocking failed: {e}"))?
    }

    /// Call `refresh_token(config, current_tokens)` in the script.
    pub async fn refresh_token(&self, current_tokens: Map) -> Result<Map> {
        let engine = self.engine.clone();
        let ast = self.ast.clone();
        let config = self.config_map();

        tokio::task::spawn_blocking(move || -> Result<Map> {
            let mut scope = Scope::new();
            let tokens_dyn = Dynamic::from_map(current_tokens);
            let result: Dynamic = engine
                .call_fn(
                    &mut scope,
                    &ast,
                    "refresh_token",
                    (config, tokens_dyn),
                )
                .map_err(|e| anyhow!("refresh_token failed: {e}"))?;
            result
                .try_cast::<Map>()
                .ok_or_else(|| anyhow!("refresh_token must return a Map"))
        })
        .await
        .map_err(|e| anyhow!("spawn_blocking failed: {e}"))?
    }

    /// Call `poll_for_token(config, device_data)` in the script.
    pub async fn poll_for_token(&self, device_data: Map) -> Result<Map> {
        let engine = self.engine.clone();
        let ast = self.ast.clone();
        let config = self.config_map();

        tokio::task::spawn_blocking(move || -> Result<Map> {
            let mut scope = Scope::new();
            let data_dyn = Dynamic::from_map(device_data);
            let result: Dynamic = engine
                .call_fn(
                    &mut scope,
                    &ast,
                    "poll_for_token",
                    (config, data_dyn),
                )
                .map_err(|e| anyhow!("poll_for_token failed: {e}"))?;
            result
                .try_cast::<Map>()
                .ok_or_else(|| anyhow!("poll_for_token must return a Map"))
        })
        .await
        .map_err(|e| anyhow!("spawn_blocking failed: {e}"))?
    }

    /// Call `needs_refresh(tokens)` in the script.
    pub async fn needs_refresh(&self, tokens: Map) -> Result<bool> {
        let engine = self.engine.clone();
        let ast = self.ast.clone();

        tokio::task::spawn_blocking(move || -> Result<bool> {
            let mut scope = Scope::new();
            let tokens_dyn = Dynamic::from_map(tokens);
            let result: Dynamic = engine
                .call_fn(&mut scope, &ast, "needs_refresh", (tokens_dyn,))
                .map_err(|e| anyhow!("needs_refresh failed: {e}"))?;
            result
                .as_bool()
                .map_err(|_| anyhow!("needs_refresh must return a bool"))
        })
        .await
        .map_err(|e| anyhow!("spawn_blocking failed: {e}"))?
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
