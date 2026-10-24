use super::manifest::SessionStore;
use super::manifest_commit::{read_manifest, ManifestCommitError};
use super::messages::durability::sync_parent;
use super::transcript::{JournalError, JournalRecord, JournalStore};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct RecoveryLease {
    root: PathBuf,
    session_id: Uuid,
    directory: PathBuf,
    _file: File,
}

impl RecoveryLease {
    pub fn acquire(root: &Path, session_id: Uuid) -> io::Result<Self> {
        let parent = root.join("persistence-recovery");
        fs::create_dir_all(&parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&parent, fs::Permissions::from_mode(0o700))?;
        }
        let directory = parent.join(session_id.to_string());
        let mut options = OpenOptions::new();
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(directory.with_extension("lock"))?;
        file.try_lock().map_err(io::Error::other)?;
        Ok(Self {
            root: root.into(),
            session_id,
            directory,
            _file: file,
        })
    }

    pub fn configured(session_id: Uuid) -> Result<Self, ManifestCommitError> {
        let root = codelet_common::get_data_dir().map_err(ManifestCommitError::Store)?;
        Ok(Self::acquire(&root, session_id)?)
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn session_exists(&self) -> Result<bool, ManifestCommitError> {
        let path = self
            .root
            .join("sessions")
            .join(format!("{}.json", self.session_id));
        match read_manifest(&path) {
            Ok(manifest) if manifest.id == self.session_id => Ok(true),
            Ok(_) => Err(ManifestCommitError::InvalidUpdate),
            Err(ManifestCommitError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    pub fn retire(&self) -> io::Result<()> {
        match fs::remove_dir_all(&self.directory) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        sync_parent(&self.directory)
    }

    pub(super) fn matches(&self, root: &Path, session_id: Uuid) -> bool {
        self.root == root && self.session_id == session_id
    }

    pub(super) fn preflight(&self) -> io::Result<()> {
        for path in [
            self.directory.clone(),
            self.root
                .join("transcripts")
                .join(self.session_id.to_string()),
        ] {
            match fs::symlink_metadata(path) {
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "private session lane is not a directory",
                    ))
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub(super) fn retire_deleted(&self) -> Result<(), ManifestCommitError> {
        let promoted = |source| ManifestCommitError::DeletionPromoted {
            session_id: self.session_id,
            source,
        };
        JournalStore::new(&self.root, self.session_id)
            .delete()
            .map_err(|error| promoted(io::Error::other(error)))?;
        super::messages::durability::perform("session.delete.recovery", || self.retire())
            .map_err(promoted)
    }
}

pub(super) fn append_owned(
    root: &Path,
    session_id: Uuid,
    record: &JournalRecord,
) -> Result<(), JournalError> {
    let mut sessions = SessionStore {
        sessions_dir: root.join("sessions"),
        leases: HashMap::new(),
        cache: HashMap::new(),
        last_session: HashMap::new(),
    };
    sessions
        .with_committed_session(session_id, || {
            JournalStore::new(root, session_id)
                .append(record)
                .map_err(ManifestCommitError::Transcript)
        })
        .map_err(|error| JournalError::SessionOwnership(Box::new(error)))
}
