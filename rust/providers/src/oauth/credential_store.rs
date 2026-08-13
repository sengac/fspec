//! Generic Credential Store (PROV-060)
//!
//! Provides `CredentialStore<T>` for type-safe, provider-agnostic credential
//! file I/O. Replaces the three separate read/write function pairs in
//! `copilot/auth.rs`, `codex/codex_auth.rs`, and `claude_auth.rs`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};

/// Generic credential store for any provider's auth JSON file.
///
/// `T` is the provider-specific auth struct (e.g. `CopilotAuthJson`,
/// `CodexAuthJson`, `ClaudeAuthJson`). It must be `Serialize + DeserializeOwned`.
#[derive(Debug, Clone)]
pub struct CredentialStore<T> {
    /// Absolute path to the credential file
    path: PathBuf,
    /// Phantom marker for the credential type
    _marker: std::marker::PhantomData<T>,
}

impl<T> CredentialStore<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Create a new `CredentialStore` pointing at the given path.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            _marker: std::marker::PhantomData,
        }
    }

    /// Return the path this store reads from / writes to.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Read credentials from the file (sync).
    ///
    /// Returns `Ok(None)` if the file does not exist.
    pub fn read_sync(&self) -> Result<Option<T>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&self.path)
            .with_context(|| format!("failed to read {}", self.path.display()))?;
        let value: T = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse {}", self.path.display()))?;
        Ok(Some(value))
    }

    /// Read credentials from the file (async).
    ///
    /// Returns `Ok(None)` if the file does not exist.
    pub async fn read(&self) -> Result<Option<T>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let content = tokio::fs::read_to_string(&self.path)
            .await
            .with_context(|| format!("failed to read {}", self.path.display()))?;
        let value: T = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse {}", self.path.display()))?;
        Ok(Some(value))
    }

    /// Write credentials to the file (sync).
    ///
    /// Creates parent directories if needed.
    pub fn write_sync(&self, value: &T) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let content = serde_json::to_string_pretty(value)?;
        std::fs::write(&self.path, content)
            .with_context(|| format!("failed to write {}", self.path.display()))?;
        Ok(())
    }

    /// Write credentials to the file (async).
    ///
    /// Creates parent directories if needed.
    pub async fn write(&self, value: &T) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let content = serde_json::to_string_pretty(value)?;
        tokio::fs::write(&self.path, content)
            .await
            .with_context(|| format!("failed to write {}", self.path.display()))?;
        Ok(())
    }

    /// Write credentials with file mode 0600 on Unix (async).
    ///
    /// Creates parent directories if needed.
    pub async fn write_secure(&self, value: &T) -> Result<()> {
        self.write(value).await?;
        enforce_mode_0600(&self.path).await?;
        Ok(())
    }

    /// Delete the credential file (async, idempotent).
    ///
    /// Returns `Ok(())` even if the file does not exist.
    pub async fn delete(&self) -> Result<()> {
        if !self.path.exists() {
            return Ok(());
        }
        tokio::fs::remove_file(&self.path)
            .await
            .with_context(|| format!("failed to delete {}", self.path.display()))?;
        Ok(())
    }

    /// Check if the credential file exists.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

/// Apply `0o600` permissions to a credential file on Unix hosts.
#[cfg(unix)]
async fn enforce_mode_0600(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = tokio::fs::metadata(path).await?.permissions();
    perms.set_mode(0o600);
    tokio::fs::set_permissions(path, perms).await?;
    Ok(())
}

#[cfg(not(unix))]
async fn enforce_mode_0600(_path: &std::path::Path) -> Result<()> {
    Ok(())
}
