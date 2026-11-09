use super::{index, MessageStore, StoreGeneration, StoredMessage};
use uuid::Uuid;

impl MessageStore {
    pub(super) fn get_recovered_locked(&self, id: Uuid) -> Result<Option<StoredMessage>, String> {
        if let Some(message) = self.get_locked(id) {
            return Ok(Some(message));
        }
        if self.lock_state()?.fully_scanned {
            return Ok(None);
        }
        // BUG-625: scan and save outside the index lock (the caller holds
        // the file lock); only the result is installed under it.
        let (entries, final_size) = index::scan_jsonl_range(&self.messages_file, 0)?;
        index::save_index(&self.index_file, &entries, final_size)?;
        let generation = StoreGeneration::read(self)?;
        {
            let mut state = self.lock_state()?;
            state.cache.clear();
            state.index = entries;
            state.generation = Some(generation.clone());
            state.fully_scanned = true;
        }
        super::verified::publish(self, &generation);
        Ok(self.get_locked(id))
    }
}
