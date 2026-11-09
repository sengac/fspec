use super::manifest::message_store;
use super::{update_session_manifest, ManifestCommitError, MessageSource};
use std::collections::HashMap;
use uuid::Uuid;

pub fn append_message_idempotent(
    session_id: Uuid,
    id: Uuid,
    role: &str,
    content: &str,
    metadata: HashMap<String, serde_json::Value>,
) -> Result<Uuid, ManifestCommitError> {
    let store = message_store().map_err(ManifestCommitError::Store)?;
    // BUG-625: under this session's writer lock only (no daemon-wide lock).
    update_session_manifest(session_id, |current| {
        store
            .store_with_id(id, role, content, metadata)
            .map_err(ManifestCommitError::Store)?;
        if !current
            .messages
            .iter()
            .any(|message| message.message_id == id)
        {
            current.add_message(id, MessageSource::Native);
        }
        Ok(())
    })?;
    Ok(id)
}

/// BUG-284: retain an immutable original in the message store without
/// conversation membership. Same-identity retries are idempotent.
pub fn store_original_idempotent(
    id: Uuid,
    role: &str,
    content: &str,
    metadata: HashMap<String, serde_json::Value>,
) -> Result<Uuid, ManifestCommitError> {
    message_store()
        .map_err(ManifestCommitError::Store)?
        .store_with_id(id, role, content, metadata)
        .map_err(ManifestCommitError::Store)?;
    Ok(id)
}

/// BUG-284: append an already retained original to conversation membership
/// exactly once inside one manifest transaction.
pub fn admit_stored_message(
    session_id: Uuid,
    id: Uuid,
) -> Result<super::SessionManifest, ManifestCommitError> {
    let store = message_store().map_err(ManifestCommitError::Store)?;
    update_session_manifest(session_id, |current| {
        let stored = store.get(id);
        if stored.is_none() {
            return Err(ManifestCommitError::Store(format!(
                "retained original {id} is unavailable for admission"
            )));
        }
        if !current
            .messages
            .iter()
            .any(|message| message.message_id == id)
        {
            current.add_message(id, MessageSource::Native);
        }
        Ok(())
    })
}
