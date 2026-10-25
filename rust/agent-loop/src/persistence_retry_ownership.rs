use super::{PersistenceSpool, RecoveryOwner, OWNERS};
use codelet_core::persistence::session_lifecycle::RecoveryLease;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub(super) fn owner(directory: &Path) -> Result<Arc<Mutex<RecoveryOwner>>, String> {
    let mut owners = OWNERS.lock().map_err(|_| "recovery registry poisoned")?;
    if !owners.contains_key(directory) && owners.len() >= 128 {
        owners.retain(|_, lane| {
            Arc::strong_count(lane) > 1
                || lane.try_lock().map_or(true, |owner| {
                    !owner.admitted.is_empty() || !owner.retained.is_empty()
                })
        });
    }
    Ok(owners.entry(directory.into()).or_default().clone())
}

impl PersistenceSpool {
    pub(super) fn lease(&self, owner: &mut RecoveryOwner) -> Result<Option<RecoveryLease>, String> {
        let root = self
            .directory
            .parent()
            .and_then(Path::parent)
            .ok_or("recovery directory has no data root")?;
        let lease = RecoveryLease::acquire(root, self.session_id)
            .map_err(|error| format!("recovery owner unavailable: {error}"))?;
        if lease.session_exists().map_err(|error| error.to_string())? {
            return Ok(Some(lease));
        }
        lease.retire().map_err(|error| error.to_string())?;
        *owner = RecoveryOwner::default();
        Ok(None)
    }
}
