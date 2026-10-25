use super::{JournalError, JournalRecord, JournalStore, MAX_RECORD_BYTES};
use crate::persistence::messages::durability::{perform, sync_parent};
use crate::persistence::transcript::admission::{Admission, HistoryStamp};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

impl JournalStore {
    pub(crate) fn append_unowned(&self, record: &JournalRecord) -> Result<(), JournalError> {
        let encoded = serde_json::to_vec(record)?;
        if encoded.len() as u64 > MAX_RECORD_BYTES {
            return Err(JournalError::Capacity);
        }
        let _lease = self.lease()?;
        let mut slot = self
            .admission
            .lock()
            .map_err(|_| std::io::Error::other("transcript admission poisoned"))?;
        let previous = slot.take();
        let mut admission = match previous {
            Some(admission)
                if HistoryStamp::read(&self.directory, admission.tail.as_deref())
                    .ok()
                    .as_ref()
                    == admission.stamp.as_ref() =>
            {
                admission
            }
            previous => {
                let paths = self.paths()?;
                if previous
                    .as_ref()
                    .is_some_and(|state| paths.len() < state.len())
                {
                    return Err(JournalError::Incomplete);
                }
                let records = self.read_paths(&paths)?;
                let mut admission = Admission::default();
                for historical in records {
                    let encoded = serde_json::to_vec(&historical)?;
                    admission.existing(&historical, &encoded)?;
                    let transition = admission.check(&historical)?;
                    admission.commit(&historical, &encoded, transition);
                }
                admission.tail = paths.last().cloned();
                admission
            }
        };
        let existing = admission.existing(record, &encoded)?;
        let transition = if existing.is_none() {
            admission.check(record)?
        } else {
            None
        };
        let destination = self.destination(existing.unwrap_or(admission.len()), record);
        self.publish_record(&destination, &encoded)?;
        if existing.is_none() {
            admission.commit(record, &encoded, transition);
            admission.tail = Some(destination);
        }
        admission.stamp = Some(HistoryStamp::read(
            &self.directory,
            admission.tail.as_deref(),
        )?);
        *slot = Some(admission);
        Ok(())
    }

    fn publish_record(&self, destination: &Path, encoded: &[u8]) -> Result<(), JournalError> {
        match fs::read(destination) {
            Ok(existing) => {
                if existing != encoded {
                    return Err(JournalError::Identity);
                }
                perform("transcript.record.sync", || {
                    File::open(destination)?.sync_all()
                })?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut stage = tempfile::Builder::new()
                    .prefix("delta-")
                    .suffix(".stage")
                    .tempfile_in(&self.directory)?;
                stage.write_all(encoded)?;
                perform("transcript.record.sync", || stage.as_file().sync_all())?;
                stage
                    .persist_noclobber(destination)
                    .map_err(|error| JournalError::Io(error.error))?;
            }
            Err(error) => return Err(error.into()),
        }
        perform("transcript.directory.sync", || sync_parent(destination))?;
        if let Some(parent) = self.directory.parent() {
            perform("transcript.root.sync", || sync_parent(parent))?;
        }
        Ok(())
    }
}
