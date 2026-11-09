use super::manifest::{SessionStore, SESSION_STORE};
use std::collections::HashMap;
use uuid::Uuid;

impl SessionStore {
    fn release_lease(&mut self, session_id: Uuid) {
        self.leases.remove(&session_id);
    }

    pub(super) fn evict(&mut self, session_id: Uuid) {
        self.release_lease(session_id);
        if let Some(session) = self.cache.remove(&session_id) {
            if self.last_session.get(&session.project) == Some(&session_id) {
                self.last_session.remove(&session.project);
            }
        }
    }
}

/// BUG-288: drop this process's manifest writer lease for a session so
/// another process can take it, e.g. after the session is destroyed. A no-op
/// when the store is not initialised.
///
/// BUG-333: only the lease is dropped. The cached manifest and the project's
/// last-session pointer stay, because listing and `resume_last` read them: a
/// destroyed session stays listed and resumable. Deleting a session evicts it.
///
/// BUG-610: this is also how a session whose writer lock was poisoned by a
/// panic recovers: the poisoned lease is dropped with it, and the reopened
/// session takes a fresh writer over the manifest on disk.
pub fn release_session_ownership(session_id: Uuid) {
    match SESSION_STORE.lock() {
        Ok(mut store) => {
            if let Some(store) = store.as_mut() {
                store.release_lease(session_id);
            }
        }
        Err(_) => tracing::error!(
            %session_id,
            "{}: the session's writer ownership cannot be released",
            super::manifest::SESSION_STORE_POISONED
        ),
    }
}

pub(super) fn delete_current_session(session_id: Uuid) -> Result<(), String> {
    super::manifest_updates::ensure_session_access()?;
    let root = codelet_common::get_data_dir()?;
    let recovery = super::session_lifecycle::RecoveryLease::acquire(&root, session_id)
        .map_err(|error| error.to_string())?;
    let sessions_dir = root.join("sessions");
    // BUG-610: the store lock is held only to take the writer; the
    // deletion runs under the session's writer lock alone.
    let writer = {
        let mut singleton = SESSION_STORE
            .lock()
            .map_err(|_| super::manifest::SESSION_STORE_POISONED.to_string())?;
        match singleton.as_mut() {
            Some(store) if store.sessions_dir == sessions_dir => Some(
                store
                    .writer(session_id)
                    .map_err(|error| error.to_string())?,
            ),
            _ => None,
        }
    };
    if let Some(writer) = writer {
        return writer
            .delete_with_recovery_lease(&recovery, &mut || evict_cached(session_id))
            .map_err(|error| error.to_string());
    }
    if sessions_dir.exists() {
        SessionStore {
            sessions_dir,
            leases: HashMap::new(),
            cache: HashMap::new(),
            last_session: HashMap::new(),
        }
        .delete_with_recovery_lease(session_id, &recovery)
        .map_err(|error| error.to_string())?;
    } else {
        recovery.preflight().map_err(|error| error.to_string())?;
        recovery
            .retire_deleted()
            .map_err(|error| error.to_string())?;
    }
    evict_cached(session_id);
    Ok(())
}

fn evict_cached(session_id: Uuid) {
    match SESSION_STORE.lock() {
        Ok(mut store) => {
            if let Some(store) = store.as_mut() {
                store.evict(session_id);
            }
        }
        Err(_) => tracing::error!(
            %session_id,
            "{}: the deleted session stays cached",
            super::manifest::SESSION_STORE_POISONED
        ),
    }
}
