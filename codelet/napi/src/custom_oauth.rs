//! NAPI Bindings for Scripted OAuth Providers — PROV-087.
//!
//! Thin wrapper around
//! `codelet_providers::oauth::custom_oauth::*_json` — the business logic
//! (script dispatch, CredentialStore round-trip, built-in vs custom
//! routing) lives in `codelet-providers` where it can be unit-tested
//! without the NAPI build. This file exposes the five async functions
//! the TypeScript `/login` dispatcher calls when a Rhai shadow config
//! exists for the requested provider name.

use codelet_providers::oauth::custom_oauth::{
    custom_oauth_authorize_json, custom_oauth_clear as core_clear,
    custom_oauth_exchange_json, custom_oauth_needs_refresh_json,
    custom_oauth_refresh_json, custom_oauth_store_path,
};
use codelet_providers::oauth::custom_oauth_device_json::{
    custom_oauth_device_poll_json, custom_oauth_device_start_json,
};
use napi::bindgen_prelude::*;

/// Authorization metadata returned by `custom_oauth_authorize`: a
/// JSON-serialized map of whatever the script's `auth_start`
/// (or legacy `build_authorization_request`) returned — typically
/// `{url, pkce_verifier, state, ...}`.
#[napi(object)]
pub struct NapiCustomAuthorizeResult {
    pub payload_json: String,
}

/// JSON envelope for token maps passed back and forth across the NAPI
/// boundary. We serialize Rhai `Map` → JSON string so we can expose the
/// full shape (including custom fields produced by user scripts)
/// without pinning it to a fixed struct.
#[napi(object)]
pub struct NapiCustomTokens {
    pub tokens_json: String,
}

/// Invoke the scripted provider's `auth_start` (with fallback to the
/// legacy `build_authorization_request`) and return the authorization
/// payload as a JSON string. The TypeScript TUI is responsible for
/// opening the browser, hosting the loopback callback server, and
/// then calling `custom_oauth_exchange` with the captured code.
#[napi]
pub async fn custom_oauth_authorize(
    provider_name: String,
) -> Result<NapiCustomAuthorizeResult> {
    let payload_json = custom_oauth_authorize_json(&provider_name).await.map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("custom_oauth_authorize('{provider_name}') failed: {e}"),
        )
    })?;
    Ok(NapiCustomAuthorizeResult { payload_json })
}

/// Exchange the captured authorization `code` + PKCE `verifier` for
/// tokens and persist them under `provider_name`.
#[napi]
pub async fn custom_oauth_exchange(
    provider_name: String,
    code: String,
    verifier: String,
) -> Result<NapiCustomTokens> {
    let tokens_json = custom_oauth_exchange_json(&provider_name, &code, &verifier)
        .await
        .map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("custom_oauth_exchange('{provider_name}') failed: {e}"),
            )
        })?;
    Ok(NapiCustomTokens { tokens_json })
}

/// Return `true` when the stored tokens for `provider_name` need a
/// refresh (per the script's `auth_needs_refresh` / `needs_refresh`).
#[napi]
pub async fn custom_oauth_needs_refresh(provider_name: String) -> Result<bool> {
    custom_oauth_needs_refresh_json(&provider_name)
        .await
        .map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("custom_oauth_needs_refresh('{provider_name}') failed: {e}"),
            )
        })
}

/// Refresh the stored tokens for `provider_name` by invoking the
/// script's `auth_refresh` / `refresh_token`, then persist the result.
#[napi]
pub async fn custom_oauth_refresh(provider_name: String) -> Result<NapiCustomTokens> {
    let tokens_json = custom_oauth_refresh_json(&provider_name).await.map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("custom_oauth_refresh('{provider_name}') failed: {e}"),
        )
    })?;
    Ok(NapiCustomTokens { tokens_json })
}

/// Remove the stored tokens for `provider_name`. Idempotent.
#[napi]
pub async fn custom_oauth_clear(provider_name: String) -> Result<()> {
    core_clear(&provider_name).await.map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("custom_oauth_clear('{provider_name}') failed: {e}"),
        )
    })
}

/// Return the stored tokens for `provider_name`. `None` if not stored.
#[napi]
pub async fn custom_oauth_get_tokens(
    provider_name: String,
) -> Result<Option<NapiCustomTokens>> {
    let path = custom_oauth_store_path(&provider_name);
    if !path.exists() {
        return Ok(None);
    }
    let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
        Error::new(
            Status::GenericFailure,
            format!("read tokens for '{provider_name}': {e}"),
        )
    })?;
    Ok(Some(NapiCustomTokens {
        tokens_json: content,
    }))
}

/// PROV-088: start the scripted device-code flow. Returns the
/// authorization map as JSON — typically `{device_code, user_code,
/// verification_uri, interval}`.
#[napi]
pub async fn custom_oauth_device_start(
    provider_name: String,
) -> Result<NapiCustomAuthorizeResult> {
    let payload_json = custom_oauth_device_start_json(&provider_name)
        .await
        .map_err(|e| {
            Error::new(
                Status::GenericFailure,
                format!("custom_oauth_device_start('{provider_name}') failed: {e}"),
            )
        })?;
    Ok(NapiCustomAuthorizeResult { payload_json })
}

/// PROV-088: poll the scripted device-code flow. `device_data_json` is
/// the JSON-encoded map returned by `custom_oauth_device_start`. The
/// return value is a JSON-encoded `{status, ...}` map. Tokens are
/// automatically persisted when the status is `"success"`.
#[napi]
pub async fn custom_oauth_device_poll(
    provider_name: String,
    device_data_json: String,
) -> Result<NapiCustomTokens> {
    let tokens_json =
        custom_oauth_device_poll_json(&provider_name, &device_data_json)
            .await
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("custom_oauth_device_poll('{provider_name}') failed: {e}"),
                )
            })?;
    Ok(NapiCustomTokens { tokens_json })
}
