//! Scripted OAuth NAPI bridge — PROV-087.
//!
//! Pure-Rust helpers that the `codelet-napi` crate re-exports as
//! `#[napi]` async functions. Keeping the logic here means that (a) it
//! is exercised by `cargo test -p codelet-providers` without pulling in
//! the NAPI build, and (b) the NAPI layer stays a thin, declarative
//! wrapper.
//!
//! The public entry points correspond 1:1 to the NAPI bindings:
//!
//! * [`custom_oauth_authorize_json`] — run the script's `auth_start`
//!   and return the authorization payload (URL + PKCE verifier +
//!   state). Opening the browser and hosting the loopback callback
//!   server is the TypeScript layer's responsibility — this split
//!   matches the built-in claude/codex/copilot flows where the TUI
//!   drives the browser and passes the captured code back into Rust
//!   via [`custom_oauth_exchange_json`].
//! * [`custom_oauth_exchange_json`]  — call `auth_exchange` /
//!   `exchange_code` and persist the resulting tokens.
//! * [`custom_oauth_needs_refresh_json`] — read the stored tokens and
//!   call `auth_needs_refresh` / `needs_refresh`.
//! * [`custom_oauth_refresh_json`] — call `auth_refresh` /
//!   `refresh_token` and replace the stored tokens.
//! * [`custom_oauth_clear`] / [`custom_oauth_clear_sync`] — remove the
//!   stored tokens for a provider.
//!
//! Tokens are persisted via the shared
//! [`CredentialStore<serde_json::Value>`](crate::oauth::CredentialStore)
//! wrapper (0600 permissions on Unix, pretty JSON). The file layout
//! mirrors claude/codex/copilot — `<fspec_home()>/oauth/<provider>.json`
//! — but the value is a dynamic JSON object because scripts can
//! produce arbitrary token maps.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use rhai::{Dynamic, Map};

use super::credential_store::CredentialStore;
use super::json_convert::{dynamic_to_json_value, json_value_to_dynamic};
use super::script_provider::{ScriptProviderConfig, ScriptedOAuthProvider};
use super::script_provider_aliases::{
    auth_exchange_or_legacy, auth_needs_refresh_or_legacy, auth_refresh_or_legacy,
    auth_start_or_legacy,
};

/// Which login implementation the TypeScript `/login` dispatcher should
/// use for a given provider name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginImplementation {
    /// Use the hard-coded NAPI binding for `claude`, `codex`, or
    /// `copilot`. The wrapped string is the provider name.
    BuiltIn(String),
    /// Route to `custom_oauth_*` because a Rhai script shadows this
    /// provider name. The wrapped string is the provider name.
    Custom(String),
}

/// The known built-in OAuth provider names. Any other provider name is
/// treated as custom.
pub const BUILTIN_LOGIN_PROVIDERS: &[&str] = &["claude", "codex", "copilot"];

/// Resolve which OAuth implementation should back `/login <provider>`.
pub fn resolve_login_implementation(provider_name: &str) -> LoginImplementation {
    if has_shadow_config(provider_name) {
        return LoginImplementation::Custom(provider_name.to_string());
    }
    if BUILTIN_LOGIN_PROVIDERS.contains(&provider_name) {
        LoginImplementation::BuiltIn(provider_name.to_string())
    } else {
        LoginImplementation::Custom(provider_name.to_string())
    }
}

/// Check whether a shadow config exists for `provider_name`.
fn has_shadow_config(provider_name: &str) -> bool {
    providers_dir()
        .join(format!("{provider_name}.json"))
        .exists()
}

/// Directory where discovered custom provider configs live.
///
/// Custom configs are siblings of the credentials directory:
/// `<fspec_home>/../providers/<name>.json` per PROV-085.
fn providers_dir() -> PathBuf {
    crate::oauth::fspec_home()
        .parent()
        .map(|p| p.join("providers"))
        .unwrap_or_else(|| PathBuf::from("providers"))
}

/// Absolute path where tokens for `provider_name` are stored. Does not
/// touch disk.
pub fn custom_oauth_store_path(provider_name: &str) -> PathBuf {
    crate::oauth::fspec_home()
        .join("oauth")
        .join(format!("{provider_name}.json"))
}

/// Typed [`CredentialStore`] scoped to a given provider's scripted-OAuth
/// token file. Centralises file I/O so scripted providers share the
/// same on-disk contract as claude/codex/copilot.
fn token_store(provider_name: &str) -> CredentialStore<serde_json::Value> {
    CredentialStore::new(custom_oauth_store_path(provider_name))
}

/// Read the stored tokens for `provider_name` as a Rhai `Map`.
///
/// Returns `Ok(None)` when the file does not exist.
pub fn read_stored_tokens(provider_name: &str) -> Result<Option<Map>> {
    let Some(value) = token_store(provider_name)
        .read_sync()
        .with_context(|| format!("read tokens for '{provider_name}'"))?
    else {
        return Ok(None);
    };
    let dy = json_value_to_dynamic(&value);
    let map = dy
        .try_cast::<Map>()
        .ok_or_else(|| anyhow!("stored tokens must be a JSON object"))?;
    Ok(Some(map))
}

/// Persist `tokens` to disk under `provider_name`.
///
/// Delegates the write + parent-dir creation to
/// [`CredentialStore::write_sync`], then enforces Unix `0o600` for
/// parity with Claude/Codex/Copilot credentials.
pub fn write_stored_tokens(provider_name: &str, tokens: &Map) -> Result<()> {
    let store = token_store(provider_name);
    let value = dynamic_to_json_value(&Dynamic::from_map(tokens.clone()));
    store
        .write_sync(&value)
        .with_context(|| format!("write tokens for '{provider_name}'"))?;
    set_mode_0600(store.path())?;
    Ok(())
}

/// Blocking implementation of the NAPI `custom_oauth_clear` binding.
/// Idempotent.
pub fn custom_oauth_clear_sync(provider_name: &str) -> Result<()> {
    let path = custom_oauth_store_path(provider_name);
    if path.exists() {
        std::fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
    }
    Ok(())
}

/// Async wrapper for the NAPI `custom_oauth_clear` binding.
pub async fn custom_oauth_clear(provider_name: &str) -> Result<()> {
    let name = provider_name.to_string();
    tokio::task::spawn_blocking(move || custom_oauth_clear_sync(&name))
        .await
        .map_err(|e| anyhow!("spawn_blocking failed: {e}"))?
}

/// NAPI helper: run the script's `auth_start` and return the
/// authorization metadata `{url, pkce_verifier, state}`.
///
/// The caller is expected to drive the loopback callback server and
/// produce `code`/`state` pairs on its own — the TypeScript TUI owns
/// the browser + HTTP listener runtime so this layer stays
/// deterministic and testable.
pub async fn custom_oauth_authorize_start(provider: &ScriptedOAuthProvider) -> Result<Map> {
    auth_start_or_legacy(provider).await
}

/// NAPI helper: exchange `code` + `verifier` for tokens and persist
/// them under `provider_name`.
pub async fn custom_oauth_exchange(
    provider: &ScriptedOAuthProvider,
    provider_name: &str,
    code: &str,
    verifier: &str,
) -> Result<Map> {
    let tokens = auth_exchange_or_legacy(provider, code, verifier).await?;
    write_stored_tokens(provider_name, &tokens)?;
    Ok(tokens)
}

/// NAPI helper: consult `auth_needs_refresh` (falling back to
/// `needs_refresh`) against the currently stored tokens. Returns
/// `false` when no tokens are stored.
pub async fn custom_oauth_needs_refresh(
    provider: &ScriptedOAuthProvider,
    provider_name: &str,
) -> Result<bool> {
    match read_stored_tokens(provider_name)? {
        Some(tokens) => auth_needs_refresh_or_legacy(provider, tokens).await,
        None => Ok(false),
    }
}

/// NAPI helper: invoke `auth_refresh` (falling back to `refresh_token`)
/// and persist the result.
pub async fn custom_oauth_refresh(
    provider: &ScriptedOAuthProvider,
    provider_name: &str,
) -> Result<Map> {
    let current = read_stored_tokens(provider_name)?
        .ok_or_else(|| anyhow!("no stored tokens for provider '{provider_name}'"))?;
    let refreshed = auth_refresh_or_legacy(provider, current).await?;
    write_stored_tokens(provider_name, &refreshed)?;
    Ok(refreshed)
}

/// Serialize a Rhai `Map` to a JSON string.
pub fn map_to_json_string(map: &Map) -> Result<String> {
    let value = dynamic_to_json_value(&Dynamic::from_map(map.clone()));
    serde_json::to_string(&value).map_err(|e| anyhow!("serialize map: {e}"))
}

/// Load a `ScriptedOAuthProvider` from the discovered config file.
///
/// Copies every available field from [`crate::custom::ProviderConfig`]
/// so scripts can inspect auth URLs, client IDs, scopes, etc. via the
/// config map passed to their callbacks — fixes the prior "hardcode
/// None" behaviour that silently dropped `token_url`, `client_id`, etc.
pub fn load_scripted_provider_for(provider_name: &str) -> Result<ScriptedOAuthProvider> {
    use crate::custom::ProviderConfig;
    let dir = providers_dir();
    let config_path = dir.join(format!("{provider_name}.json"));
    let cfg = ProviderConfig::from_file(&config_path)
        .with_context(|| format!("load provider config {}", config_path.display()))?;
    let script_abs = resolve_script_path(&dir, &cfg.script);
    let script_config = script_config_from(&cfg, script_abs.to_string_lossy().into_owned());
    ScriptedOAuthProvider::load(&script_abs, script_config)
        .with_context(|| format!("load script {provider_name}"))
}

/// Resolve a script path against the providers directory when it is
/// relative. Absolute paths are returned as-is.
fn resolve_script_path(dir: &Path, script: &str) -> PathBuf {
    let p = Path::new(script);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        dir.join(p)
    }
}

/// Build a [`ScriptProviderConfig`] from the loaded [`ProviderConfig`],
/// copying every field the script callbacks may need.
fn script_config_from(
    cfg: &crate::custom::ProviderConfig,
    script_abs: String,
) -> ScriptProviderConfig {
    use crate::custom::AuthConfig;
    let (token_url, client_id, scopes) = match &cfg.auth {
        AuthConfig::OauthPkce {
            token_url,
            client_id,
            scopes,
            ..
        }
        | AuthConfig::OauthDeviceCode {
            token_url,
            client_id,
            scopes,
            ..
        } => (
            Some(token_url.clone()),
            Some(client_id.clone()),
            scopes.clone(),
        ),
        AuthConfig::Bearer { .. } | AuthConfig::ApiKeyHeader { .. } | AuthConfig::Custom { .. } => {
            (None, None, None)
        }
    };
    ScriptProviderConfig {
        name: cfg.name.clone(),
        display_name: cfg.display_name.clone(),
        script: script_abs,
        auth_url: Some(cfg.base_url.clone()),
        token_url,
        client_id,
        redirect_uri: None,
        scopes,
        flow: None,
        credential_file: None,
    }
}

/// High-level wrapper used by the NAPI layer: loads the provider, calls
/// `auth_start`, returns the authorization payload as a JSON string.
pub async fn custom_oauth_authorize_json(provider_name: &str) -> Result<String> {
    let provider = load_scripted_provider_for(provider_name)?;
    let map = custom_oauth_authorize_start(&provider).await?;
    map_to_json_string(&map)
}

/// High-level wrapper: loads the provider, calls `auth_exchange`,
/// persists tokens, returns them as a JSON string.
pub async fn custom_oauth_exchange_json(
    provider_name: &str,
    code: &str,
    verifier: &str,
) -> Result<String> {
    let provider = load_scripted_provider_for(provider_name)?;
    let tokens = custom_oauth_exchange(&provider, provider_name, code, verifier).await?;
    map_to_json_string(&tokens)
}

/// High-level wrapper: loads the provider, calls `auth_needs_refresh`
/// against the currently stored tokens.
pub async fn custom_oauth_needs_refresh_json(provider_name: &str) -> Result<bool> {
    let provider = load_scripted_provider_for(provider_name)?;
    custom_oauth_needs_refresh(&provider, provider_name).await
}

/// High-level wrapper: loads the provider, calls `auth_refresh`,
/// persists the result, returns it as a JSON string.
pub async fn custom_oauth_refresh_json(provider_name: &str) -> Result<String> {
    let provider = load_scripted_provider_for(provider_name)?;
    let tokens = custom_oauth_refresh(&provider, provider_name).await?;
    map_to_json_string(&tokens)
}

#[cfg(unix)]
fn set_mode_0600(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)
        .with_context(|| format!("metadata {}", path.display()))?
        .permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(path, perms).with_context(|| format!("chmod {}", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_mode_0600(_path: &Path) -> Result<()> {
    Ok(())
}
