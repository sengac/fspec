use super::super::{get_message, SessionManifest};
use super::ArchiveError;
use sha2::{Digest, Sha256};

pub(super) fn hash(manifest: &SessionManifest, end: usize) -> Result<String, ArchiveError> {
    let references = manifest
        .messages
        .get(..end)
        .ok_or(ArchiveError::SourceChanged)?;
    let mut digest = Sha256::new();
    digest.update(manifest.id.as_bytes());
    for reference in references {
        let stored = get_message(reference.message_id)
            .map_err(|message| ArchiveError::SourceUnavailable { message })?
            .ok_or(ArchiveError::MissingMessage {
                message_id: reference.message_id,
            })?;
        let mut original = serde_json::to_value(stored)?;
        original.sort_all_objects();
        let identity = serde_json::to_vec(reference)?;
        let bytes = serde_json::to_vec(&original)?;
        digest.update((identity.len() as u64).to_le_bytes());
        digest.update(identity);
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    }
    Ok(hex::encode(digest.finalize()))
}
