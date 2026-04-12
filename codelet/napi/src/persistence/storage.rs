//! File system storage for messages and sessions (BUG-122 Layer 2)
//!
//! MessageStore uses a binary index file (`messages.idx`) with on-demand
//! seek() and an LRU cache instead of loading the entire `messages.jsonl`
//! into a HashMap.

use super::message_index::{self, IndexEntry};
use super::types::*;
use super::{ensure_directories, get_data_dir};
use chrono::Utc;
use codelet_common::token_estimator::count_tokens;
use lru::LruCache;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use tracing::warn;
use uuid::Uuid;

/// Default LRU cache capacity for deserialized messages
const LRU_CACHE_CAPACITY: usize = 4000;

/// Message store — binary-indexed, on-demand loading with LRU cache.
///
/// Instead of reading the entire `messages.jsonl` into memory, this struct
/// maintains a lightweight in-memory index (`UUID → (offset, length)`) loaded
/// from a binary `messages.idx` file, and fetches individual messages on
/// demand via `seek()`.
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
    /// Create a new message store with binary index
    pub fn new() -> Result<Self, String> {
        ensure_directories()?;
        let messages_dir = get_data_dir()?.join("messages");
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

        if let Some((loaded_index, recorded_size)) = message_index::load_index(&self.index_file) {
            if recorded_size == actual_size {
                // Index is current
                self.index = loaded_index;
                return Ok(());
            } else if actual_size > recorded_size {
                // Incremental scan from where we left off
                self.index = loaded_index;
                let (new_entries, final_size) =
                    message_index::scan_jsonl_range(&self.messages_file, recorded_size)?;
                self.index.extend(new_entries);
                message_index::save_index(&self.index_file, &self.index, final_size)?;
                return Ok(());
            }
            // actual < recorded or corrupt — fall through to full rebuild
            warn!("Index file stale (actual {actual_size} < recorded {recorded_size}), rebuilding");
        }

        // Full scan
        let (entries, final_size) = message_index::scan_jsonl_range(&self.messages_file, 0)?;
        self.index = entries;
        message_index::save_index(&self.index_file, &self.index, final_size)?;
        Ok(())
    }

    /// Store a new message and return its ID
    pub fn store(&mut self, role: &str, content: &str) -> Result<Uuid, String> {
        self.store_with_metadata(role, content, HashMap::new())
    }

    /// Store a new message with metadata and return its ID
    ///
    /// If metadata contains `_actualTokenCount`, that value is used instead of
    /// calculating tokens from content. This is critical for blob-stored messages
    /// where the content is a summary/reference, not the actual data.
    pub fn store_with_metadata(
        &mut self,
        role: &str,
        content: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<Uuid, String> {
        let id = Uuid::new_v4();
        let content_hash = compute_hash(content.as_bytes());

        // Use _actualTokenCount from metadata if present (for blob-stored messages)
        // Otherwise fall back to counting tokens from the content string
        let token_count = metadata
            .get("_actualTokenCount")
            .and_then(|v| v.as_u64())
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
        message_index::save_index(&self.index_file, &self.index, new_data_size)?;

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
        let msg = message_index::read_message_at(&self.messages_file, entry).ok()?;

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
    /// (reclaimed on next cleanup_orphans).
    pub fn update_metadata(
        &mut self,
        id: Uuid,
        entries: HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
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
        message_index::save_index(&self.index_file, &self.index, new_data_size)?;

        Ok(())
    }

    /// Get all message IDs referenced by any session
    pub fn get_referenced_ids(
        &self,
        sessions: &[SessionManifest],
    ) -> std::collections::HashSet<Uuid> {
        sessions
            .iter()
            .flat_map(|s| s.messages.iter().map(|m| m.message_id))
            .collect()
    }

    /// Remove orphaned messages (not referenced by any session)
    ///
    /// Rewrites `messages.jsonl` to contain only referenced messages,
    /// then rebuilds the binary index.
    pub fn cleanup_orphans(
        &mut self,
        referenced_ids: &std::collections::HashSet<Uuid>,
    ) -> Result<usize, String> {
        let all_ids: Vec<Uuid> = self.index.keys().copied().collect();
        let orphans: Vec<Uuid> = all_ids
            .into_iter()
            .filter(|id| !referenced_ids.contains(id))
            .collect();

        let count = orphans.len();
        if count == 0 {
            return Ok(0);
        }

        // Rewrite the messages file, keeping only non-orphan entries
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
            let msg = message_index::read_message_at(&self.messages_file, entry)
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
        message_index::save_index(&self.index_file, &self.index, current_offset)?;

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

/// Session store - handles storing and retrieving session manifests
pub struct SessionStore {
    sessions_dir: PathBuf,
    /// In-memory cache of sessions by ID
    cache: HashMap<Uuid, SessionManifest>,
    /// Track the last active session per project
    last_session: HashMap<PathBuf, Uuid>,
}

impl SessionStore {
    /// Create a new session store
    pub fn new() -> Result<Self, String> {
        ensure_directories()?;
        let sessions_dir = get_data_dir()?.join("sessions");
        let mut store = Self {
            sessions_dir,
            cache: HashMap::new(),
            last_session: HashMap::new(),
        };
        store.load_all()?;
        Ok(store)
    }

    /// Load all sessions from disk
    fn load_all(&mut self) -> Result<(), String> {
        if !self.sessions_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.sessions_dir)
            .map_err(|e| format!("Failed to read sessions dir: {e}"))?
        {
            let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "json") {
                let content = match fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        warn!("Skipping unreadable session file {:?}: {}", path, e);
                        continue;
                    }
                };
                let session: SessionManifest = match serde_json::from_str(&content) {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("Skipping incompatible session file {:?}: {}", path, e);
                        continue;
                    }
                };

                // Track as last session for this project
                self.last_session
                    .insert(session.project.clone(), session.id);
                self.cache.insert(session.id, session);
            }
        }

        Ok(())
    }

    /// Create a new session
    pub fn create(&mut self, name: &str, project: &Path) -> Result<SessionManifest, String> {
        let session = SessionManifest::new(name, project.to_path_buf());
        self.save(&session)?;
        self.last_session.insert(project.to_path_buf(), session.id);
        self.cache.insert(session.id, session.clone());
        Ok(session)
    }

    /// Create a new session with provider
    pub fn create_with_provider(
        &mut self,
        name: &str,
        project: &Path,
        provider: &str,
    ) -> Result<SessionManifest, String> {
        let session = SessionManifest::with_provider(name, project.to_path_buf(), provider);
        self.save(&session)?;
        self.last_session.insert(project.to_path_buf(), session.id);
        self.cache.insert(session.id, session.clone());
        Ok(session)
    }

    /// Save a session to disk
    pub fn save(&mut self, session: &SessionManifest) -> Result<(), String> {
        let filename = format!("{}.json", session.id);
        let path = self.sessions_dir.join(&filename);

        let json = serde_json::to_string_pretty(session)
            .map_err(|e| format!("Failed to serialize session: {e}"))?;

        fs::write(&path, json).map_err(|e| format!("Failed to write session file: {e}"))?;

        self.cache.insert(session.id, session.clone());
        self.last_session
            .insert(session.project.clone(), session.id);

        Ok(())
    }

    /// Get a session by ID
    pub fn get(&self, id: Uuid) -> Option<&SessionManifest> {
        self.cache.get(&id)
    }

    /// Get a mutable reference to a session
    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut SessionManifest> {
        self.cache.get_mut(&id)
    }

    /// Load a session by ID (returns owned value)
    pub fn load(&self, id: Uuid) -> Result<SessionManifest, String> {
        self.cache
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("Session {id} not found"))
    }

    /// Get the last active session for a project
    pub fn get_last_session(&self, project: &Path) -> Option<&SessionManifest> {
        self.last_session
            .get(project)
            .and_then(|id| self.cache.get(id))
    }

    /// Resume the last session for a project
    pub fn resume_last(&self, project: &Path) -> Result<SessionManifest, String> {
        self.get_last_session(project)
            .cloned()
            .ok_or_else(|| format!("No session found for project {project:?}"))
    }

    /// List all sessions for a project
    pub fn list_for_project(&self, project: &Path) -> Vec<&SessionManifest> {
        self.cache
            .values()
            .filter(|s| s.project == project)
            .collect()
    }

    /// List all sessions
    pub fn list_all(&self) -> Vec<&SessionManifest> {
        self.cache.values().collect()
    }

    /// Delete a session
    pub fn delete(&mut self, id: Uuid) -> Result<(), String> {
        let session = self.cache.remove(&id);
        if let Some(session) = session {
            // Remove from last_session if it was the last
            if self.last_session.get(&session.project) == Some(&id) {
                self.last_session.remove(&session.project);
            }

            // Delete the file
            let filename = format!("{id}.json");
            let path = self.sessions_dir.join(&filename);
            if path.exists() {
                fs::remove_file(&path)
                    .map_err(|e| format!("Failed to delete session file: {e}"))?;
            }
        }
        Ok(())
    }

    /// Rename a session
    pub fn rename(&mut self, id: Uuid, new_name: &str) -> Result<(), String> {
        let session = self
            .cache
            .get_mut(&id)
            .ok_or_else(|| format!("Session {id} not found"))?;
        session.name = new_name.to_string();
        session.updated_at = Utc::now();

        let session_clone = session.clone();
        self.save(&session_clone)?;
        Ok(())
    }

    /// Fork a session at a specific message index
    pub fn fork(
        &mut self,
        source: &SessionManifest,
        at_index: usize,
        name: &str,
    ) -> Result<SessionManifest, String> {
        // Validate index
        if at_index >= source.messages.len() {
            return Err(format!(
                "Fork index {} is out of range (session has {} messages)",
                at_index,
                source.messages.len()
            ));
        }

        // Check compaction boundary
        if let Some(ref compaction) = source.compaction {
            if at_index < compaction.compacted_before_index {
                return Err(format!(
                    "Cannot fork at index {} which is before compaction boundary {}. \
                     Compacted messages cannot be individually accessed. \
                     Fork at index {} or later.",
                    at_index, compaction.compacted_before_index, compaction.compacted_before_index
                ));
            }
        }

        // Create new session with forked messages (preserve provider)
        let mut new_session = if source.provider.is_empty() {
            SessionManifest::new(name, source.project.clone())
        } else {
            SessionManifest::with_provider(name, source.project.clone(), &source.provider)
        };

        new_session.forked_from = Some(ForkPoint {
            source_session_id: source.id,
            fork_after_index: at_index,
            forked_at: Utc::now(),
        });

        // Copy message references up to and including at_index
        for (i, msg_ref) in source.messages.iter().enumerate() {
            if i > at_index {
                break;
            }
            new_session.messages.push(MessageRef {
                message_id: msg_ref.message_id,
                source: MessageSource::Forked {
                    from_session: source.id,
                },
            });
        }

        // Inherit compaction state if applicable
        if let Some(ref compaction) = source.compaction {
            if at_index >= compaction.compacted_before_index {
                new_session.compaction = Some(compaction.clone());
            }
        }

        self.save(&new_session)?;
        self.cache.insert(new_session.id, new_session.clone());

        Ok(new_session)
    }
}

/// Compute SHA-256 hash of content
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
