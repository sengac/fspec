use super::super::{
    get_message, original_blobs::restore_original_blobs, provider_state::legacy, StoredMessage,
};
use super::{ArchiveError, ArchiveRecordKind};
use serde_json::{json, Value};
use uuid::Uuid;

pub(super) fn read(
    message_id: Uuid,
) -> Result<(StoredMessage, ArchiveRecordKind, String), ArchiveError> {
    let stored = get_message(message_id)
        .map_err(|message| ArchiveError::SourceUnavailable { message })?
        .ok_or(ArchiveError::MissingMessage { message_id })?;
    let (kind, mut envelope) = if let Some(marker) =
        legacy::marker(&stored).map_err(|error| ArchiveError::InvalidEnvelope {
            message_id,
            message: error.to_string(),
        })? {
        (ArchiveRecordKind::ProviderState, marker)
    } else {
        let mut envelope = Value::Object(stored.metadata.clone().into_iter().collect());
        let kind = if let Some(message) = envelope.get("message") {
            if !message.is_object() {
                return Err(ArchiveError::InvalidEnvelope {
                    message_id,
                    message: "message must be structured".into(),
                });
            }
            ArchiveRecordKind::Conversation
        } else {
            envelope["message"] =
                json!({"role":stored.role,"content":[{"type":"text","text":stored.content}]});
            envelope["recordKind"] = json!("legacy_text");
            ArchiveRecordKind::LegacyText
        };
        let fields = envelope
            .as_object_mut()
            .ok_or_else(|| ArchiveError::InvalidEnvelope {
                message_id,
                message: "envelope must be structured".into(),
            })?;
        fields.entry("uuid").or_insert(json!(stored.id));
        fields
            .entry("timestamp")
            .or_insert(json!(stored.created_at.to_rfc3339()));
        fields.entry("type").or_insert(json!(stored.role));
        fields.entry("provider").or_insert(json!("unknown"));
        restore_original_blobs(&mut envelope).map_err(|error| ArchiveError::InvalidEnvelope {
            message_id,
            message: error.to_string(),
        })?;
        (kind, envelope)
    };
    envelope.sort_all_objects();
    Ok((stored, kind, serde_json::to_string(&envelope)?))
}
