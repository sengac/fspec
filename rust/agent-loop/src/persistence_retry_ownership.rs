use super::{PersistenceSpool, RecoveryOwner};
use codelet_core::persistence::session_lifecycle::RecoveryLease;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

impl PersistenceSpool {
    pub(super) fn lease(
        &self,
        owners: &mut HashMap<PathBuf, RecoveryOwner>,
    ) -> Result<Option<RecoveryLease>, String> {
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
        owners.remove(&self.directory);
        Ok(None)
    }
}
