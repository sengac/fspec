use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use super::auth_lifecycle::{normalized_home, LoginAttempt};
use super::auth_storage::{self, AuthError};
use super::codex_browser_authorization::BROWSER_REDIRECT_URI;
use super::codex_browser_completion::BrowserLoginStart;
use super::codex_oauth::{
    build_authorize_url, generate_pkce, generate_state, CODEX_ISSUER, OAUTH_TIMEOUT_MS,
};

pub(super) struct Pending {
    pub attempt: LoginAttempt,
    pub deadline: tokio::time::Instant,
    pub issuer: String,
    verifier: String,
    expiration: tokio::task::JoinHandle<()>,
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.expiration.abort();
    }
}

static PENDING: LazyLock<Mutex<HashMap<PathBuf, Pending>>> = LazyLock::new(Mutex::default);

pub(super) fn cancel_pending(home: &Path) -> Result<(), AuthError> {
    let removed = PENDING
        .lock()
        .map_err(|error| {
            AuthError::OAuth(format!("Browser pending login store poisoned: {error}"))
        })?
        .remove(&normalized_home(home)?);
    drop(removed);
    Ok(())
}

fn expire_pending(home: &Path, id: &str) {
    let removed = {
        let mut sessions = match PENDING.lock() {
            Ok(sessions) => sessions,
            Err(error) => {
                tracing::error!(%error, "Cannot release expired Codex browser login; refusing completion");
                return;
            }
        };
        if sessions
            .get(home)
            .is_some_and(|pending| pending.attempt.id() == id)
        {
            sessions.remove(home)
        } else {
            None
        }
    };
    drop(removed);
}

pub(super) async fn start() -> Result<BrowserLoginStart, AuthError> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(OAUTH_TIMEOUT_MS);
    let attempt = LoginAttempt::begin_until(deadline).await?;
    let pkce = generate_pkce();
    let authorize_url = build_authorize_url(BROWSER_REDIRECT_URI, &pkce, &generate_state());
    let issuer = CODEX_ISSUER.to_owned();
    #[cfg(feature = "test-support")]
    let issuer = match super::oauth_test_support::config() {
        Some(config) => config.issuer,
        None => issuer,
    };
    let home = attempt.home().to_path_buf();
    let expiration_home = home.clone();
    let id = attempt.id().to_owned();
    let mut sessions = PENDING.lock().map_err(|error| {
        AuthError::OAuth(format!("Browser pending login store poisoned: {error}"))
    })?;
    attempt.ensure_current()?;
    let expiration = tokio::spawn(async move {
        tokio::time::sleep_until(deadline).await;
        expire_pending(&expiration_home, &id);
    });
    let previous = sessions.insert(
        home,
        Pending {
            attempt,
            deadline,
            issuer,
            verifier: pkce.verifier.clone(),
            expiration,
        },
    );
    drop(sessions);
    drop(previous);
    Ok(BrowserLoginStart {
        authorize_url,
        pkce_verifier: pkce.verifier,
    })
}

pub(super) fn take(verifier: &str) -> Result<Pending, AuthError> {
    let home = normalized_home(&auth_storage::home())?;
    let pending = {
        let mut sessions = PENDING.lock().map_err(|error| {
            AuthError::OAuth(format!("Browser pending login store poisoned: {error}"))
        })?;
        if sessions
            .get(&home)
            .is_none_or(|pending| pending.verifier != verifier)
        {
            return Err(AuthError::Changed);
        }
        sessions.remove(&home).ok_or(AuthError::Changed)?
    };
    pending.expiration.abort();
    pending.attempt.ensure_current()?;
    Ok(pending)
}
