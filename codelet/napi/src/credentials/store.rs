//! Credential Store
//!
//! Manages cached credentials with mtime-based change detection.
//! Uses lazy_static singleton pattern matching the persistence module.

use super::types::CredentialsFile;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

lazy_static::lazy_static! {
    static ref CREDENTIAL_STORE: Mutex<Option<CredentialStore>> = Mutex::new(None);
}

/// Atomic counter for disk reads (for testing)
static DISK_READ_COUNT: AtomicU64 = AtomicU64::new(0);

/// Credential store with in-memory cache and mtime tracking
pub struct CredentialStore {
    credentials_file: PathBuf,
    cache: CredentialsFile,
    last_mtime: Option<SystemTime>,
}

impl CredentialStore {
    /// Create a new credential store for the given data directory
    pub fn new(data_dir: &PathBuf) -> Result<Self, String> {
        let credentials_file = data_dir.join("credentials").join("credentials.json");
        let mut store = Self {
            credentials_file,
            cache: CredentialsFile::default(),
            last_mtime: None,
        };
        store.reload_if_changed()?;
        Ok(store)
    }

    /// Check file mtime and reload if changed
    /// Returns true if the file was reloaded
    pub fn reload_if_changed(&mut self) -> Result<bool, String> {
        let current_mtime = std::fs::metadata(&self.credentials_file)
            .ok()
            .and_then(|m| m.modified().ok());

        if current_mtime != self.last_mtime {
            self.load_from_disk()?;
            self.last_mtime = current_mtime;
            return Ok(true);
        }
        Ok(false)
    }

    /// Force reload from disk
    pub fn force_reload(&mut self) -> Result<(), String> {
        self.load_from_disk()?;
        self.last_mtime = std::fs::metadata(&self.credentials_file)
            .ok()
            .and_then(|m| m.modified().ok());
        Ok(())
    }

    /// Load credentials from disk
    fn load_from_disk(&mut self) -> Result<(), String> {
        DISK_READ_COUNT.fetch_add(1, Ordering::SeqCst);
        
        if !self.credentials_file.exists() {
            self.cache = CredentialsFile::default();
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.credentials_file)
            .map_err(|e| format!("Failed to read credentials file: {}", e))?;

        if content.trim().is_empty() {
            self.cache = CredentialsFile::default();
            return Ok(());
        }

        self.cache = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse credentials file: {}", e))?;

        Ok(())
    }

    /// Get API key for a provider from cache
    pub fn get_api_key(&mut self, provider_id: &str) -> Result<Option<String>, String> {
        // Check for file changes before returning cached value
        self.reload_if_changed()?;
        Ok(self.cache
            .providers
            .get(provider_id)
            .map(|c| c.api_key.clone()))
    }
}

/// Initialize the credential store with a specific data directory
pub fn init_credential_store_with_dir(data_dir: &PathBuf) -> Result<(), String> {
    let mut store = CREDENTIAL_STORE.lock().map_err(|e| e.to_string())?;
    *store = Some(CredentialStore::new(data_dir)?);
    Ok(())
}

/// Initialize the credential store using the global data directory
pub fn init_credential_store() -> Result<(), String> {
    let data_dir = codelet_common::get_data_dir()?;
    init_credential_store_with_dir(&data_dir)
}

/// Get the global credential store, initializing if needed
fn get_store() -> Result<std::sync::MutexGuard<'static, Option<CredentialStore>>, String> {
    CREDENTIAL_STORE.lock().map_err(|e| e.to_string())
}

/// Get the current disk read count (for testing)
pub fn get_disk_read_count() -> u64 {
    DISK_READ_COUNT.load(Ordering::SeqCst)
}

/// Reset disk read count (for testing)
pub fn reset_disk_read_count() {
    DISK_READ_COUNT.store(0, Ordering::SeqCst);
}

/// Reset the credential store
/// Clears the global store so a fresh one will be created on next access.
/// Called by persistence::set_data_directory to ensure credentials use new directory.
pub fn reset_credential_store() {
    if let Ok(mut store) = CREDENTIAL_STORE.lock() {
        *store = None;
    }
    reset_disk_read_count();
}

/// Reload credentials from disk (called after TypeScript saves credentials).
/// Returns true if the file was reloaded (mtime changed), false otherwise.
/// After reloading, also updates environment variables for all providers
/// so that active sessions pick up the new credentials immediately.
pub(crate) fn credentials_reload() -> Result<bool, String> {
    let mut store = get_store()?;
    if let Some(ref mut s) = *store {
        let reloaded = s.reload_if_changed()?;
        if reloaded {
            // Update env vars for all providers so active sessions pick up changes
            drop(store); // Release lock before calling resolver
            super::resolver::update_all_provider_env_vars()?;
        }
        return Ok(reloaded);
    }
    // If store not initialized, try to initialize it
    drop(store);
    init_credential_store()?;
    let mut store = get_store()?;
    if let Some(ref mut s) = *store {
        let reloaded = s.reload_if_changed()?;
        if reloaded {
            drop(store);
            super::resolver::update_all_provider_env_vars()?;
        }
        return Ok(reloaded);
    }
    Err("Credential store not initialized".to_string())
}

/// Refresh credentials on session resume
pub fn refresh_credentials_on_resume(data_dir: &PathBuf) -> Result<(), String> {
    let mut store = get_store()?;
    if store.is_none() {
        *store = Some(CredentialStore::new(data_dir)?);
    }
    if let Some(ref mut s) = *store {
        s.reload_if_changed()?;
    }
    Ok(())
}

/// Get API key from the credential store for a provider
pub fn get_stored_api_key(provider_id: &str) -> Result<Option<String>, String> {
    let mut store = get_store()?;
    if store.is_none() {
        drop(store);
        if let Err(_e) = init_credential_store() {
            return Ok(None);
        }
        store = get_store()?;
    }
    if let Some(ref mut s) = *store {
        return s.get_api_key(provider_id);
    }
    Ok(None)
}

/// Get API key from the credential store for a provider with specific data dir
pub fn get_stored_api_key_with_dir(
    provider_id: &str,
    data_dir: &PathBuf,
) -> Result<Option<String>, String> {
    let mut store = get_store()?;
    if store.is_none() {
        *store = Some(CredentialStore::new(data_dir)?);
    }
    if let Some(ref mut s) = *store {
        return s.get_api_key(provider_id);
    }
    Ok(None)
}
