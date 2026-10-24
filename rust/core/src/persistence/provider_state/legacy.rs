use super::{
    super::{get_message, SessionManifest, StoredMessage},
    types::*,
};
use serde_json::{json, Value};

pub(crate) fn is_checkpoint(stored: &StoredMessage) -> bool {
    stored.role == "assistant"
        && stored.content.is_empty()
        && stored.metadata.get("provider").and_then(Value::as_str) == Some("codex")
        && stored.metadata.contains_key("codexContinuation")
        && stored.metadata.keys().all(|key| {
            matches!(
                key.as_str(),
                "provider" | "message" | "codexContinuation" | "_codexReset"
            )
        })
        && stored.metadata.get("message").is_some_and(|message| {
            message.as_object().is_some_and(|fields| fields.len() == 2)
                && message["role"] == "assistant"
                && message["content"].as_array().is_some_and(Vec::is_empty)
        })
}

fn metadata(stored: &StoredMessage) -> Metadata {
    canonical_metadata(
        stored
            .metadata
            .iter()
            .filter(|(key, _)| key.as_str() != "message")
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    )
}

pub(crate) fn marker(stored: &StoredMessage) -> Result<Option<Value>, ProviderStateError> {
    if !is_checkpoint(stored) {
        return Ok(None);
    }
    let metadata = metadata(stored);
    let (provider, generation) = native_identity(&metadata)?;
    Ok(Some(
        json!({"recordKind":"provider_state","version":1,"provider":provider,
        "generation":generation,"stateId":metadata_hash(&metadata)?}),
    ))
}

pub(super) fn collect(manifest: &SessionManifest) -> Result<State, ProviderStateError> {
    let mut state = State::empty(manifest.id);
    for (index, reference) in manifest.messages.iter().enumerate() {
        let stored = get_message(reference.message_id)
            .map_err(ProviderStateError::Storage)?
            .ok_or_else(|| invalid("missing source record"))?;
        if is_checkpoint(&stored) {
            state.upsert(manifest, metadata(&stored), index + 1)?;
        }
    }
    Ok(state)
}
