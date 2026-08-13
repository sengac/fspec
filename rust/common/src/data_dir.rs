//! Global data directory management
//!
//! Provides a single source of truth for the application data directory.
//! All file storage (sessions, cache, blobs, etc.) derives from this base path.
//!
//! Usage:
//! 1. Call `set_data_directory()` once at application startup
//! 2. All modules use `get_data_dir()` to derive their paths

use std::path::PathBuf;
use std::sync::Mutex;

static DATA_DIRECTORY: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Set the base data directory for the application
///
/// This MUST be called once at startup before any file operations.
/// All storage paths are derived from this directory:
/// - {data_dir}/sessions/   - Session manifests
/// - {data_dir}/messages/   - Message store
/// - {data_dir}/blobs/      - Blob storage
/// - {data_dir}/cache/      - Model cache
/// - {data_dir}/debug/      - Debug captures
///
/// # Arguments
/// * `dir` - The base data directory (e.g., ~/.fspec)
pub fn set_data_directory(dir: PathBuf) -> Result<(), String> {
    let mut guard = DATA_DIRECTORY.lock().map_err(|e| e.to_string())?;
    *guard = Some(dir);
    Ok(())
}

/// Get the base data directory
///
/// Returns the directory set via `set_data_directory()`.
/// Returns an error if not initialized - this ensures proper startup.
pub fn get_data_dir() -> Result<PathBuf, String> {
    let guard = DATA_DIRECTORY.lock().map_err(|e| e.to_string())?;
    guard.clone().ok_or_else(|| {
        "Data directory not initialized. Call set_data_directory() at startup.".to_string()
    })
}
