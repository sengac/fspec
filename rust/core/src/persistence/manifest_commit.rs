use super::manifest::{SessionManifest, SessionStore};
use super::messages::durability::perform;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, Weak};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ManifestCommitError {
    #[error("session store unavailable: {0}")]
    Store(String),
    #[error("session ownership unavailable for {session_id}: {reason}")]
    Ownership { session_id: Uuid, reason: String },
    #[error(
        "manifest revision conflict for {session_id}: expected {expected}, committed {actual}"
    )]
    Conflict {
        session_id: Uuid,
        expected: u64,
        actual: u64,
    },
    #[error("manifest I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("incompatible committed manifest: {0}")]
    Json(#[from] serde_json::Error),
    #[error("session transcript cleanup failed: {0}")]
    Transcript(#[from] super::transcript::JournalError),
    #[error("session {session_id} deletion promoted; private cleanup remains retryable: {source}")]
    DeletionPromoted { session_id: Uuid, source: io::Error },
    #[error("session revision exhausted")]
    RevisionExhausted,
    #[error("session update changed its identity or source revision")]
    InvalidUpdate,
    #[error("durable promotion is unsupported on this platform")]
    UnsupportedPlatform,
    #[error("revision {revision} for {session_id} was promoted; durability acknowledgement failed: {source}")]
    Promoted {
        session_id: Uuid,
        revision: u64,
        source: io::Error,
    },
}

pub(super) struct WriterLease {
    _file: File,
    gate: Mutex<()>,
}

static LEASES: LazyLock<Mutex<HashMap<PathBuf, Weak<WriterLease>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

impl SessionStore {
    fn writer_lease(&mut self, session_id: Uuid) -> Result<Arc<WriterLease>, ManifestCommitError> {
        if let Some(lease) = self.leases.get(&session_id) {
            return Ok(lease.clone());
        }
        let path = self
            .sessions_dir
            .canonicalize()?
            .join(format!("{session_id}.lock"));
        let mut leases = LEASES
            .lock()
            .map_err(|error| ManifestCommitError::Store(error.to_string()))?;
        let lease = match leases.get(&path).and_then(Weak::upgrade) {
            Some(lease) => lease,
            None => {
                let file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&path)?;
                file.try_lock()
                    .map_err(|error| ManifestCommitError::Ownership {
                        session_id,
                        reason: error.to_string(),
                    })?;
                let lease = Arc::new(WriterLease {
                    _file: file,
                    gate: Mutex::new(()),
                });
                leases.insert(path, Arc::downgrade(&lease));
                lease
            }
        };
        self.leases.insert(session_id, lease.clone());
        Ok(lease)
    }

    pub fn commit(
        &mut self,
        snapshot: &SessionManifest,
    ) -> Result<SessionManifest, ManifestCommitError> {
        let _ownership = super::manifest_updates::SessionWriterGuard::enter(snapshot.id)?;
        let lease = self.writer_lease(snapshot.id)?;
        let _guard = lease
            .gate
            .lock()
            .map_err(|error| ManifestCommitError::Store(error.to_string()))?;
        self.commit_locked(snapshot)
    }

    pub fn update(
        &mut self,
        session_id: Uuid,
        update: impl FnOnce(&mut SessionManifest) -> Result<(), ManifestCommitError>,
    ) -> Result<SessionManifest, ManifestCommitError> {
        let _ownership = super::manifest_updates::SessionWriterGuard::enter(session_id)?;
        let lease = self.writer_lease(session_id)?;
        let _guard = lease
            .gate
            .lock()
            .map_err(|error| ManifestCommitError::Store(error.to_string()))?;
        let mut snapshot = read_manifest(&self.sessions_dir.join(format!("{session_id}.json")))?;
        let revision = snapshot.revision;
        update(&mut snapshot)?;
        if snapshot.id != session_id || snapshot.revision != revision {
            return Err(ManifestCommitError::InvalidUpdate);
        }
        self.commit_locked(&snapshot)
    }

    pub(super) fn with_committed_session<Output>(
        &mut self,
        session_id: Uuid,
        operation: impl FnOnce() -> Result<Output, ManifestCommitError>,
    ) -> Result<Output, ManifestCommitError> {
        let _ownership = super::manifest_updates::SessionWriterGuard::enter(session_id)?;
        let lease = self.writer_lease(session_id)?;
        let _guard = lease
            .gate
            .lock()
            .map_err(|error| ManifestCommitError::Store(error.to_string()))?;
        let current = read_manifest(&self.sessions_dir.join(format!("{session_id}.json")))?;
        if current.id != session_id {
            return Err(ManifestCommitError::InvalidUpdate);
        }
        operation()
    }

    pub(super) fn delete_owned(&mut self, session_id: Uuid) -> Result<(), ManifestCommitError> {
        let root = self
            .sessions_dir
            .parent()
            .ok_or(ManifestCommitError::InvalidUpdate)?;
        let recovery = super::session_lifecycle::RecoveryLease::acquire(root, session_id)?;
        self.delete_with_recovery_lease(session_id, &recovery)
    }

    pub(super) fn delete_with_recovery_lease(
        &mut self,
        session_id: Uuid,
        recovery: &super::session_lifecycle::RecoveryLease,
    ) -> Result<(), ManifestCommitError> {
        let _ownership = super::manifest_updates::SessionWriterGuard::enter(session_id)?;
        let lease = self.writer_lease(session_id)?;
        let _guard = lease
            .gate
            .lock()
            .map_err(|error| ManifestCommitError::Store(error.to_string()))?;
        let data_root = self
            .sessions_dir
            .parent()
            .ok_or_else(|| ManifestCommitError::Store("session store has no data root".into()))?;
        if !recovery.matches(data_root, session_id) {
            return Err(ManifestCommitError::InvalidUpdate);
        }
        recovery.preflight()?;
        let path = self.sessions_dir.join(format!("{session_id}.json"));
        perform("session.delete.manifest", || {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            Ok(())
        })?;
        self.evict(session_id);
        perform("session.delete.directory", || {
            super::messages::durability::sync_parent(&path)
        })
        .map_err(|source| ManifestCommitError::DeletionPromoted { session_id, source })?;
        recovery.retire_deleted()
    }

    fn commit_locked(
        &mut self,
        snapshot: &SessionManifest,
    ) -> Result<SessionManifest, ManifestCommitError> {
        let path = self.sessions_dir.join(format!("{}.json", snapshot.id));
        let actual = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<SessionManifest>(&bytes)?.revision,
            Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
            Err(error) => return Err(error.into()),
        };
        if snapshot.revision != actual {
            return Err(ManifestCommitError::Conflict {
                session_id: snapshot.id,
                expected: snapshot.revision,
                actual,
            });
        }
        let mut committed = snapshot.clone();
        committed.revision = actual
            .checked_add(1)
            .ok_or(ManifestCommitError::RevisionExhausted)?;
        committed.updated_at = chrono::Utc::now();
        let result = promote(&path, &committed);
        if result.is_ok() || matches!(&result, Err(ManifestCommitError::Promoted { .. })) {
            self.cache.insert(committed.id, committed.clone());
            self.last_session
                .insert(committed.project.clone(), committed.id);
        }
        result?;
        Ok(committed)
    }
}

pub(super) fn read_manifest(path: &Path) -> Result<SessionManifest, ManifestCommitError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

#[cfg(unix)]
fn promote(path: &Path, committed: &SessionManifest) -> Result<(), ManifestCommitError> {
    let staging = path.with_extension(format!("{}.stage", Uuid::new_v4()));
    let prepare = || -> Result<(), ManifestCommitError> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staging)?;
        file.write_all(&serde_json::to_vec_pretty(committed)?)?;
        perform("manifest.sync", || file.sync_all())?;
        perform("manifest.rename", || fs::rename(&staging, path))?;
        Ok(())
    };
    if let Err(error) = prepare() {
        if let Err(cleanup) = fs::remove_file(&staging) {
            if cleanup.kind() != io::ErrorKind::NotFound {
                tracing::warn!(%cleanup, "could not remove uncommitted manifest stage");
            }
        }
        return Err(error);
    }
    let directory = path.parent().ok_or(ManifestCommitError::InvalidUpdate)?;
    perform("manifest.directory", || {
        File::open(directory).and_then(|directory| directory.sync_all())
    })
    .map_err(|source| ManifestCommitError::Promoted {
        session_id: committed.id,
        revision: committed.revision,
        source,
    })
}

#[cfg(not(unix))]
fn promote(_path: &Path, _committed: &SessionManifest) -> Result<(), ManifestCommitError> {
    Err(ManifestCommitError::UnsupportedPlatform)
}
