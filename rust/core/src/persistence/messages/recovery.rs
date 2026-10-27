use super::{index, MessageStore, StoreGeneration, StoredMessage};
use uuid::Uuid;

impl MessageStore {
    pub(super) fn get_recovered_locked(&self, id: Uuid) -> Result<Option<StoredMessage>, String> {
        if let Some(message) = self.get_locked(id) {
            return Ok(Some(message));
        }
        {
            let mut state = self.state.lock().map_err(|error| error.to_string())?;
            if state.fully_scanned {
                return Ok(None);
            }
            let (entries, final_size) = index::scan_jsonl_range(&self.messages_file, 0)?;
            index::save_index(&self.index_file, &entries, final_size)?;
            state.cache.clear();
            state.index = entries;
            state.generation = Some(StoreGeneration::read(self)?);
            state.fully_scanned = true;
            if let Some(generation) = &state.generation {
                super::verified::publish(self, generation);
            }
        }
        Ok(self.get_locked(id))
    }
}
