use super::durability::{perform, sync_parent};
use super::{compute_hash, index, MessageStore, StoreGeneration, StoredMessage};
use chrono::Utc;
use codelet_common::token_estimator::count_tokens;
use index::IndexEntry;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use uuid::Uuid;

impl MessageStore {
    /// Store a new message and return its ID.
    pub fn store(&mut self, role: &str, content: &str) -> Result<Uuid, String> {
        self.store_with_metadata(role, content, HashMap::new())
    }

    /// Store a new message with metadata and return its ID.
    ///
    /// `_actualTokenCount` overrides token estimation for blob-stored messages.
    pub fn store_with_metadata(
        &mut self,
        role: &str,
        content: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<Uuid, String> {
        self.store_with_id(Uuid::new_v4(), role, content, metadata)
    }

    pub fn store_with_id(
        &mut self,
        id: Uuid,
        role: &str,
        content: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<Uuid, String> {
        self.with_file_lock(|store| {
            store.load_or_build_index()?;
            let indexed = store
                .state
                .lock()
                .map_err(|error| error.to_string())?
                .index
                .contains_key(&id);
            if !indexed {
                store.get_recovered_locked(id)?;
            }
            let entry = store
                .state
                .lock()
                .map_err(|error| error.to_string())?
                .index
                .get(&id)
                .copied();
            if let Some(entry) = entry {
                let existing = index::read_message_at(&store.messages_file, &entry)?;
                if existing.id != id
                    || existing.role != role
                    || existing.content != content
                    || existing.metadata != metadata
                {
                    return Err(format!("Message identity conflict for {id}"));
                }
                let file = File::open(&store.messages_file)
                    .map_err(|error| format!("Failed to open existing message: {error}"))?;
                store.sync_message(&file)?;
                return Ok(id);
            }
            store.append_message(Self::new_message(id, role, content, metadata))?;
            Ok(id)
        })
    }

    fn new_message(
        id: Uuid,
        role: &str,
        content: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> StoredMessage {
        let token_count = metadata
            .get("_actualTokenCount")
            .and_then(serde_json::Value::as_u64)
            .map(|value| value as u32)
            .unwrap_or_else(|| count_tokens(content) as u32);
        StoredMessage {
            id,
            content_hash: compute_hash(content.as_bytes()),
            created_at: Utc::now(),
            role: role.to_string(),
            content: content.to_string(),
            token_count: Some(token_count),
            blob_refs: Vec::new(),
            metadata,
        }
    }

    /// Re-append a message with merged metadata, leaving its old record as dead space.
    pub fn update_metadata(
        &mut self,
        id: Uuid,
        entries: HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        self.with_file_lock(|store| {
            let recovery_message = if store.messages_file.exists() {
                None
            } else {
                store.get_locked(id)
            };
            store.load_or_build_index()?;
            let mut message = store
                .get_recovered_locked(id)?
                .or(recovery_message)
                .ok_or_else(|| format!("Message {id} not found"))?;
            message.metadata.extend(entries);
            store.append_message(message)
        })
    }

    fn append_message(&mut self, message: StoredMessage) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.messages_file)
            .map_err(|error| format!("Failed to open messages file: {error}"))?;
        let byte_offset = file
            .metadata()
            .map_err(|error| format!("Failed to stat messages file: {error}"))?
            .len();
        let json = serde_json::to_string(&message)
            .map_err(|error| format!("Failed to serialize message: {error}"))?;
        let line = format!("{json}\n");
        file.write_all(line.as_bytes())
            .map_err(|error| format!("Failed to write message: {error}"))?;
        self.sync_message(&file)?;
        let byte_length = line.len() as u32;
        let mut state = self.state.lock().map_err(|error| error.to_string())?;
        state.index.insert(
            message.id,
            IndexEntry {
                byte_offset,
                byte_length,
            },
        );
        state.cache.put(message.id, message);
        index::save_index(
            &self.index_file,
            &state.index,
            byte_offset + byte_length as u64,
        )?;
        state.generation = Some(StoreGeneration::read(self)?);
        Ok(())
    }

    fn sync_message(&self, file: &File) -> Result<(), String> {
        perform("message.sync", || file.sync_all())
            .map_err(|error| format!("Failed to synchronize message: {error}"))?;
        perform("message.directory", || sync_parent(&self.messages_file))
            .map_err(|error| format!("Failed to synchronize message directory: {error}"))
    }
}
