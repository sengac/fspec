use super::{get_session_messages, load_session};
use serde_json::{json, Value};
use uuid::Uuid;

pub fn get_session_message_envelopes(session_id: Uuid) -> Result<Vec<String>, String> {
    let session = load_session(session_id)?;
    envelopes_for_manifest(&session)
}

pub fn envelopes_for_manifest(session: &super::SessionManifest) -> Result<Vec<String>, String> {
    let session_id = session.id;
    super::compaction_boundary::validate_committed_boundary(session)?;
    super::compaction_boundary::validate_original_references(session)?;
    let identity = json!({"version":1,"sessionId":session_id,"revision":session.revision});
    let provider_metadata =
        super::provider_state::replay::metadata(session).map_err(|error| error.to_string())?;
    let messages = get_session_messages(session)?;
    let mut envelopes = Vec::with_capacity(messages.len());
    for stored in messages {
        if super::provider_state::legacy::is_checkpoint(&stored) {
            continue;
        }
        let content = stored
            .metadata
            .get("message")
            .and_then(|message| message.get("content"))
            .cloned()
            .unwrap_or_else(|| json!([{"type":"text","text":stored.content}]));
        let mut envelope = json!({
            "uuid":stored.id,"parentUuid":null,"timestamp":stored.created_at.to_rfc3339(),
            "type":stored.role,"provider":stored.metadata.get("provider").and_then(Value::as_str).unwrap_or("unknown"),
            "message":{"role":stored.role,"content":content},"requestId":null,
        });
        if let Some(fields) = envelope.as_object_mut() {
            for (key, value) in stored.metadata {
                fields.entry(key).or_insert(value);
            }
        }
        super::provider_state::replay::strip_native_metadata(&mut envelope);
        if stored.id == Uuid::nil() {
            envelope["provider"] = json!("compaction");
            envelope["_synthetic"] = json!(true);
            envelope["_compactionSummary"] = json!(true);
            if let Some(commit) = &session.compaction_commit {
                envelope["_committedCompaction"] = json!({
                    "version":commit.version,
                    "generation":commit.identity.generation,
                    "operationId":commit.identity.operation_id,
                    "revision":commit.identity.revision,
                });
                envelope["_providerCheckpoint"] = commit.checkpoint.payload.clone();
                if let Some(metadata) = commit.checkpoint.payload.as_object() {
                    for (key, value) in metadata {
                        envelope[key] = value.clone();
                    }
                }
            }
        }
        let encoded = serde_json::to_string(&envelope).map_err(|error| error.to_string())?;
        if !envelope["message"]["content"].is_string() {
            let rehydrated = super::blob_processing::rehydrate_envelope_blobs(&encoded)?;
            envelope = serde_json::from_str(&rehydrated).map_err(|error| error.to_string())?;
        }
        envelope["_sessionReplay"] = identity.clone();
        envelopes.push(serde_json::to_string(&envelope).map_err(|error| error.to_string())?);
    }
    for mut metadata in provider_metadata {
        metadata["_sessionReplay"] = identity.clone();
        envelopes.push(serde_json::to_string(&metadata).map_err(|error| error.to_string())?);
    }
    if envelopes.is_empty() {
        envelopes.push(json!({"_sessionReplay":identity}).to_string());
    }
    Ok(envelopes)
}
