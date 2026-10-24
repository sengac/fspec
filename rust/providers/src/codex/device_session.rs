use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use super::auth_lifecycle::{normalized_home, LoginAttempt};
use super::auth_storage::{self, AuthError};
use super::codex_auth::CodexTokens;
use super::codex_device_auth::{self, DeviceAuthConfig, DeviceCodeResponse};
use super::codex_oauth::{CODEX_ISSUER, OAUTH_TIMEOUT_MS};

#[cfg(all(test, feature = "test-support"))]
#[path = "../../tests/prov159_support/device_session.rs"]
mod tests;

struct Pending {
    attempt: LoginAttempt,
    code: DeviceCodeResponse,
    issuer: String,
    started: tokio::time::Instant,
    expiration: tokio::task::JoinHandle<()>,
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.expiration.abort();
    }
}

static PENDING: LazyLock<Mutex<HashMap<PathBuf, Pending>>> = LazyLock::new(Mutex::default);

pub(super) fn cancel_pending(home: &Path) {
    let home = normalized_home(home);
    let removed = PENDING
        .lock()
        .ok()
        .and_then(|mut pending| pending.remove(&home));
    drop(removed);
}

fn expire_pending(home: &Path, id: &str) {
    let removed = PENDING.lock().ok().and_then(|mut sessions| {
        if sessions
            .get(home)
            .is_some_and(|pending| pending.attempt.id() == id)
        {
            sessions.remove(home)
        } else {
            None
        }
    });
    drop(removed);
}

pub async fn start_device_login() -> Result<DeviceCodeResponse, AuthError> {
    let started = tokio::time::Instant::now();
    let deadline = started + std::time::Duration::from_millis(OAUTH_TIMEOUT_MS);
    let attempt = LoginAttempt::begin_until(deadline).await?;
    let issuer = CODEX_ISSUER.to_owned();
    #[cfg(feature = "test-support")]
    let issuer = super::oauth_test_support::config().map_or(issuer, |config| config.issuer);
    let code = tokio::select! {
        biased;
        () = attempt.cancelled() => return Err(AuthError::Changed),
        result = tokio::time::timeout_at(deadline, codex_device_auth::request_device_code(&issuer)) => {
            result.map_err(|_| AuthError::OAuth("Device login timed out".into()))?
                .map_err(|error| AuthError::OAuth(error.to_string()))?
        }
    };
    let home = attempt.home().to_path_buf();
    let expiration_home = home.clone();
    let id = attempt.id().to_owned();
    let mut sessions = PENDING.lock().map_err(|_| AuthError::Changed)?;
    attempt.ensure_current()?;
    let expiration = tokio::spawn(async move {
        tokio::time::sleep_until(deadline).await;
        expire_pending(&expiration_home, &id);
    });
    let previous = sessions.insert(
        home,
        Pending {
            attempt,
            code: code.clone(),
            issuer,
            started,
            expiration,
        },
    );
    drop(sessions);
    drop(previous);
    Ok(code)
}

pub async fn complete_device_login(device_auth_id: &str) -> Result<CodexTokens, AuthError> {
    let pending = {
        let home = normalized_home(&auth_storage::home());
        let mut sessions = PENDING.lock().map_err(|_| AuthError::Changed)?;
        if sessions
            .get(&home)
            .is_none_or(|pending| pending.code.device_auth_id != device_auth_id)
        {
            return Err(AuthError::Changed);
        }
        sessions.remove(&home).ok_or(AuthError::Changed)?
    };
    pending.expiration.abort();
    let config = DeviceAuthConfig {
        issuer_url: pending.issuer.clone(),
        timeout_ms: OAUTH_TIMEOUT_MS,
        poll_interval_override_ms: None,
        slow_down_increment_override_ms: None,
        display_fn: None,
    };
    let deadline = pending.started + std::time::Duration::from_millis(OAUTH_TIMEOUT_MS);
    tokio::select! {
        biased;
        () = pending.attempt.cancelled() => Err(AuthError::Changed),
        result = tokio::time::timeout_at(deadline,
            codex_device_auth::finish(&config, &pending.code, &pending.attempt)) => {
            result.map_err(|_| AuthError::OAuth("Device login timed out".into()))?
                .map_err(|error| AuthError::OAuth(error.to_string()))
        }
    }
}
