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
    /// BUG-610: an earlier update of this session panicked while it held
    /// the session's writer lock. Only this session is affected.
    #[error(
        "the manifest writer lock of session {session_id} is poisoned: an earlier update of \
         this session panicked; close and reopen the session to recover"
    )]
    WriterPoisoned { session_id: Uuid },
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

/// BUG-610: called with every manifest a writer promoted, while the
/// session's writer lock is still held, so the store's cache sees one
/// session's revisions in commit order.
pub(super) type Recorder<'record> = &'record mut dyn FnMut(&SessionManifest);

/// BUG-610: one session's manifest writer. It is taken from the store
/// under the store lock and used after that lock is released: the update
/// closure, the manifest I/O and the promotion run under this session's
/// own writer lock only, so a panic in one session's work poisons that
/// session's writer, never the store every session shares.
pub(super) struct SessionWriter {
    sessions_dir: PathBuf,
    session_id: Uuid,
    lease: Arc<WriterLease>,
}

impl SessionWriter {
    pub(super) fn manifest_path(&self) -> PathBuf {
        self.sessions_dir.join(format!("{}.json", self.session_id))
    }

    fn locked<Output>(
        &self,
        operation: impl FnOnce() -> Result<Output, ManifestCommitError>,
    ) -> Result<Output, ManifestCommitError> {
        let _ownership = super::manifest_updates::SessionWriterGuard::enter(self.session_id)?;
        // A poisoned writer lock is never recovered here (lock_state rule,
        // BUG-369): the session reports it until it is closed, which drops
        // this lease (release_session_ownership), and reopened.
        let _guard = self
            .lease
            .gate
            .lock()
            .map_err(|_| ManifestCommitError::WriterPoisoned {
                session_id: self.session_id,
            })?;
        operation()
    }

    pub(super) fn commit(
        &self,
        snapshot: &SessionManifest,
        record: Recorder<'_>,
    ) -> Result<SessionManifest, ManifestCommitError> {
        if snapshot.id != self.session_id {
            return Err(ManifestCommitError::InvalidUpdate);
        }
        self.locked(|| commit_locked(&self.manifest_path(), snapshot, record))
    }

    pub(super) fn update(
        &self,
        update: impl FnOnce(&mut SessionManifest) -> Result<(), ManifestCommitError>,
        record: Recorder<'_>,
    ) -> Result<SessionManifest, ManifestCommitError> {
        self.locked(|| {
            let path = self.manifest_path();
            let mut snapshot = read_manifest(&path)?;
            let revision = snapshot.revision;
            update(&mut snapshot)?;
            if snapshot.id != self.session_id || snapshot.revision != revision {
                return Err(ManifestCommitError::InvalidUpdate);
            }
            commit_locked(&path, &snapshot, record)
        })
    }

    pub(super) fn with_committed_session<Output>(
        &self,
        operation: impl FnOnce() -> Result<Output, ManifestCommitError>,
    ) -> Result<Output, ManifestCommitError> {
        self.locked(|| {
            let current = read_manifest(&self.manifest_path())?;
            if current.id != self.session_id {
                return Err(ManifestCommitError::InvalidUpdate);
            }
            operation()
        })
    }

    /// Delete the manifest; `evict` runs once the manifest file is gone.
    pub(super) fn delete_with_recovery_lease(
        &self,
        recovery: &super::session_lifecycle::RecoveryLease,
        evict: &mut dyn FnMut(),
    ) -> Result<(), ManifestCommitError> {
        let session_id = self.session_id;
        self.locked(|| {
            let data_root = self.sessions_dir.parent().ok_or_else(|| {
                ManifestCommitError::Store("session store has no data root".into())
            })?;
            if !recovery.matches(data_root, session_id) {
                return Err(ManifestCommitError::InvalidUpdate);
            }
            recovery.preflight()?;
            let path = self.manifest_path();
            perform("session.delete.manifest", || {
                match fs::remove_file(&path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
                Ok(())
            })?;
            evict();
            perform("session.delete.directory", || {
                super::messages::durability::sync_parent(&path)
            })
            .map_err(|source| ManifestCommitError::DeletionPromoted { session_id, source })?;
            recovery.retire_deleted()
        })
    }
}

impl SessionStore {
    fn writer_lease(&mut self, session_id: Uuid) -> Result<Arc<WriterLease>, ManifestCommitError> {
        if let Some(lease) = self.leases.get(&session_id) {
            return Ok(lease.clone());
        }
        let path = self
            .sessions_dir
            .canonicalize()?
            .join(format!("{session_id}.lock"));
        let mut leases = LEASES.lock().map_err(|_| {
            ManifestCommitError::Store("the session writer lease registry lock is poisoned".into())
        })?;
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

    /// BUG-610: this session's writer; use it after releasing any lock on
    /// this store.
    pub(super) fn writer(
        &mut self,
        session_id: Uuid,
    ) -> Result<SessionWriter, ManifestCommitError> {
        Ok(SessionWriter {
            sessions_dir: self.sessions_dir.clone(),
            session_id,
            lease: self.writer_lease(session_id)?,
        })
    }

    /// Cache a manifest a writer promoted.
    pub(super) fn record(&mut self, committed: &SessionManifest) {
        self.cache.insert(committed.id, committed.clone());
        self.last_session
            .insert(committed.project.clone(), committed.id);
    }

    pub fn commit(
        &mut self,
        snapshot: &SessionManifest,
    ) -> Result<SessionManifest, ManifestCommitError> {
        let writer = self.writer(snapshot.id)?;
        writer.commit(snapshot, &mut |committed| self.record(committed))
    }

    pub fn update(
        &mut self,
        session_id: Uuid,
        update: impl FnOnce(&mut SessionManifest) -> Result<(), ManifestCommitError>,
    ) -> Result<SessionManifest, ManifestCommitError> {
        let writer = self.writer(session_id)?;
        writer.update(update, &mut |committed| self.record(committed))
    }

    pub(super) fn with_committed_session<Output>(
        &mut self,
        session_id: Uuid,
        operation: impl FnOnce() -> Result<Output, ManifestCommitError>,
    ) -> Result<Output, ManifestCommitError> {
        self.writer(session_id)?.with_committed_session(operation)
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
        let writer = self.writer(session_id)?;
        writer.delete_with_recovery_lease(recovery, &mut || self.evict(session_id))
    }
}

/// Promote `snapshot` as the next revision of the manifest at `path`.
/// Runs under the session's writer lock; `record` sees the promoted
/// manifest even when only its durability acknowledgement failed.
fn commit_locked(
    path: &Path,
    snapshot: &SessionManifest,
    record: Recorder<'_>,
) -> Result<SessionManifest, ManifestCommitError> {
    let actual = match fs::read(path) {
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
    let result = promote(path, &committed);
    if result.is_ok() || matches!(&result, Err(ManifestCommitError::Promoted { .. })) {
        record(&committed);
    }
    result?;
    Ok(committed)
}

pub(super) fn read_manifest(path: &Path) -> Result<SessionManifest, ManifestCommitError> {
    Ok(serde_json::from_slice(
        &codelet_common::read_work::read_file(path, "manifest.bytes")?,
    )?)
}

/// BUG-284: retry the post-rename durability barrier for a promoted manifest
/// whose acknowledgment was lost, under the session writer ownership.
pub fn reacknowledge_session_directory(session_id: Uuid) -> Result<(), ManifestCommitError> {
    let _ownership = super::manifest_updates::SessionWriterGuard::enter(session_id)?;
    let directory = codelet_common::get_data_dir()
        .map_err(ManifestCommitError::Store)?
        .join("sessions");
    perform("manifest.directory", || {
        File::open(&directory).and_then(|directory| directory.sync_all())
    })
    .map_err(|source| ManifestCommitError::Promoted {
        session_id,
        revision: 0,
        source,
    })
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
