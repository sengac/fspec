use super::RecoveryRecord;
use codelet_core::persistence::transcript::JournalStore;
use codelet_core::persistence::{
    conversation_commit::{append_message_idempotent, store_original_idempotent},
    process_envelope_for_blob_storage, MessageEnvelope,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

pub(super) fn commit(
    session_id: Uuid,
    record: &RecoveryRecord,
    journal: &mut Option<JournalStore>,
) -> Result<(), String> {
    if record.role == "transcript" {
        use codelet_core::persistence::transcript::JournalRecord;
        let event: JournalRecord = serde_json::from_value(
            record
                .metadata
                .get("_transcript")
                .cloned()
                .ok_or("missing transcript recovery event")?,
        )
        .map_err(|_| "incompatible transcript recovery event")?;
        if event.id != record.id
            || event.identity.session_id != session_id
            || !record.content.is_empty()
        {
            return Err("transcript recovery identity conflict".into());
        }
        if journal.is_none() {
            *journal = Some(
                JournalStore::configured(session_id)
                    .map_err(|_| "transcript persistence recovery required")?,
            );
        }
        return journal
            .as_ref()
            .ok_or("transcript writer unavailable")?
            .append(&event)
            .map_err(|_| "transcript persistence recovery required".into());
    }
    let mut metadata = record.metadata.clone();
    let audit_only = metadata.remove(super::AUDIT_ONLY).is_some();
    // BUG-332: a conversation record must carry its message envelope.
    if !metadata.contains_key("message") {
        return Err("conversation record carries no message envelope".into());
    }
    {
        let mut envelope: MessageEnvelope = serde_json::from_value(json!(metadata))
            .map_err(|_| "incompatible recovery envelope")?;
        if record.role == "assistant" {
            envelope.message = codelet_core::persistence::MessagePayload::Assistant(
                serde_json::from_value(metadata["message"].clone())
                    .map_err(|_| "incompatible assistant recovery envelope")?,
            );
        }
        if envelope.uuid != record.id {
            return Err("recovery envelope identity conflict".into());
        }
        let (processed, references) = process_envelope_for_blob_storage(&envelope)
            .map_err(|_| "persistence blob recovery required")?;
        let fields: HashMap<String, Value> = serde_json::from_value(json!(processed))
            .map_err(|_| "recovery envelope encoding failed")?;
        metadata.extend(fields);
        if !references.is_empty() {
            metadata.insert(
                "_blobRefs".into(),
                references
                    .into_iter()
                    .map(|(key, hash)| json!({"key":key,"hash":hash}))
                    .collect(),
            );
        }
    }
    if audit_only {
        store_original_idempotent(record.id, &record.role, &record.content, metadata)
            .map_err(|_| "persistence archive recovery required")?;
        return Ok(());
    }
    append_message_idempotent(
        session_id,
        record.id,
        &record.role,
        &record.content,
        metadata,
    )
    .map_err(|_| "persistence archive recovery required")?;
    Ok(())
}
