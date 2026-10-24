use super::durability::{perform, sync_parent};
use super::{index, MessageStore, StoreGeneration};
use index::IndexEntry;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use tracing::warn;
use uuid::Uuid;

impl MessageStore {
    /// Lock a stable sidecar outside the replaceable messages directory.
    pub(super) fn with_file_lock<T>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String> {
        let _lock = self.lock_file()?;
        operation(self)
    }

    pub(super) fn lock_file(&self) -> Result<File, String> {
        fs::create_dir_all(&self.messages_dir).map_err(|error| {
            format!(
                "Failed to ensure messages dir {:?}: {error}",
                self.messages_dir
            )
        })?;
        let lock_path = self.messages_dir.with_extension("lock");
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|error| format!("Failed to open messages lock {lock_path:?}: {error}"))?;
        lock.lock()
            .map_err(|error| format!("Failed to lock messages store {lock_path:?}: {error}"))?;
        Ok(lock)
    }

    /// Refresh from the saved index, scanning missing bytes or rebuilding as needed.
    pub(super) fn load_or_build_index(&self) -> Result<(), String> {
        let generation = StoreGeneration::read(self)?;
        let mut state = self.state.lock().map_err(|error| error.to_string())?;
        if state.generation.as_ref() == Some(&generation) {
            return Ok(());
        }
        state.cache.clear();
        state.generation = None;
        state.fully_scanned = false;
        if generation.messages.is_none() {
            state.index.clear();
            state.generation = Some(generation);
            state.fully_scanned = true;
            return Ok(());
        }

        self.recover_tail_locked()
            .map_err(|error| error.to_string())?;
        let generation = StoreGeneration::read(self)?;
        let data_metadata = fs::metadata(&self.messages_file)
            .map_err(|error| format!("Failed to stat messages file: {error}"))?;
        let actual_size = data_metadata.len();
        let index_is_newer = fs::metadata(&self.index_file)
            .and_then(|metadata| metadata.modified())
            .ok()
            .zip(data_metadata.modified().ok())
            .is_some_and(|(index_time, data_time)| index_time > data_time);

        if let Some((entries, recorded_size)) = index::load_index(&self.index_file) {
            if recorded_size == actual_size && index_is_newer {
                state.index = entries;
                state.generation = Some(generation);
                return Ok(());
            }
            warn!("Index stale or data changed (actual {actual_size}, recorded {recorded_size}), rebuilding");
        }

        let (entries, final_size) = index::scan_jsonl_range(&self.messages_file, 0)?;
        index::save_index(&self.index_file, &entries, final_size)?;
        state.index = entries;
        state.generation = Some(StoreGeneration::read(self)?);
        state.fully_scanned = true;
        Ok(())
    }

    /// Remove orphaned messages (not referenced by any session).
    ///
    /// Rewrites `messages.jsonl` to contain only referenced messages,
    /// then rebuilds the binary index. The caller supplies referenced IDs.
    pub fn cleanup_orphans(&mut self, referenced_ids: &HashSet<Uuid>) -> Result<usize, String> {
        self.with_file_lock(|store| {
            store.load_or_build_index()?;
            store.cleanup_orphans_locked(referenced_ids)
        })
    }

    pub(crate) fn cleanup_with_references<Guard>(
        &mut self,
        discover: impl FnOnce() -> Result<Option<(HashSet<Uuid>, Guard)>, String>,
    ) -> Result<usize, String> {
        self.with_file_lock(|store| {
            store.load_or_build_index()?;
            let Some((referenced, _guard)) = discover()? else {
                return Ok(0);
            };
            store.cleanup_orphans_locked(&referenced)
        })
    }

    fn cleanup_orphans_locked(&mut self, referenced_ids: &HashSet<Uuid>) -> Result<usize, String> {
        for id in referenced_ids {
            self.get_recovered_locked(*id)?.ok_or_else(|| {
                format!("Required original message {id} is missing; cleanup refused")
            })?;
        }
        let mut state = self.state.lock().map_err(|error| error.to_string())?;
        let count = state
            .index
            .keys()
            .filter(|id| !referenced_ids.contains(id))
            .count();
        if count == 0 {
            return Ok(0);
        }

        let temp_file = self.messages_dir.join("messages.jsonl.tmp");
        let mut out = File::create(&temp_file)
            .map_err(|error| format!("Failed to create temp file: {error}"))?;
        let mut new_index = HashMap::new();
        let mut current_offset = 0;

        for (id, entry) in &state.index {
            if !referenced_ids.contains(id) {
                continue;
            }
            let message = index::read_message_at(&self.messages_file, entry)
                .map_err(|error| format!("Failed to read message {id} during cleanup: {error}"))?;
            let json = serde_json::to_string(&message)
                .map_err(|error| format!("Failed to serialize message: {error}"))?;
            let line = format!("{json}\n");
            out.write_all(line.as_bytes())
                .map_err(|error| format!("Failed to write message: {error}"))?;

            let byte_length = line.len() as u32;
            new_index.insert(
                *id,
                IndexEntry {
                    byte_offset: current_offset,
                    byte_length,
                },
            );
            current_offset += byte_length as u64;
        }

        out.flush()
            .map_err(|error| format!("Failed to flush temp file: {error}"))?;
        perform("cleanup.sync", || out.sync_all())
            .map_err(|error| format!("Failed to synchronize cleanup stage: {error}"))?;
        perform("cleanup.invalidate", || {
            match fs::remove_file(&self.index_file) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            sync_parent(&self.index_file)
        })
        .map_err(|error| format!("Failed to invalidate cleanup index: {error}"))?;
        perform("cleanup.rename", || {
            fs::rename(&temp_file, &self.messages_file)
        })
        .map_err(|error| format!("Failed to rename temp file: {error}"))?;
        state.index = new_index;
        state.cache.clear();
        state.generation = None;
        state.fully_scanned = true;
        perform("cleanup.directory", || sync_parent(&self.messages_file))
            .map_err(|error| format!("Cleanup promoted; directory sync failed: {error}"))?;
        index::save_index(&self.index_file, &state.index, current_offset)
            .map_err(|error| format!("Cleanup promoted; derived index failed: {error}"))?;
        state.generation =
            Some(StoreGeneration::read(self).map_err(|error| {
                format!("Cleanup promoted; generation refresh failed: {error}")
            })?);
        Ok(count)
    }
}
