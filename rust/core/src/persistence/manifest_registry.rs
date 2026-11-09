lazy_static::lazy_static! {
    pub(super) static ref MESSAGE_STORE: Mutex<Option<std::sync::Arc<MessageStore>>> = Mutex::new(None);
    pub(super) static ref SESSION_STORE: Mutex<Option<SessionStore>> = Mutex::new(None);
}

/// BUG-625: a shared handle to the message store. The daemon-wide lock is
/// held only to read or install the handle, never while the store is
/// created or used, so a panic in message store work never poisons it.
pub(super) fn message_store() -> Result<std::sync::Arc<MessageStore>, String> {
    let installed = || codelet_common::read_work::lock_named(&MESSAGE_STORE, "message store");
    if let Some(store) = installed()?.as_ref() {
        return Ok(std::sync::Arc::clone(store));
    }
    let created = std::sync::Arc::new(MessageStore::new()?);
    Ok(std::sync::Arc::clone(
        installed()?.get_or_insert(created),
    ))
}

pub(super) fn init_session_store() -> Result<(), String> {
    super::manifest_updates::ensure_session_access()?;
    let mut store = lock_store()?;
    if store.is_none() {
        *store = Some(SessionStore::new()?);
    }
    Ok(())
}

/// Reset the lifted MESSAGE_STORE, SESSION_STORE, BLOB_STORE and history singletons.
///
/// Called by `codelet_napi::persistence::set_data_directory` after it
/// updates the shared `codelet_common` data dir, so the next persistence
/// operation re-initialises against the new directory. Also used by
/// codelet-core integration tests that swap the data dir under the
/// process's feet.
pub fn reset_stores_for_tests() {
    if let Ok(mut msg) = MESSAGE_STORE.lock() {
        *msg = None;
    }
    if let Ok(mut sess) = SESSION_STORE.lock() {
        *sess = None;
    }
    crate::persistence::history::reset_for_tests();
    crate::persistence::blob::reset_blob_store_for_tests();
}

/// Reset only the MESSAGE_STORE singleton.
///
/// Used by the BUG-122 lazy-init tests in codelet-napi to force the
/// MessageStore to re-load from disk between operations without
/// disturbing the SESSION_STORE cache.
pub fn reset_message_store_for_tests() {
    if let Ok(mut msg) = MESSAGE_STORE.lock() {
        *msg = None;
    }
}

/// Reset only the SESSION_STORE singleton.
///
/// Symmetric counterpart to [`reset_message_store_for_tests`].
pub fn reset_session_store_for_tests() {
    if let Ok(mut sess) = SESSION_STORE.lock() {
        *sess = None;
    }
}

/// Test-only accessor: is the lifted MESSAGE_STORE singleton currently
/// initialised?
///
/// Used by `codelet_napi::persistence::lazy_init_tests` to verify
/// BUG-122's per-store lazy initialisation invariants without poking the
/// lazy_static globals directly. Matches the pattern set by
/// [`crate::persistence::history::is_initialized_for_tests`].
pub fn is_message_store_initialized_for_tests() -> bool {
    MESSAGE_STORE.lock().map(|s| s.is_some()).unwrap_or(false)
}

/// Test-only accessor: is the lifted SESSION_STORE singleton currently
/// initialised?
pub fn is_session_store_initialized_for_tests() -> bool {
    SESSION_STORE.lock().map(|s| s.is_some()).unwrap_or(false)
}

/// BUG-610: the shared store lock is poisoned (only in-memory work runs
/// under it, so this means a bug in that work, never a session's update).
pub(super) const SESSION_STORE_POISONED: &str = "the session store lock is poisoned";

/// BUG-610: lock the shared store for in-memory work only: the cache, the
/// last-session pointers and the writer lease map.
pub(super) fn lock_store() -> Result<std::sync::MutexGuard<'static, Option<SessionStore>>, String> {
    codelet_common::read_work::lock_named(&SESSION_STORE, "session store")
}

/// BUG-610: take `session_id`'s writer under the store lock and release
/// the lock before the writer runs any closure, I/O or commit.
pub(super) fn session_writer(
    session_id: Uuid,
) -> Result<super::manifest_commit::SessionWriter, super::manifest_commit::ManifestCommitError> {
    use super::manifest_commit::ManifestCommitError;
    SESSION_STORE
        .lock()
        .map_err(|_| ManifestCommitError::Store(SESSION_STORE_POISONED.into()))?
        .as_mut()
        .ok_or_else(|| ManifestCommitError::Store("not initialized".into()))?
        .writer(session_id)
}

/// BUG-610: cache a promoted manifest. The manifest on disk is already
/// authoritative, so a poisoned store only leaves the cache behind; that
/// is logged as an error, never skipped silently.
pub(super) fn record_committed(committed: &SessionManifest) {
    match SESSION_STORE.lock() {
        Ok(mut store) => {
            if let Some(store) = store.as_mut() {
                store.record(committed);
            }
        }
        Err(_) => tracing::error!(
            session_id = %committed.id,
            revision = committed.revision,
            "{SESSION_STORE_POISONED}: the committed manifest is not cached"
        ),
    }
}

/// Promote a whole manifest snapshot through its session's writer.
fn commit_snapshot(snapshot: &SessionManifest) -> Result<SessionManifest, String> {
    session_writer(snapshot.id)
        .and_then(|writer| writer.commit(snapshot, &mut record_committed))
        .map_err(|error| error.to_string())
}

fn sessions_dir() -> Result<PathBuf, String> {
    Ok(lock_store()?
        .as_ref()
        .ok_or("Session store not initialized")?
        .sessions_dir
        .clone())
}

/// Create a new session.
pub fn create_session(name: &str, project: &Path) -> Result<SessionManifest, String> {
    init_session_store()?;
    commit_snapshot(&SessionManifest::new(name, project.to_path_buf()))
}

/// Create a new session with provider.
pub fn create_session_with_provider(
    name: &str,
    project: &Path,
    provider: &str,
) -> Result<SessionManifest, String> {
    init_session_store()?;
    let session = SessionManifest::with_provider(name, project.to_path_buf(), provider);
    info!(session_id = %session.id, provider = %provider, name = %name, "create_session_with_provider: created new session manifest");
    commit_snapshot(&session)
}

/// Save a session manifest to disk (for externally-created manifests).
pub fn save_session(session: &SessionManifest) -> Result<(), String> {
    init_session_store()?;
    commit_snapshot(session).map(|_| ())
}

/// Load a session by ID.
pub fn load_session(id: Uuid) -> Result<SessionManifest, String> {
    init_session_store()?;
    load_manifest(&sessions_dir()?, id)
}

/// Resume the last session for a project.
pub fn resume_last_session(project: &Path) -> Result<SessionManifest, String> {
    init_session_store()?;
    lock_store()?
        .as_ref()
        .ok_or("Session store not initialized")?
        .resume_last(project)
}

/// Fork a session at a specific message index.
pub fn fork_session(
    session: &SessionManifest,
    at_index: usize,
    name: &str,
) -> Result<SessionManifest, String> {
    init_session_store()?;
    commit_snapshot(&fork_manifest(session, at_index, name)?)
}
