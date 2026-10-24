use super::auth_storage::{self, AuthError};
use super::codex_auth::CodexTokens;
use super::refreshing_client::{token_expiry, TokenState, DEFAULT_EXPIRY_SECS, EXPIRY_BUFFER_SECS};
use base64::Engine;
use rig::http_client::Error;
use serde_json::Value;
use std::path::PathBuf;
use tokio::time::Instant;

#[derive(Debug, Clone)]
pub struct Binding {
    pub(super) home: PathBuf,
    generation: Value,
    lifecycle: String,
    freshness: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Identity {
    home: PathBuf,
    lifecycle: String,
}

impl Binding {
    pub(super) fn identity(&self) -> Identity {
        Identity {
            home: self.home.clone(),
            lifecycle: self.lifecycle.clone(),
        }
    }
}

fn failure(error: impl std::fmt::Display) -> Error {
    Error::Instance(error.to_string().into())
}

fn freshness(document: &Value) -> Value {
    serde_json::json!([document["last_refresh"], document["fspec_expires_at"]])
}

pub(super) fn bind(access: &str, refresh: &str) -> Result<Option<Binding>, AuthError> {
    let home = auth_storage::home();
    auth_storage::transaction(&home, || {
        let document = auth_storage::document(&home)?;
        let mut lifecycle = auth_storage::generation(&home)?;
        if document["tokens"]["access_token"] != access
            || document["tokens"]["refresh_token"] != refresh
        {
            return if document["tokens"].is_null()
                && document["fspec_generation"].is_null()
                && lifecycle.is_empty()
            {
                Ok(None)
            } else {
                Err(AuthError::Changed)
            };
        }
        if lifecycle.is_empty() {
            lifecycle = uuid::Uuid::new_v4().to_string();
            auth_storage::claim_generation(&home, &lifecycle)?;
        }
        Ok(Some(Binding {
            home: home.clone(),
            generation: document["fspec_generation"].clone(),
            lifecycle,
            freshness: freshness(&document),
        }))
    })
}

pub(super) fn stored_lifetime(access: &str) -> Result<u64, AuthError> {
    let document = auth_storage::document(&auth_storage::home())?;
    if document["tokens"]["access_token"] != access {
        return Err(AuthError::Changed);
    }
    Ok(lifetime(&document, access))
}

fn jwt_expiry(access: &str) -> Option<i64> {
    access
        .split('.')
        .nth(1)
        .and_then(|payload| {
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(payload)
                .ok()
        })
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|claims| claims["exp"].as_i64())
}

fn remaining(expiry: i64) -> u64 {
    u64::try_from(expiry.saturating_sub(chrono::Utc::now().timestamp())).unwrap_or(0)
}

fn lifetime(document: &Value, access: &str) -> u64 {
    let expiry = match (document["fspec_expires_at"].as_i64(), jwt_expiry(access)) {
        (Some(metadata), Some(claim)) => Some(metadata.min(claim)),
        (metadata, claim) => metadata.or(claim),
    };
    expiry
        .or_else(|| {
            document["last_refresh"]
                .as_str()
                .and_then(|timestamp| chrono::DateTime::parse_from_rfc3339(timestamp).ok())
                .map(|timestamp| {
                    timestamp
                        .timestamp()
                        .saturating_add(DEFAULT_EXPIRY_SECS as i64)
                })
        })
        .map(remaining)
        .unwrap_or(0)
}

pub(super) fn validate(state: &TokenState) -> Result<(), AuthError> {
    if let Some(binding) = state.binding.as_ref().map_err(|_| AuthError::Changed)? {
        current(state, binding)?;
    }
    Ok(())
}

pub(super) fn current(state: &TokenState, binding: &Binding) -> Result<Value, AuthError> {
    let document = auth_storage::document(&binding.home)?;
    if auth_storage::generation(&binding.home)? != binding.lifecycle
        || (!document["fspec_generation"].is_null()
            && document["fspec_generation"] != binding.generation)
        || document["tokens"]["account_id"] != state.account_id
    {
        return Err(AuthError::Changed);
    }
    Ok(document)
}

pub(super) fn reload(state: &mut TokenState, document: &Value) -> Result<(), AuthError> {
    let tokens: CodexTokens = serde_json::from_value(document["tokens"].clone())?;
    let freshness = freshness(document);
    let renewed = state
        .binding
        .as_ref()
        .ok()
        .and_then(Option::as_ref)
        .is_some_and(|binding| binding.freshness != freshness);
    if renewed
        || state.access_token != tokens.access_token
        || state.refresh_token != tokens.refresh_token
    {
        state.access_token = tokens.access_token;
        state.refresh_token = tokens.refresh_token;
        state.expires_at = token_expiry(lifetime(document, &state.access_token));
        if let Ok(Some(binding)) = &mut state.binding {
            binding.freshness = freshness;
        }
    }
    Ok(())
}

pub(super) async fn ensure(state: &mut TokenState) -> Result<(), Error> {
    ensure_inner(state).await
}

async fn ensure_inner(state: &mut TokenState) -> Result<(), Error> {
    let binding = state.binding.clone().map_err(failure)?;
    if let Some(binding) = &binding {
        let document = current(state, binding).map_err(failure)?;
        reload(state, &document).map_err(failure)?;
    }
    if Instant::now() + std::time::Duration::from_secs(EXPIRY_BUFFER_SECS) < state.expires_at {
        return Ok(());
    }
    let _rotation = if let Some(binding) = &binding {
        let lock = auth_storage::lock(&binding.home, ".fspec-refresh.lock").map_err(failure)?;
        loop {
            match lock.try_lock() {
                Ok(()) => break,
                Err(std::fs::TryLockError::WouldBlock) => {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await
                }
                Err(error) => return Err(failure(error)),
            }
        }
        let document = current(state, binding).map_err(failure)?;
        reload(state, &document).map_err(failure)?;
        if Instant::now() + std::time::Duration::from_secs(EXPIRY_BUFFER_SECS) < state.expires_at {
            return Ok(());
        }
        Some(lock)
    } else {
        None
    };
    let previous = state.refresh_token.clone();
    let response = super::recovery_auth::refresh(&state.issuer_url, &previous).await?;
    if let Some(binding) = &binding {
        auth_storage::transaction_async(&binding.home, || {
            persist(state, Some(binding), &previous, &response)
        })
        .await
        .map_err(failure)?;
    } else {
        persist(state, None, &previous, &response).map_err(failure)?;
    }
    state.access_token = response.access_token;
    state.refresh_token = response.refresh_token;
    state.expires_at = token_expiry(response.expires_in.unwrap_or(DEFAULT_EXPIRY_SECS));
    Ok(())
}

pub(super) fn persist(
    state: &TokenState,
    binding: Option<&Binding>,
    previous: &str,
    response: &super::codex_oauth::TokenRefreshResponse,
) -> Result<(), AuthError> {
    if super::codex_oauth::extract_account_id(
        Some(&response.id_token),
        Some(&response.access_token),
    )
    .is_some_and(|account| account != state.account_id)
    {
        return Err(AuthError::Changed);
    }
    if let Some(binding) = &binding {
        let mut document = current(state, binding)?;
        if document["tokens"]["refresh_token"] != previous {
            return Err(AuthError::Changed);
        }
        document["tokens"] = serde_json::to_value(CodexTokens {
            id_token: response.id_token.clone(),
            access_token: response.access_token.clone(),
            refresh_token: response.refresh_token.clone(),
            account_id: state.account_id.clone(),
        })?;
        document["last_refresh"] = chrono::Utc::now().to_rfc3339().into();
        document["fspec_expires_at"] = chrono::Utc::now()
            .timestamp()
            .saturating_add(
                response
                    .expires_in
                    .unwrap_or(DEFAULT_EXPIRY_SECS)
                    .min(i64::MAX as u64) as i64,
            )
            .into();
        auth_storage::replace(&binding.home, &document)?;
    }
    Ok(())
}
