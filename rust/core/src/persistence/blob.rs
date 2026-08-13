//! Content-addressed blob storage for large envelope content.
//!
//! Lifted from `rust/napi/src/persistence/blob.rs` in RPC-034 so
//! codelet-rpc, codelet-rpc-embedded, codelet-rpc-server and the upcoming
//! codelet-sessions crate can hash-store and retrieve blobs without
//! re-introducing the forbidden `rpc → napi` arrow.
//!
//! On-disk layout (byte-identical with the pre-lift NAPI store):
//!
//! - `{data_dir}/blobs/{first2hex}/{full_64_hex}` — one file per blob,
//!   where `{first2hex}` is the first two hex chars of the SHA-256 digest
//!   used as a directory prefix for filesystem distribution.
//!
//! The wire-format prefix [`crate::persistence::BLOB_REF_PREFIX`]
//! (`"blob:sha256:"`) is documented alongside the envelope helpers in
//! [`crate::persistence::blob_processing`].

use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;

/// Threshold above which envelope content is extracted to blob storage.
///
/// MUST stay byte-identical with the pre-lift NAPI constant (10 KiB) —
/// changing it alters which messages get blob-extracted, which is a
/// wire-observable behaviour for downstream agents holding pre-lift
/// blob references.
const BLOB_THRESHOLD: usize = 10 * 1024;

/// Blob storage for large content.
pub struct BlobStore {
    blobs_dir: PathBuf,
}

impl BlobStore {
    /// Create a new blob store.
    ///
    /// Resolves the data directory via [`codelet_common::get_data_dir`]
    /// and ensures `{data_dir}/blobs/` exists. Matches the RPC-032 /
    /// RPC-033 pattern of inlining the directory-creation responsibility
    /// instead of depending on the NAPI-local `ensure_directories`
    /// helper.
    pub fn new() -> Result<Self, String> {
        let data_dir = codelet_common::get_data_dir()?;
        let blobs_dir = data_dir.join("blobs");
        fs::create_dir_all(&blobs_dir)
            .map_err(|e| format!("Failed to create blobs dir {blobs_dir:?}: {e}"))?;
        Ok(Self { blobs_dir })
    }

    /// Store content and return its SHA-256 hash.
    ///
    /// If the content already exists (same hash), this is a no-op
    /// (content-addressed deduplication).
    pub fn store(&self, content: &[u8]) -> Result<String, String> {
        let hash = compute_sha256(content);
        let blob_path = self.get_blob_path(&hash);

        // If blob already exists, skip writing (deduplication).
        if !blob_path.exists() {
            // Write to temp file first, then rename for atomicity.
            let temp_path = self.blobs_dir.join(format!("{hash}.tmp"));

            let mut file = File::create(&temp_path)
                .map_err(|e| format!("Failed to create blob temp file: {e}"))?;

            file.write_all(content)
                .map_err(|e| format!("Failed to write blob content: {e}"))?;

            file.sync_all()
                .map_err(|e| format!("Failed to sync blob file: {e}"))?;

            fs::rename(&temp_path, &blob_path)
                .map_err(|e| format!("Failed to rename blob temp file: {e}"))?;
        }

        Ok(hash)
    }

    /// Retrieve content by its hash.
    pub fn get(&self, hash: &str) -> Result<Vec<u8>, String> {
        // Validate hash format.
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("Invalid blob hash: {hash}"));
        }

        let blob_path = self.get_blob_path(hash);

        if !blob_path.exists() {
            return Err(format!("Blob not found: {hash}"));
        }

        let mut file =
            File::open(&blob_path).map_err(|e| format!("Failed to open blob file: {e}"))?;

        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .map_err(|e| format!("Failed to read blob content: {e}"))?;

        // Verify hash.
        let actual_hash = compute_sha256(&content);
        if actual_hash != hash {
            return Err(format!(
                "Blob hash mismatch: expected {hash}, got {actual_hash}"
            ));
        }

        Ok(content)
    }

    /// Check if a blob exists.
    pub fn exists(&self, hash: &str) -> bool {
        // Validate hash format to prevent slice panic.
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
        self.get_blob_path(hash).exists()
    }

    /// Delete a blob by hash.
    pub fn delete(&self, hash: &str) -> Result<(), String> {
        // Validate hash format to prevent slice panic.
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("Invalid blob hash: {hash}"));
        }
        let blob_path = self.get_blob_path(hash);
        if blob_path.exists() {
            fs::remove_file(&blob_path).map_err(|e| format!("Failed to delete blob: {e}"))?;
        }
        Ok(())
    }

    /// Get the file path for a blob.
    fn get_blob_path(&self, hash: &str) -> PathBuf {
        // Use first 2 chars as subdirectory for better filesystem distribution.
        let subdir = &hash[0..2];
        let dir = self.blobs_dir.join(subdir);

        // Create subdirectory if it doesn't exist.
        let _ = fs::create_dir_all(&dir);

        dir.join(hash)
    }

    /// Get the total size of all blobs.
    pub fn total_size(&self) -> Result<u64, String> {
        let mut total = 0u64;

        for entry in
            fs::read_dir(&self.blobs_dir).map_err(|e| format!("Failed to read blobs dir: {e}"))?
        {
            let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
            let path = entry.path();

            if path.is_dir() {
                for sub_entry in
                    fs::read_dir(&path).map_err(|e| format!("Failed to read subdir: {e}"))?
                {
                    let sub_entry = sub_entry.map_err(|e| format!("Failed to read entry: {e}"))?;
                    if let Ok(meta) = sub_entry.metadata() {
                        total += meta.len();
                    }
                }
            }
        }

        Ok(total)
    }
}

/// Compute SHA-256 hash of content.
fn compute_sha256(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Determine if content should be stored in blob storage.
///
/// Content larger than [`BLOB_THRESHOLD`] (10 KiB) is extracted to blob
/// storage via [`crate::persistence::process_envelope_for_blob_storage`].
pub fn should_use_blob_storage(content: &[u8]) -> bool {
    content.len() > BLOB_THRESHOLD
}

// ============================================================================
// Global singleton store
// ============================================================================
//
// BLOB_STORE is the process-wide cache used by the free-function facade
// below. It mirrors the pre-RPC-034 NAPI layout so the on-disk layout
// stays byte-identical and the existing 80+ persistence tests can be
// exercised through the NAPI re-export shim.

lazy_static::lazy_static! {
    static ref BLOB_STORE: Mutex<Option<BlobStore>> = Mutex::new(None);
}

fn init_blob_store() -> Result<(), String> {
    let mut store = BLOB_STORE.lock().map_err(|e| e.to_string())?;
    if store.is_none() {
        *store = Some(BlobStore::new()?);
    }
    Ok(())
}

/// Store content in blob storage.
///
/// Lazily initialises the global [`BlobStore`] singleton against the
/// directory configured by [`codelet_common::set_data_directory`].
pub fn store_blob(content: &[u8]) -> Result<String, String> {
    init_blob_store()?;
    let store = BLOB_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_ref()
        .ok_or("Blob store not initialized")?
        .store(content)
}

/// Get content from blob storage.
pub fn get_blob(hash: &str) -> Result<Vec<u8>, String> {
    init_blob_store()?;
    let store = BLOB_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_ref()
        .ok_or("Blob store not initialized")?
        .get(hash)
}

/// Check if a blob exists.
pub fn blob_exists(hash: &str) -> Result<bool, String> {
    init_blob_store()?;
    let store = BLOB_STORE.lock().map_err(|e| e.to_string())?;
    Ok(store
        .as_ref()
        .ok_or("Blob store not initialized")?
        .exists(hash))
}

/// Reset the lifted BLOB_STORE singleton.
///
/// Called by [`crate::persistence::manifest::reset_stores_for_tests`]
/// (which is in turn called by `codelet_napi::persistence::set_data_directory`)
/// so the next blob operation re-initialises against the new directory.
pub fn reset_blob_store_for_tests() {
    if let Ok(mut store) = BLOB_STORE.lock() {
        *store = None;
    }
}

/// Test-only accessor: is the lifted BLOB_STORE singleton currently
/// initialised?
///
/// Matches the pattern set by
/// [`crate::persistence::is_message_store_initialized_for_tests`] and
/// [`crate::persistence::is_session_store_initialized_for_tests`] from
/// RPC-033, so codelet-napi's `lazy_init_tests` can verify BUG-122's
/// per-store lazy-initialisation invariants without poking the
/// `lazy_static` global directly.
pub fn is_blob_store_initialized_for_tests() -> bool {
    BLOB_STORE.lock().map(|s| s.is_some()).unwrap_or(false)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha256() {
        let hash = compute_sha256(b"hello world");
        assert_eq!(hash.len(), 64);
        // Known hash for "hello world".
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_should_use_blob_storage() {
        let small = vec![0u8; 100];
        let large = vec![0u8; 20_000];

        assert!(!should_use_blob_storage(&small));
        assert!(should_use_blob_storage(&large));
    }

    #[test]
    fn test_blob_threshold_boundary_is_strict_greater_than() {
        // Preserve the pre-lift `>` boundary semantics: exactly 10 KiB
        // stays inline, one byte over crosses into blob storage.
        let exactly = vec![0u8; BLOB_THRESHOLD];
        let just_over = vec![0u8; BLOB_THRESHOLD + 1];
        assert!(!should_use_blob_storage(&exactly));
        assert!(should_use_blob_storage(&just_over));
    }
}
