//! On-disk message store + supporting types.
//!
//! Lifted from `rust/napi/src/persistence/storage.rs` and
//! `rust/napi/src/persistence/types.rs` (the `StoredMessage`,
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

#[doc(hidden)]
pub mod durability;
mod framing;
pub use framing::FramingError;
mod index;
mod maintenance;
mod recovery;
mod state;
mod write;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

use state::{StoreGeneration, StoreState};

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
    state: Mutex<StoreState>,
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

        let mut store = Self {
            messages_dir,
            messages_file,
            index_file,
            state: Mutex::new(StoreState::new()?),
        };
        store.with_file_lock(|store| store.load_or_build_index())?;
        Ok(store)
    }

    /// Get a message by ID (on-demand loading with LRU cache)
    pub fn get(&self, id: Uuid) -> Option<StoredMessage> {
        let _lock = self.lock_file().ok()?;
        self.load_or_build_index().ok()?;
        self.get_recovered_locked(id).ok().flatten()
    }

    /// Number of entries in the index (for testing)
    pub fn index_len(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.index.len())
            .unwrap_or(0)
    }

    /// Number of entries currently in the LRU cache (for testing)
    pub fn cache_len(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.cache.len())
            .unwrap_or(0)
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
