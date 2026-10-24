use super::codex_auth::CodexAuthJson;
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

#[path = "auth_storage_files.rs"]
mod files;
#[path = "auth_storage_sources.rs"]
mod sources;

pub(crate) use sources::document;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Codex credential storage failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Codex credential document is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Codex keyring storage failed: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("{operation}; credential rollback failed: {rollback}; recovery backup: {recovery:?}")]
    Rollback {
        operation: Box<AuthError>,
        rollback: std::io::Error,
        recovery: Option<PathBuf>,
    },
    #[error("Codex authentication changed; sign in again")]
    Changed,
    #[error("Codex OAuth failed: {0}")]
    OAuth(String),
}

pub(crate) type Result<T> = std::result::Result<T, AuthError>;

pub(crate) fn home() -> PathBuf {
    super::codex_auth::get_auth_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

pub(crate) fn lock(home: &Path, name: &str) -> Result<File> {
    fs::create_dir_all(home)?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(options.open(home.join(name))?)
}

pub(crate) fn transaction<T>(home: &Path, operation: impl FnOnce() -> Result<T>) -> Result<T> {
    let guard = lock(home, ".fspec-auth.lock")?;
    guard.lock()?;
    operation()
}

pub(crate) async fn transaction_async<T>(
    home: &Path,
    operation: impl FnOnce() -> Result<T>,
) -> Result<T> {
    let guard = lock(home, ".fspec-auth.lock")?;
    loop {
        match guard.try_lock() {
            Ok(()) => return operation(),
            Err(std::fs::TryLockError::WouldBlock) => {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            Err(std::fs::TryLockError::Error(error)) => return Err(error.into()),
        }
    }
}

pub(crate) fn generation(home: &Path) -> Result<String> {
    match fs::read_to_string(home.join(".fspec-auth-generation")) {
        Ok(generation) => Ok(generation),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn claim_generation(home: &Path, generation: &str) -> Result<()> {
    fs::create_dir_all(home)?;
    files::stage(home, ".fspec-auth-generation-", generation.as_bytes())?
        .persist(home.join(".fspec-auth-generation"))
        .map_err(|error| error.error)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn keychain(home: &Path) -> Result<keyring::Entry> {
    use sha2::{Digest, Sha256};
    let canonical = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
    let hash = format!(
        "{:x}",
        Sha256::digest(canonical.to_string_lossy().as_bytes())
    );
    Ok(keyring::Entry::new(
        "Codex Auth",
        &format!("cli|{}", &hash[..16]),
    )?)
}

fn delete_preferred(home: &Path) -> Result<()> {
    #[cfg(not(target_os = "macos"))]
    let _ = home;
    #[cfg(target_os = "macos")]
    match keychain(home)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => (),
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

pub(crate) fn read(home: &Path) -> Result<Option<CodexAuthJson>> {
    let document = document(home)?;
    if document.as_object().is_some_and(serde_json::Map::is_empty) {
        return Ok(None);
    }
    Ok(Some(serde_json::from_value(document)?))
}

pub(crate) fn replace(home: &Path, document: &Value) -> Result<()> {
    files::change(home, Some(document), || delete_preferred(home))
}

pub(crate) fn update(home: &Path, auth: &CodexAuthJson, generation: bool) -> Result<()> {
    let mut document = document(home)?;
    let fields = document.as_object_mut().ok_or(AuthError::Changed)?;
    if let Some(key) = &auth.openai_api_key {
        fields.insert("OPENAI_API_KEY".into(), key.clone().into());
    }
    match &auth.tokens {
        Some(tokens) => {
            fields.insert("tokens".into(), serde_json::to_value(tokens)?);
        }
        None => {
            fields.remove("tokens");
        }
    }
    fields.insert(
        "last_refresh".into(),
        auth.last_refresh
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
            .into(),
    );
    if generation {
        fields.remove("fspec_expires_at");
        let next_generation = uuid::Uuid::new_v4().to_string();
        claim_generation(home, &next_generation)?;
        fields.insert("fspec_generation".into(), next_generation.into());
    }
    replace(home, &document)
}

pub fn read_codex_auth() -> Result<Option<CodexAuthJson>> {
    read(&home())
}

pub fn write_codex_auth(auth: &CodexAuthJson) -> Result<()> {
    let home = home();
    transaction(&home, || update(&home, auth, true))?;
    super::auth_lifecycle::cancel_at(&home);
    Ok(())
}

pub fn clear_codex_oauth() -> Result<()> {
    let home = home();
    super::auth_lifecycle::cancel_at(&home);
    transaction(&home, || {
        let mut auth = read(&home)?.unwrap_or(CodexAuthJson {
            openai_api_key: None,
            tokens: None,
            last_refresh: None,
        });
        auth.tokens = None;
        update(&home, &auth, true)
    })
}

pub fn delete_codex_auth() -> Result<()> {
    let home = home();
    super::auth_lifecycle::cancel_at(&home);
    transaction(&home, || {
        claim_generation(&home, &uuid::Uuid::new_v4().to_string())?;
        document(&home)?;
        files::change(&home, None, || delete_preferred(&home))
    })
}
