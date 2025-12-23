//! Content-addressed blob storage for large content
//!
//! Uses SHA-256 hashing for content addressing.
//! Identical content is automatically deduplicated.

use super::{ensure_directories, get_data_dir};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

/// Blob storage for large content
pub struct BlobStore {
    blobs_dir: PathBuf,
}

impl BlobStore {
    /// Create a new blob store
    pub fn new() -> Result<Self, String> {
        ensure_directories()?;
        let blobs_dir = get_data_dir()?.join("blobs");
        Ok(Self { blobs_dir })
    }

    /// Store content and return its SHA-256 hash
    ///
    /// If the content already exists (same hash), this is a no-op.
    pub fn store(&self, content: &[u8]) -> Result<String, String> {
        let hash = compute_sha256(content);
        let blob_path = self.get_blob_path(&hash);

        // If blob already exists, skip writing (deduplication)
        if !blob_path.exists() {
            // Write to temp file first, then rename for atomicity
            let temp_path = self.blobs_dir.join(format!("{}.tmp", hash));

            let mut file = File::create(&temp_path)
                .map_err(|e| format!("Failed to create blob temp file: {}", e))?;

            file.write_all(content)
                .map_err(|e| format!("Failed to write blob content: {}", e))?;

            file.sync_all()
                .map_err(|e| format!("Failed to sync blob file: {}", e))?;

            fs::rename(&temp_path, &blob_path)
                .map_err(|e| format!("Failed to rename blob temp file: {}", e))?;
        }

        Ok(hash)
    }

    /// Retrieve content by its hash
    pub fn get(&self, hash: &str) -> Result<Vec<u8>, String> {
        // Validate hash format
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("Invalid blob hash: {}", hash));
        }

        let blob_path = self.get_blob_path(hash);

        if !blob_path.exists() {
            return Err(format!("Blob not found: {}", hash));
        }

        let mut file =
            File::open(&blob_path).map_err(|e| format!("Failed to open blob file: {}", e))?;

        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .map_err(|e| format!("Failed to read blob content: {}", e))?;

        // Verify hash
        let actual_hash = compute_sha256(&content);
        if actual_hash != hash {
            return Err(format!(
                "Blob hash mismatch: expected {}, got {}",
                hash, actual_hash
            ));
        }

        Ok(content)
    }

    /// Check if a blob exists
    pub fn exists(&self, hash: &str) -> bool {
        self.get_blob_path(hash).exists()
    }

    /// Delete a blob by hash
    pub fn delete(&self, hash: &str) -> Result<(), String> {
        let blob_path = self.get_blob_path(hash);
        if blob_path.exists() {
            fs::remove_file(&blob_path).map_err(|e| format!("Failed to delete blob: {}", e))?;
        }
        Ok(())
    }

    /// Get the file path for a blob
    fn get_blob_path(&self, hash: &str) -> PathBuf {
        // Use first 2 chars as subdirectory for better filesystem distribution
        let subdir = &hash[0..2];
        let dir = self.blobs_dir.join(subdir);

        // Create subdirectory if it doesn't exist
        let _ = fs::create_dir_all(&dir);

        dir.join(hash)
    }

    /// Get the total size of all blobs
    pub fn total_size(&self) -> Result<u64, String> {
        let mut total = 0u64;

        for entry in
            fs::read_dir(&self.blobs_dir).map_err(|e| format!("Failed to read blobs dir: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.is_dir() {
                for sub_entry in
                    fs::read_dir(&path).map_err(|e| format!("Failed to read subdir: {}", e))?
                {
                    let sub_entry =
                        sub_entry.map_err(|e| format!("Failed to read entry: {}", e))?;
                    if let Ok(meta) = sub_entry.metadata() {
                        total += meta.len();
                    }
                }
            }
        }

        Ok(total)
    }
}

/// Compute SHA-256 hash of content
fn compute_sha256(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Determine if content should be stored in blob storage
///
/// Content larger than the threshold should be extracted to blob storage.
pub fn should_use_blob_storage(content: &[u8]) -> bool {
    // Threshold: 10KB
    const BLOB_THRESHOLD: usize = 10 * 1024;
    content.len() > BLOB_THRESHOLD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha256() {
        let hash = compute_sha256(b"hello world");
        assert_eq!(hash.len(), 64);
        // Known hash for "hello world"
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
}
