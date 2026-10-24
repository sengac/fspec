use super::FramingError;
use crate::persistence::SessionManifest;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use uuid::Uuid;

pub(super) fn committed_references(directory: &Path) -> Result<HashSet<Uuid>, FramingError> {
    let entries = fs::read_dir(directory).map_err(|error| {
        FramingError::Evidence(format!(
            "cannot read committed sessions {directory:?}: {error}"
        ))
    })?;
    let mut referenced = HashSet::new();
    for entry in entries {
        let path = entry?.path();
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        let session: SessionManifest =
            serde_json::from_slice(&fs::read(&path)?).map_err(|error| {
                FramingError::Evidence(format!("invalid manifest {path:?}: {error}"))
            })?;
        referenced.extend(session.messages.into_iter().map(|entry| entry.message_id));
    }
    Ok(referenced)
}
