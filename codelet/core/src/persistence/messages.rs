//! On-disk message store + supporting types.
//!
//! Lifted from `codelet/napi/src/persistence/storage.rs` and
//! `codelet/napi/src/persistence/types.rs` (the `StoredMessage`,
//! `MessageSource`, `MessageRef` slices) in RPC-032 so codelet-rpc,
//! codelet-rpc-embedded, codelet-rpc-server and the upcoming
//! codelet-sessions crate can append/load messages without re-introducing
//! the forbidden `rpc → napi` arrow.
//!
//! On-disk layout (byte-identical with the pre-lift NAPI store):
//!
//! - `{data_dir}/messages/messages.jsonl` — append-only JSONL of
//!   `StoredMessage` records, one per line.
//! - `{data_dir}/messages/messages.idx` — BUG-122 Layer 2 binary index
//!   produced by [`index`].
//!
//! `SessionManifest`-coupled helpers (e.g. the old
//! `MessageStore::get_referenced_ids(&[SessionManifest])`) are NOT lifted
//! here — they belong with `SessionStore` (lifted in RPC-033). The single
//! caller of `get_referenced_ids` inside `codelet-napi` inlines the
//! trivial HashSet build so the lifted `MessageStore` has zero
//! `SessionManifest` dependency.

mod index;

use chrono::{DateTime, Utc};
use codelet_common::token_estimator::count_tokens;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use tracing::warn;
use uuid::Uuid;

use index::IndexEntry;

/// Default LRU cache capacity for deserialized messages
const LRU_CACHE_CAPACITY: usize = 4000;

// ============================================================================
// Public on-the-wire types (lifted from napi::persistence::types)
// ============================================================================

/// A stored message in the content-addressed message store.
///
/// On-disk JSON serialization MUST stay byte-identical with the pre-lift
/// NAPI definition — field order and serde defaults are preserved
/// verbatim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    /// Unique identifier for this message
    pub id: Uuid,
    /// SHA-256 hash of the content (for integrity verification)
    pub content_hash: String,
    /// When this message was created
    pub created_at: DateTime<Utc>,
    /// Role: "user" or "assistant"
    pub role: String,
    /// The message content (or preview if blob_refs is populated)
    pub content: String,
    /// Approximate token count for context tracking
    pub token_count: Option<u32>,
    /// References to blob storage for large content
    pub blob_refs: Vec<String>,
    /// Provider-specific metadata (model used, stop reason, etc.)
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Tracks where a message reference came from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageSource {
    /// Created natively in this session
    Native,
    /// Inherited from a forked session
    Forked { from_session: Uuid },
    /// Imported via merge or cherry-pick
    Imported {
        from_session: Uuid,
        original_index: usize,
    },
}

/// A reference to a message in a session manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRef {
    /// The ID of the stored message
    pub message_id: Uuid,
    /// How this message got into this session
    pub source: MessageSource,
}

// ============================================================================
// MessageStore — append-only, binary-indexed, LRU-cached
// ============================================================================

/// Message store — binary-indexed, on-demand loading with LRU cache.
///
/// Instead of reading the entire `messages.jsonl` into memory, this struct
/// maintains a lightweight in-memory index (`UUID → (offset, length)`)
/// loaded from a binary `messages.idx` file, and fetches individual
/// messages on demand via `seek()`.
pub struct MessageStore {
    messages_dir: PathBuf,
    messages_file: PathBuf,
    index_file: PathBuf,
    /// UUID → (byte_offset, byte_length) — loaded from binary .idx file
    index: HashMap<Uuid, IndexEntry>,
    /// LRU cache for recently accessed deserialized messages
    cache: std::sync::Mutex<LruCache<Uuid, StoredMessage>>,
}

impl MessageStore {
    /// Create a new message store with binary index.
    ///
    /// Resolves the data directory via [`codelet_common::get_data_dir`]
    /// and ensures `{data_dir}/messages/` exists.
    pub fn new() -> Result<Self, String> {
        let data_dir = codelet_common::get_data_dir()?;
        let messages_dir = data_dir.join("messages");
        fs::create_dir_all(&messages_dir)
            .map_err(|e| format!("Failed to create messages dir {messages_dir:?}: {e}"))?;
        let messages_file = messages_dir.join("messages.jsonl");
        let index_file = messages_dir.join("messages.idx");

        let capacity = NonZeroUsize::new(LRU_CACHE_CAPACITY)
            .ok_or("Invalid LRU cache capacity")?;

        let mut store = Self {
            messages_dir,
            messages_file,
            index_file,
            index: HashMap::new(),
            cache: std::sync::Mutex::new(LruCache::new(capacity)),
        };
        store.load_or_build_index()?;
        Ok(store)
    }

    /// Load the binary index or build it from scratch.
    ///
    /// 1. If `messages.idx` exists: load it, compare stored data_file_size
    ///    - Equal: index is current — done
    ///    - Actual > stored: incrementally scan only new bytes
    ///    - Actual < stored or corrupt: full rebuild
    /// 2. If missing: full scan, then save to `messages.idx`
    fn load_or_build_index(&mut self) -> Result<(), String> {
        if !self.messages_file.exists() {
            return Ok(());
        }

        let actual_size = fs::metadata(&self.messages_file)
            .map_err(|e| format!("Failed to stat messages file: {e}"))?
            .len();

        if let Some((loaded_index, recorded_size)) = index::load_index(&self.index_file) {
            if recorded_size == actual_size {
                // Index is current
                self.index = loaded_index;
                return Ok(());
            } else if actual_size > recorded_size {
                // Incremental scan from where we left off
                self.index = loaded_index;
                let (new_entries, final_size) =
                    index::scan_jsonl_range(&self.messages_file, recorded_size)?;
                self.index.extend(new_entries);
                index::save_index(&self.index_file, &self.index, final_size)?;
                return Ok(());
            }
            // actual < recorded or corrupt — fall through to full rebuild
            warn!("Index file stale (actual {actual_size} < recorded {recorded_size}), rebuilding");
        }

        // Full scan
        let (entries, final_size) = index::scan_jsonl_range(&self.messages_file, 0)?;
        self.index = entries;
        index::save_index(&self.index_file, &self.index, final_size)?;
        Ok(())
    }

    /// LOG-006 defence: ensure `self.messages_dir` exists before any
    /// write-path I/O.
    ///
    /// `MessageStore::new()` calls `create_dir_all(&messages_dir)` once
    /// at construction time, but the store is a long-lived
    /// `lazy_static` singleton (see [`crate::persistence::manifest`])
    /// that caches `messages_dir: PathBuf` for the lifetime of the
    /// process. If the directory disappears between construction and a
    /// later write — test teardown, a sibling tool wiping `~/.fspec`,
    /// `codelet_common::set_data_directory` swapped to a new root —
    /// every cached write path (`store_with_metadata`,
    /// `update_metadata`, `cleanup_orphans`, `save_index`) would
    /// surface `ENOENT` via different error strings. The first symptom
    /// to make it to the production combined log was
    /// `"Failed to rename index temp file: No such file or directory
    /// (os error 2)"` from `save_index`; the JSONL append path is the
    /// next-most-likely victim and would surface as
    /// `"Failed to open messages file: No such file or directory
    /// (os error 2)"`.
    ///
    /// Calling `create_dir_all` is a no-op when the directory already
    /// exists, so this guard has zero cost on the happy path.
    fn ensure_messages_dir(&self) -> Result<(), String> {
        fs::create_dir_all(&self.messages_dir).map_err(|e| {
            format!(
                "Failed to ensure messages dir {:?}: {e}",
                self.messages_dir
            )
        })
    }

    /// Store a new message and return its ID
    pub fn store(&mut self, role: &str, content: &str) -> Result<Uuid, String> {
        self.store_with_metadata(role, content, HashMap::new())
    }

    /// Store a new message with metadata and return its ID.
    ///
    /// If metadata contains `_actualTokenCount`, that value is used instead
    /// of calculating tokens from content. This is critical for blob-stored
    /// messages where the content is a summary/reference, not the actual data.
    pub fn store_with_metadata(
        &mut self,
        role: &str,
        content: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<Uuid, String> {
        // LOG-006 defence: see `ensure_messages_dir` doc-comment.
        self.ensure_messages_dir()?;

        let id = Uuid::new_v4();
        let content_hash = compute_hash(content.as_bytes());

        // Use _actualTokenCount from metadata if present (for blob-stored messages)
        // Otherwise fall back to counting tokens from the content string
        let token_count = metadata
            .get("_actualTokenCount")
            .and_then(serde_json::Value::as_u64)
            .map(|v| v as u32)
            .unwrap_or_else(|| count_tokens(content) as u32);

        let msg = StoredMessage {
            id,
            content_hash,
            created_at: Utc::now(),
            role: role.to_string(),
            content: content.to_string(),
            token_count: Some(token_count),
            blob_refs: Vec::new(),
            metadata,
        };

        // Record byte offset before writing
        let byte_offset = if self.messages_file.exists() {
            fs::metadata(&self.messages_file)
                .map_err(|e| format!("Failed to stat messages file: {e}"))?
                .len()
        } else {
            0
        };

        // Append to JSONL file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.messages_file)
            .map_err(|e| format!("Failed to open messages file: {e}"))?;

        let json = serde_json::to_string(&msg)
            .map_err(|e| format!("Failed to serialize message: {e}"))?;
        let line = format!("{json}\n");
        file.write_all(line.as_bytes())
            .map_err(|e| format!("Failed to write message: {e}"))?;

        let byte_length = line.len() as u32;

        // Update in-memory index
        self.index.insert(id, IndexEntry { byte_offset, byte_length });

        // Update LRU cache
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(id, msg);
        }

        // Persist index (record the new file size)
        let new_data_size = byte_offset + byte_length as u64;
        index::save_index(&self.index_file, &self.index, new_data_size)?;

        Ok(id)
    }

    /// Get a message by ID (on-demand loading with LRU cache)
    pub fn get(&self, id: Uuid) -> Option<StoredMessage> {
        // Check LRU cache first
        if let Ok(mut cache) = self.cache.lock() {
            if let Some(msg) = cache.get(&id) {
                return Some(msg.clone());
            }
        }

        // Look up in index
        let entry = self.index.get(&id)?;

        // Read from disk
        let msg = index::read_message_at(&self.messages_file, entry).ok()?;

        // Insert into LRU cache
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(id, msg.clone());
        }

        Some(msg)
    }

    /// Update metadata on an existing stored message.
    ///
    /// Re-appends the message with updated metadata and updates the index
    /// to point to the new position. The old entry becomes dead space
    /// (reclaimed on next `cleanup_orphans`).
    pub fn update_metadata(
        &mut self,
        id: Uuid,
        entries: HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        // LOG-006 defence: see `ensure_messages_dir` doc-comment.
        self.ensure_messages_dir()?;

        // Read current message
        let mut msg = self.get(id)
            .ok_or_else(|| format!("Message {id} not found"))?;

        msg.metadata.extend(entries);

        // Record new byte offset
        let byte_offset = if self.messages_file.exists() {
            fs::metadata(&self.messages_file)
                .map_err(|e| format!("Failed to stat messages file: {e}"))?
                .len()
        } else {
            0
        };

        // Re-append with updated metadata
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.messages_file)
            .map_err(|e| format!("Failed to open messages file: {e}"))?;

        let json = serde_json::to_string(&msg)
            .map_err(|e| format!("Failed to serialize message: {e}"))?;
        let line = format!("{json}\n");
        file.write_all(line.as_bytes())
            .map_err(|e| format!("Failed to write message: {e}"))?;

        let byte_length = line.len() as u32;

        // Update index to point to the new position
        self.index.insert(id, IndexEntry { byte_offset, byte_length });

        // Update LRU cache
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(id, msg);
        }

        // Persist index
        let new_data_size = byte_offset + byte_length as u64;
        index::save_index(&self.index_file, &self.index, new_data_size)?;

        Ok(())
    }

    /// Remove orphaned messages (not referenced by any session).
    ///
    /// Rewrites `messages.jsonl` to contain only referenced messages,
    /// then rebuilds the binary index.
    ///
    /// The caller is responsible for computing the set of referenced
    /// message IDs (previously `get_referenced_ids(&[SessionManifest])`
    /// lived here, but that coupled the message store to
    /// `SessionManifest` which now lives in NAPI until RPC-033).
    pub fn cleanup_orphans(
        &mut self,
        referenced_ids: &std::collections::HashSet<Uuid>,
    ) -> Result<usize, String> {
        let orphans: Vec<Uuid> = self
            .index
            .keys()
            .copied()
            .filter(|id| !referenced_ids.contains(id))
            .collect();

        let count = orphans.len();
        if count == 0 {
            return Ok(0);
        }

        // Rewrite the messages file, keeping only non-orphan entries.
        //
        // LOG-006 defence: ensure `messages_dir` exists before opening
        // the temp file. See `ensure_messages_dir` doc-comment for the
        // full rationale.
        self.ensure_messages_dir()?;
        let temp_file = self.messages_dir.join("messages.jsonl.tmp");
        let mut out = File::create(&temp_file)
            .map_err(|e| format!("Failed to create temp file: {e}"))?;

        let mut new_index: HashMap<Uuid, IndexEntry> = HashMap::new();
        let mut current_offset: u64 = 0;

        // Iterate over index entries, reading each referenced message from disk
        for (id, entry) in &self.index {
            if !referenced_ids.contains(id) {
                continue;
            }
            let msg = index::read_message_at(&self.messages_file, entry)
                .map_err(|e| format!("Failed to read message {id} during cleanup: {e}"))?;
            let json = serde_json::to_string(&msg)
                .map_err(|e| format!("Failed to serialize message: {e}"))?;
            let line = format!("{json}\n");
            out.write_all(line.as_bytes())
                .map_err(|e| format!("Failed to write message: {e}"))?;

            let byte_length = line.len() as u32;
            new_index.insert(*id, IndexEntry {
                byte_offset: current_offset,
                byte_length,
            });
            current_offset += byte_length as u64;
        }

        out.flush()
            .map_err(|e| format!("Failed to flush temp file: {e}"))?;

        fs::rename(&temp_file, &self.messages_file)
            .map_err(|e| format!("Failed to rename temp file: {e}"))?;

        // Update in-memory state
        self.index = new_index;

        // Clear orphans from LRU cache
        if let Ok(mut cache) = self.cache.lock() {
            for id in &orphans {
                cache.pop(id);
            }
        }

        // Persist rebuilt index
        index::save_index(&self.index_file, &self.index, current_offset)?;

        Ok(count)
    }

    /// Number of entries in the index (for testing)
    pub fn index_len(&self) -> usize {
        self.index.len()
    }

    /// Number of entries currently in the LRU cache (for testing)
    pub fn cache_len(&self) -> usize {
        self.cache.lock().map(|c| c.len()).unwrap_or(0)
    }
}

/// Compute SHA-256 hash of `content` as a lowercase 64-char hex string.
pub fn compute_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let hash1 = compute_hash(b"hello");
        let hash2 = compute_hash(b"hello");
        let hash3 = compute_hash(b"world");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA-256 produces 64 hex chars
    }
}
