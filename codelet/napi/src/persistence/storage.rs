//! File system storage for messages and sessions

use super::types::*;
use super::{ensure_directories, get_data_dir};
use chrono::Utc;
use codelet_common::token_estimator::count_tokens;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Message store - handles storing and retrieving messages
pub struct MessageStore {
    messages_dir: PathBuf,
    /// In-memory cache of messages by ID
    cache: HashMap<Uuid, StoredMessage>,
}

impl MessageStore {
    /// Create a new message store
    pub fn new() -> Result<Self, String> {
        ensure_directories()?;
        let messages_dir = get_data_dir()?.join("messages");
        let mut store = Self {
            messages_dir,
            cache: HashMap::new(),
        };
        store.load_all()?;
        Ok(store)
    }

    /// Load all messages from disk into cache
    fn load_all(&mut self) -> Result<(), String> {
        let messages_file = self.messages_dir.join("messages.jsonl");
        if !messages_file.exists() {
            return Ok(());
        }

        let file = File::open(&messages_file)
            .map_err(|e| format!("Failed to open messages file: {}", e))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
            if line.trim().is_empty() {
                continue;
            }
            let msg: StoredMessage = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse message: {}", e))?;
            self.cache.insert(msg.id, msg);
        }

        Ok(())
    }

    /// Store a new message and return its ID
    pub fn store(&mut self, role: &str, content: &str) -> Result<Uuid, String> {
        self.store_with_metadata(role, content, HashMap::new())
    }

    /// Store a new message with metadata and return its ID
    pub fn store_with_metadata(
        &mut self,
        role: &str,
        content: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<Uuid, String> {
        let id = Uuid::new_v4();
        let content_hash = compute_hash(content.as_bytes());

        let msg = StoredMessage {
            id,
            content_hash,
            created_at: Utc::now(),
            role: role.to_string(),
            content: content.to_string(),
            token_count: Some(count_tokens(content) as u32),
            blob_refs: Vec::new(),
            metadata,
        };

        // Append to JSONL file
        let messages_file = self.messages_dir.join("messages.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&messages_file)
            .map_err(|e| format!("Failed to open messages file: {}", e))?;

        let json = serde_json::to_string(&msg)
            .map_err(|e| format!("Failed to serialize message: {}", e))?;
        writeln!(file, "{}", json).map_err(|e| format!("Failed to write message: {}", e))?;

        self.cache.insert(id, msg);
        Ok(id)
    }

    /// Get a message by ID
    pub fn get(&self, id: Uuid) -> Option<&StoredMessage> {
        self.cache.get(&id)
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
    pub fn cleanup_orphans(
        &mut self,
        referenced_ids: &std::collections::HashSet<Uuid>,
    ) -> Result<usize, String> {
        let all_ids: Vec<Uuid> = self.cache.keys().cloned().collect();
        let orphans: Vec<Uuid> = all_ids
            .into_iter()
            .filter(|id| !referenced_ids.contains(id))
            .collect();

        let count = orphans.len();
        for id in orphans {
            self.cache.remove(&id);
        }

        // Rewrite the messages file without orphans
        self.rewrite_messages_file()?;

        Ok(count)
    }

    /// Rewrite the messages file with current cache
    fn rewrite_messages_file(&self) -> Result<(), String> {
        let messages_file = self.messages_dir.join("messages.jsonl");
        let temp_file = self.messages_dir.join("messages.jsonl.tmp");

        let mut file =
            File::create(&temp_file).map_err(|e| format!("Failed to create temp file: {}", e))?;

        for msg in self.cache.values() {
            let json = serde_json::to_string(msg)
                .map_err(|e| format!("Failed to serialize message: {}", e))?;
            writeln!(file, "{}", json).map_err(|e| format!("Failed to write message: {}", e))?;
        }

        fs::rename(&temp_file, &messages_file)
            .map_err(|e| format!("Failed to rename temp file: {}", e))?;

        Ok(())
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
            .map_err(|e| format!("Failed to read sessions dir: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "json") {
                let content = fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read session file: {}", e))?;
                let session: SessionManifest = serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse session: {}", e))?;

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
            .map_err(|e| format!("Failed to serialize session: {}", e))?;

        fs::write(&path, json).map_err(|e| format!("Failed to write session file: {}", e))?;

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
            .ok_or_else(|| format!("Session {} not found", id))
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
            .ok_or_else(|| format!("No session found for project {:?}", project))
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
            let filename = format!("{}.json", id);
            let path = self.sessions_dir.join(&filename);
            if path.exists() {
                fs::remove_file(&path)
                    .map_err(|e| format!("Failed to delete session file: {}", e))?;
            }
        }
        Ok(())
    }

    /// Rename a session
    pub fn rename(&mut self, id: Uuid, new_name: &str) -> Result<(), String> {
        let session = self
            .cache
            .get_mut(&id)
            .ok_or_else(|| format!("Session {} not found", id))?;
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
