use super::manifest::{init_message_store, MESSAGE_STORE};
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
    init_message_store().map_err(ManifestCommitError::Store)?;
    update_session_manifest(session_id, |current| {
        MESSAGE_STORE
            .lock()
            .map_err(|error| ManifestCommitError::Store(error.to_string()))?
            .as_mut()
            .ok_or_else(|| ManifestCommitError::Store("message store unavailable".into()))?
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
