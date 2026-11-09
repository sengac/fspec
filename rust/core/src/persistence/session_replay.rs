use super::{get_session_messages, load_session};
use serde_json::{json, Value};
use uuid::Uuid;

/// Envelope fields every stored conversation row carries (BUG-332).
const ENVELOPE_KEYS: [&str; 6] = [
    "parentUuid",
    "timestamp",
    "type",
    "provider",
    "message",
    "requestId",
];

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
        // BUG-327/BUG-332: the replayed envelope is the stored envelope, which
        // every conversation row must carry. Replay sets only the row identity
        // and the message role and blob-rehydrated content.
        if let Some(missing) = ENVELOPE_KEYS
            .iter()
            .find(|key| !stored.metadata.contains_key(**key))
        {
            return Err(format!(
                "stored message {} has no message envelope (missing {missing})",
                stored.id
            ));
        }
        let content = stored
            .metadata
            .get("message")
            .and_then(|message| message.get("content"))
            .cloned()
            .ok_or_else(|| format!("stored message {} has no message content", stored.id))?;
        let mut fields: serde_json::Map<String, Value> = stored.metadata.into_iter().collect();
        fields.insert("uuid".into(), json!(stored.id));
        let message = fields
            .get_mut("message")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| format!("stored message {} is not an object", stored.id))?;
        message.insert("role".into(), json!(stored.role));
        message.insert("content".into(), content);
        let mut envelope = Value::Object(ordered(fields));
        super::provider_state::replay::strip_native_metadata(&mut envelope);
        if stored.id == Uuid::nil() {
            envelope["provider"] = json!("compaction");
            envelope["_synthetic"] = json!(true);
            envelope["_compactionSummary"] = json!(true);
            if let Some(commit) = &session.compaction_commit {
                envelope["_committedCompaction"] = json!({
                    "version":commit.version,
                    "generation":commit.identity.generation,
                    "providerGeneration":commit.checkpoint.provider_generation()?,
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

/// Envelope fields first in their canonical order, then the remaining stored
/// keys sorted, so replay output does not depend on map iteration order.
fn ordered(mut fields: serde_json::Map<String, Value>) -> serde_json::Map<String, Value> {
    let mut out = serde_json::Map::with_capacity(fields.len());
    for key in [
        "uuid",
        "parentUuid",
        "timestamp",
        "type",
        "provider",
        "message",
        "requestId",
    ] {
        if let Some(value) = fields.remove(key) {
            out.insert(key.into(), value);
        }
    }
    let mut rest: Vec<_> = fields.into_iter().collect();
    rest.sort_by(|left, right| left.0.cmp(&right.0));
    out.extend(rest);
    out
}
