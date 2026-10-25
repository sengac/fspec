use super::lifecycle::{Lifecycle, Transition};
use super::{JournalError, JournalRecord};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(PartialEq, Eq)]
struct FileStamp {
    length: u64,
    modified: SystemTime,
    #[cfg(unix)]
    identity: (u64, u64, i64, i64),
}

impl FileStamp {
    fn read(path: &Path) -> Result<Self, JournalError> {
        let metadata = fs::metadata(path)?;
        Ok(Self {
            length: metadata.len(),
            modified: metadata.modified()?,
            #[cfg(unix)]
            identity: {
                use std::os::unix::fs::MetadataExt;
                (
                    metadata.dev(),
                    metadata.ino(),
                    metadata.ctime(),
                    metadata.ctime_nsec(),
                )
            },
        })
    }
}

#[derive(PartialEq, Eq)]
pub(super) struct HistoryStamp {
    directory: FileStamp,
    tail: Option<FileStamp>,
}

impl HistoryStamp {
    pub(super) fn read(directory: &Path, tail: Option<&Path>) -> Result<Self, JournalError> {
        Ok(Self {
            directory: FileStamp::read(directory)?,
            tail: tail.map(FileStamp::read).transpose()?,
        })
    }
}

#[derive(Default)]
pub(super) struct Admission {
    identities: HashMap<Uuid, (usize, [u8; 32])>,
    coordinates: HashMap<(Uuid, u64), Uuid>,
    sequences: HashMap<Uuid, u64>,
    lifecycle: Lifecycle,
    pub(super) stamp: Option<HistoryStamp>,
    pub(super) tail: Option<PathBuf>,
}

impl Admission {
    pub(super) fn len(&self) -> usize {
        self.identities.len()
    }

    pub(super) fn existing(
        &self,
        record: &JournalRecord,
        encoded: &[u8],
    ) -> Result<Option<usize>, JournalError> {
        if let Some((ordinal, digest)) = self.identities.get(&record.id) {
            if *digest != <[u8; 32]>::from(Sha256::digest(encoded)) {
                return Err(JournalError::Identity);
            }
            return Ok(Some(*ordinal));
        }
        if self
            .coordinates
            .contains_key(&(record.identity.request_id, record.sequence))
        {
            return Err(JournalError::Identity);
        }
        if record.sequence
            != *self
                .sequences
                .get(&record.identity.request_id)
                .unwrap_or(&0)
        {
            return Err(JournalError::Incomplete);
        }
        Ok(None)
    }

    pub(super) fn check(&self, record: &JournalRecord) -> Result<Transition, JournalError> {
        self.lifecycle.check(record)
    }

    pub(super) fn commit(
        &mut self,
        record: &JournalRecord,
        encoded: &[u8],
        transition: Transition,
    ) {
        self.identities
            .insert(record.id, (self.len(), Sha256::digest(encoded).into()));
        self.coordinates
            .insert((record.identity.request_id, record.sequence), record.id);
        self.sequences
            .insert(record.identity.request_id, record.sequence + 1);
        self.lifecycle.commit(transition);
    }
}
