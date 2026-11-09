use super::{index, MessageStore, StoredMessage, LRU_CACHE_CAPACITY};
use lru::LruCache;
use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::MutexGuard;
use std::time::SystemTime;
use uuid::Uuid;

pub(super) struct StoreState {
    pub(super) index: HashMap<Uuid, index::IndexEntry>,
    pub(super) cache: LruCache<Uuid, StoredMessage>,
    pub(super) generation: Option<StoreGeneration>,
    pub(super) fully_scanned: bool,
}

impl StoreState {
    pub(super) fn new() -> Result<Self, String> {
        let capacity = NonZeroUsize::new(LRU_CACHE_CAPACITY).ok_or("Invalid LRU cache capacity")?;
        Ok(Self {
            index: HashMap::new(),
            cache: LruCache::new(capacity),
            generation: None,
            fully_scanned: false,
        })
    }
}

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct StoreGeneration {
    pub(super) messages: Option<FileStamp>,
    index: Option<FileStamp>,
}

impl StoreGeneration {
    pub(super) fn read(store: &MessageStore) -> Result<Self, String> {
        Ok(Self {
            messages: FileStamp::read(&store.messages_file)?,
            index: FileStamp::read(&store.index_file)?,
        })
    }
}

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct FileStamp {
    length: u64,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
    #[cfg(unix)]
    identity: (u64, u64, i64, i64),
}

impl FileStamp {
    fn read(path: &Path) -> Result<Option<Self>, String> {
        match fs::metadata(path) {
            Ok(metadata) => Ok(Some(Self::from_metadata(metadata))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!(
                "Failed to stat message store file {path:?}: {error}"
            )),
        }
    }

    fn from_metadata(metadata: Metadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;

        Self {
            length: metadata.len(),
            modified: metadata.modified().ok(),
            created: metadata.created().ok(),
            #[cfg(unix)]
            identity: (
                metadata.dev(),
                metadata.ino(),
                metadata.ctime(),
                metadata.ctime_nsec(),
            ),
        }
    }
}

/// BUG-625: the index lock is held only for in-memory work, so a poisoned
/// one means a bug in that work; it is reported, never recovered silently.
pub(super) const INDEX_POISONED: &str = "the message store index lock is poisoned";

impl MessageStore {
    /// BUG-625: lock the in-memory index and cache. Never hold the guard
    /// across file I/O: the file lock serializes that work.
    pub(super) fn lock_state(&self) -> Result<MutexGuard<'_, StoreState>, String> {
        self.state.lock().map_err(|_| INDEX_POISONED.to_string())
    }

    pub(super) fn get_locked(&self, id: Uuid) -> Option<StoredMessage> {
        let entry = {
            let mut state = self.lock_state().ok()?;
            if let Some(message) = state.cache.get(&id) {
                return Some(message.clone());
            }
            *state.index.get(&id)?
        };
        let message = index::read_message_at(&self.messages_file, &entry).ok()?;
        if message.id != id {
            return None;
        }
        self.lock_state().ok()?.cache.put(id, message.clone());
        Some(message)
    }
}
