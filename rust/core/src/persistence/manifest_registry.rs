lazy_static::lazy_static! {
    pub(super) static ref MESSAGE_STORE: Mutex<Option<MessageStore>> = Mutex::new(None);
    pub(super) static ref SESSION_STORE: Mutex<Option<SessionStore>> = Mutex::new(None);
}

pub(super) fn init_message_store() -> Result<(), String> {
    let mut store = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    if store.is_none() {
        *store = Some(MessageStore::new()?);
    }
    Ok(())
}

pub(super) fn init_session_store() -> Result<(), String> {
    super::manifest_updates::ensure_session_access()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
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

/// Create a new session.
pub fn create_session(name: &str, project: &Path) -> Result<SessionManifest, String> {
    init_session_store()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .create(name, project)
}

/// Create a new session with provider.
pub fn create_session_with_provider(
    name: &str,
    project: &Path,
    provider: &str,
) -> Result<SessionManifest, String> {
    init_session_store()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .create_with_provider(name, project, provider)
}

/// Save a session manifest to disk (for externally-created manifests).
pub fn save_session(session: &SessionManifest) -> Result<(), String> {
    init_session_store()?;
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .save(session)
}

/// Load a session by ID.
pub fn load_session(id: Uuid) -> Result<SessionManifest, String> {
    init_session_store()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_ref()
        .ok_or("Session store not initialized")?
        .load(id)
}

/// Resume the last session for a project.
pub fn resume_last_session(project: &Path) -> Result<SessionManifest, String> {
    init_session_store()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
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
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store
        .as_mut()
        .ok_or("Session store not initialized")?
        .fork(session, at_index, name)
}
