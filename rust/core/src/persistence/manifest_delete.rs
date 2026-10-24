use super::manifest::{SessionStore, SESSION_STORE};
use std::collections::HashMap;
use uuid::Uuid;

impl SessionStore {
    pub(super) fn evict(&mut self, session_id: Uuid) {
        self.leases.remove(&session_id);
        if let Some(session) = self.cache.remove(&session_id) {
            if self.last_session.get(&session.project) == Some(&session_id) {
                self.last_session.remove(&session.project);
            }
        }
    }
}

pub(super) fn delete_current_session(session_id: Uuid) -> Result<(), String> {
    super::manifest_updates::ensure_session_access()?;
    let root = codelet_common::get_data_dir()?;
    let recovery = super::session_lifecycle::RecoveryLease::acquire(&root, session_id)
        .map_err(|error| error.to_string())?;
    let sessions_dir = root.join("sessions");
    let mut singleton = SESSION_STORE.lock().map_err(|error| error.to_string())?;
    if let Some(store) = singleton.as_mut() {
        if store.sessions_dir == sessions_dir {
            return store
                .delete_with_recovery_lease(session_id, &recovery)
                .map_err(|error| error.to_string());
        }
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
    if let Some(store) = singleton.as_mut() {
        store.evict(session_id);
    }
    Ok(())
}
