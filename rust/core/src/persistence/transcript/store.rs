use super::{JournalProjection, JournalRecord};
use crate::persistence::messages::durability::{perform, sync_parent};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

#[path = "writer.rs"]
mod writer;

const MAX_RECORD_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("transcript storage unavailable")]
    Io(#[from] std::io::Error),
    #[error("transcript record cannot be decoded")]
    Json(#[from] serde_json::Error),
    #[error("incompatible transcript version")]
    Version,
    #[error("transcript identity conflict")]
    Identity,
    #[error("transcript record exceeds admission capacity")]
    Capacity,
    #[error("transcript data directory unavailable")]
    Configuration,
    #[error("transcript lifecycle is inconsistent")]
    Lifecycle,
    #[error("transcript acknowledged sequence is incomplete")]
    Incomplete,
    #[error("session transcript ownership unavailable: {0}")]
    SessionOwnership(#[source] Box<crate::persistence::ManifestCommitError>),
}

pub struct JournalStore {
    session_id: Uuid,
    directory: PathBuf,
    session_owned: bool,
    admission: Mutex<Option<super::admission::Admission>>,
}

impl JournalStore {
    pub fn new(data_root: &Path, session_id: Uuid) -> Self {
        Self {
            session_id,
            directory: data_root.join("transcripts").join(session_id.to_string()),
            session_owned: false,
            admission: Mutex::new(None),
        }
    }

    pub fn configured(session_id: Uuid) -> Result<Self, JournalError> {
        let directory = codelet_common::get_data_dir().map_err(|_| JournalError::Configuration)?;
        let mut store = Self::new(&directory, session_id);
        store.session_owned = true;
        Ok(store)
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    fn validate(&self, record: &JournalRecord) -> Result<(), JournalError> {
        if record.version != 1 {
            return Err(JournalError::Version);
        }
        if record.identity.session_id != self.session_id {
            return Err(JournalError::Identity);
        }
        Ok(())
    }

    fn destination(&self, ordinal: usize, record: &JournalRecord) -> PathBuf {
        self.directory.join(format!(
            "{ordinal:020}-{}-{:020}.json",
            record.identity.request_id, record.sequence
        ))
    }

    fn paths(&self) -> Result<Vec<PathBuf>, JournalError> {
        let mut paths = Vec::new();
        for entry in fs::read_dir(&self.directory)? {
            let path = entry?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                paths.push(path);
            }
        }
        paths.sort();
        for (ordinal, path) in paths.iter().enumerate() {
            let prefix = format!("{ordinal:020}-");
            if !path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
            {
                return Err(JournalError::Incomplete);
            }
        }
        Ok(paths)
    }

    fn lease(&self) -> Result<File, JournalError> {
        fs::create_dir_all(&self.directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if fs::metadata(&self.directory)?.permissions().mode() & 0o777 != 0o700 {
                fs::set_permissions(&self.directory, fs::Permissions::from_mode(0o700))?;
            }
        }
        let mut options = OpenOptions::new();
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lease = options.open(self.directory.join("writer.lock"))?;
        lease.lock()?;
        Ok(lease)
    }

    pub fn append(&self, record: &JournalRecord) -> Result<(), JournalError> {
        self.validate(record)?;
        if self.session_owned {
            let root = self
                .directory
                .parent()
                .and_then(Path::parent)
                .ok_or(JournalError::Configuration)?;
            return crate::persistence::session_lifecycle::append_owned(
                root,
                self.session_id,
                record,
                self,
            );
        }
        self.append_unowned(record)
    }

    pub fn read(&self) -> Result<Vec<JournalRecord>, JournalError> {
        if !self.directory.try_exists()? {
            return Ok(Vec::new());
        }
        let _lease = self.lease()?;
        let records = self.read_paths(&self.paths()?)?;
        JournalProjection::from_records(records.clone(), &HashSet::new())?;
        Ok(records)
    }

    fn read_paths(&self, paths: &[PathBuf]) -> Result<Vec<JournalRecord>, JournalError> {
        let mut records = Vec::new();
        let mut identities = HashSet::new();
        let mut sequences = std::collections::HashMap::new();
        for (ordinal, path) in paths.iter().enumerate() {
            if fs::metadata(path)?.len() > MAX_RECORD_BYTES {
                return Err(JournalError::Capacity);
            }
            let record: JournalRecord =
                serde_json::from_slice(&perform("transcript.history.read", || fs::read(path))?)?;
            self.validate(&record)?;
            if self.destination(ordinal, &record) != *path || !identities.insert(record.id) {
                return Err(JournalError::Identity);
            }
            let sequence = sequences.entry(record.identity.request_id).or_insert(0);
            if record.sequence != *sequence {
                return Err(JournalError::Incomplete);
            }
            *sequence += 1;
            records.push(record);
        }
        Ok(records)
    }

    pub fn project(&self, originals: &HashSet<Uuid>) -> Result<JournalProjection, JournalError> {
        JournalProjection::from_records(self.read()?, originals)
    }

    pub fn export(&self, originals: &HashSet<Uuid>) -> Result<String, JournalError> {
        Ok(serde_json::to_string(&self.project(originals)?.records)?)
    }

    pub fn delete(&self) -> Result<(), JournalError> {
        if !self.directory.try_exists()? {
            return Ok(());
        }
        let _lease = self.lease()?;
        fs::remove_dir_all(&self.directory)?;
        sync_parent(&self.directory)?;
        Ok(())
    }
}
