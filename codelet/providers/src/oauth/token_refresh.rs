//! Shared Token Refresh Logic (PROV-060)
//!
//! Double-check locking pattern for OAuth token refresh, extracted from
//! the nearly identical implementations in `codex/refreshing_client.rs`
//! and `claude_refreshing_client.rs`.

use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::time::Instant;

/// Expiry buffer in seconds — refresh token this many seconds before
/// actual expiry to prevent edge cases where token expires between
/// check and actual API call.
pub const EXPIRY_BUFFER_SECS: u64 = 30;

/// Default token expiry in seconds when `expires_in` is not provided.
pub const DEFAULT_EXPIRY_SECS: u64 = 3600;

/// Generic in-memory token state shared across concurrent requests.
///
/// The concrete provider-specific state (Codex account_id, Claude
/// token_endpoint_base, etc.) lives in the `extra` field.
#[derive(Debug, Clone)]
pub struct TokenState<E: Clone + Send + Sync = ()> {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Instant,
    /// Provider-specific extra state (e.g. issuer_url, account_id).
    pub extra: E,
}

impl<E: Clone + Send + Sync> TokenState<E> {
    /// Check if the token is expired (including expiry buffer).
    pub fn is_expired(&self) -> bool {
        let buffer = std::time::Duration::from_secs(EXPIRY_BUFFER_SECS);
        Instant::now() + buffer >= self.expires_at
    }
}

/// Ensure the token is fresh using double-check locking.
///
/// 1. Read lock — check expiry. Return immediately if still valid.
/// 2. Write lock — re-check expiry (another task may have refreshed).
/// 3. Call `refresh_fn` to actually refresh the token.
/// 4. Call `persist_fn` outside the lock for best-effort persistence.
///
/// Returns `Err` only if `refresh_fn` fails.
pub async fn ensure_fresh_token<E, Fut>(
    token_state: &Arc<RwLock<TokenState<E>>>,
    refresh_fn: impl FnOnce(TokenState<E>) -> Fut,
    persist_fn: impl FnOnce(TokenState<E>),
) -> Result<(), rig::http_client::Error>
where
    E: Clone + Send + Sync,
    Fut: std::future::Future<Output = Result<TokenState<E>, String>>,
{
    let buffer = std::time::Duration::from_secs(EXPIRY_BUFFER_SECS);

    // Read lock: check if expired
    {
        let state = token_state.read().await;
        if Instant::now() + buffer < state.expires_at {
            return Ok(()); // Token still valid
        }
    }
    // Read lock dropped

    // Write lock: double-check and refresh if still expired
    let persist_data = {
        let mut state = token_state.write().await;
        // Re-check under write lock (another task may have refreshed)
        if Instant::now() + buffer < state.expires_at {
            return Ok(()); // Another task refreshed it
        }

        let old_state = state.clone();
        let new_state = refresh_fn(old_state)
            .await
            .map_err(|e| rig::http_client::Error::Instance(e.into()))?;

        // Update in-memory state
        *state = new_state.clone();

        Some(new_state)
    };
    // Write lock dropped here

    // Persist outside the lock (best-effort)
    if let Some(new_state) = persist_data {
        persist_fn(new_state);
    }

    Ok(())
}
