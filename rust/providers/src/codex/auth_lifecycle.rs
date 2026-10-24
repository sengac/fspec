use super::auth_storage::{self, AuthError};
use super::codex_auth::CodexTokens;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use tokio::sync::watch;
use tokio::time::Instant;

struct Active {
    id: String,
    cancel: watch::Sender<bool>,
    done: watch::Receiver<bool>,
}

static ATTEMPTS: LazyLock<Mutex<HashMap<PathBuf, Active>>> = LazyLock::new(Mutex::default);

pub struct LoginAttempt {
    home: PathBuf,
    id: String,
    snapshot: Value,
    deadline: Instant,
    cancel: watch::Receiver<bool>,
    _done: watch::Sender<bool>,
}

pub(super) fn normalized_home(home: &Path) -> PathBuf {
    home.canonicalize().unwrap_or_else(|_| home.to_path_buf())
}

pub(crate) fn cancel_at(home: &Path) {
    let home = normalized_home(home);
    if let Ok(attempts) = ATTEMPTS.lock() {
        if let Some(active) = attempts.get(&home) {
            let _ = active.cancel.send(true);
        }
    }
    super::device_session::cancel_pending(&home);
}

pub fn cancel_login() {
    cancel_at(&auth_storage::home());
}

fn snapshot(home: &Path) -> Result<Value, AuthError> {
    let document = auth_storage::document(home)?;
    Ok(serde_json::json!({
        "generation": document.get("fspec_generation"),
        "tokens": document.get("tokens"),
    }))
}

impl LoginAttempt {
    pub async fn begin() -> Result<Self, AuthError> {
        Self::begin_until(
            Instant::now() + std::time::Duration::from_millis(super::codex_oauth::OAUTH_TIMEOUT_MS),
        )
        .await
    }

    pub(super) async fn begin_until(deadline: Instant) -> Result<Self, AuthError> {
        let home = auth_storage::home();
        std::fs::create_dir_all(&home)?;
        let home = home.canonicalize()?;
        super::device_session::cancel_pending(&home);
        let id = uuid::Uuid::new_v4().to_string();
        let (cancel_tx, cancel) = watch::channel(false);
        let (done, done_rx) = watch::channel(false);
        let previous = ATTEMPTS.lock().map_err(|_| AuthError::Changed)?.insert(
            home.clone(),
            Active {
                id: id.clone(),
                cancel: cancel_tx,
                done: done_rx,
            },
        );
        let mut attempt = Self {
            home,
            id,
            snapshot: Value::Null,
            deadline,
            cancel,
            _done: done,
        };
        let initial = tokio::select! {
            biased;
            () = attempt.cancelled() => return Err(AuthError::Changed),
            result = tokio::time::timeout_at(deadline, async {
                if let Some(mut previous) = previous {
                    let _ = previous.cancel.send(true);
                    let _ = previous.done.changed().await;
                }
                auth_storage::transaction_async(&attempt.home, || {
                    attempt.ensure_current()?;
                    let snapshot = snapshot(&attempt.home)?;
                    auth_storage::claim_generation(&attempt.home, &attempt.id)?;
                    Ok(snapshot)
                }).await
            }) => result.map_err(|_| AuthError::OAuth("Login timed out".into()))??,
        };
        attempt.snapshot = initial;
        Ok(attempt)
    }

    pub(super) fn home(&self) -> &Path {
        &self.home
    }

    pub(super) fn id(&self) -> &str {
        &self.id
    }

    pub(super) fn ensure_current(&self) -> Result<(), AuthError> {
        let attempts = ATTEMPTS.lock().map_err(|_| AuthError::Changed)?;
        if attempts
            .get(&self.home)
            .is_none_or(|active| active.id != self.id)
            || *self.cancel.borrow()
            || Instant::now() >= self.deadline
        {
            return Err(AuthError::Changed);
        }
        Ok(())
    }

    pub async fn cancelled(&self) {
        let mut cancel = self.cancel.clone();
        if !*cancel.borrow() {
            let _ = cancel.changed().await;
        }
    }

    pub async fn commit(&self, tokens: &CodexTokens) -> Result<(), AuthError> {
        self.commit_with_expiry(tokens, None).await
    }

    pub(super) async fn commit_with_expiry(
        &self,
        tokens: &CodexTokens,
        expires_in: Option<u64>,
    ) -> Result<(), AuthError> {
        let received = chrono::Utc::now();
        let expires_at = expires_in.map(|seconds| {
            received
                .timestamp()
                .saturating_add(seconds.min(i64::MAX as u64) as i64)
        });
        tokio::select! {
            biased;
            () = self.cancelled() => Err(AuthError::Changed),
            result = tokio::time::timeout_at(self.deadline,
                auth_storage::transaction_async(&self.home, || {
                    let attempts = ATTEMPTS.lock().map_err(|_| AuthError::Changed)?;
                    if attempts.get(&self.home).is_none_or(|active| active.id != self.id)
                        || *self.cancel.borrow() {
                        return Err(AuthError::Changed);
                    }
                    if auth_storage::generation(&self.home)? != self.id
                        || snapshot(&self.home)? != self.snapshot {
                        return Err(AuthError::Changed);
                    }
                    if Instant::now() >= self.deadline {
                        return Err(AuthError::OAuth("Login timed out".into()));
                    }
                    let mut document = auth_storage::document(&self.home)?;
                    let fields = document.as_object_mut().ok_or(AuthError::Changed)?;
                    fields.insert("tokens".into(), serde_json::to_value(tokens)?);
                    fields.insert("last_refresh".into(), received.to_rfc3339().into());
                    match expires_at {
                        Some(expiry) => {
                            fields.insert("fspec_expires_at".into(), expiry.into());
                        }
                        None => {
                            fields.remove("fspec_expires_at");
                        }
                    }
                    let generation = uuid::Uuid::new_v4().to_string();
                    auth_storage::claim_generation(&self.home, &generation)?;
                    fields.insert("fspec_generation".into(), generation.into());
                    auth_storage::replace(&self.home, &document)
                })) => result.map_err(|_| AuthError::OAuth("Login timed out".into()))?,
        }
    }
}

impl Drop for LoginAttempt {
    fn drop(&mut self) {
        if let Ok(mut attempts) = ATTEMPTS.lock() {
            if attempts
                .get(&self.home)
                .is_some_and(|active| active.id == self.id)
            {
                attempts.remove(&self.home);
            }
        }
    }
}
