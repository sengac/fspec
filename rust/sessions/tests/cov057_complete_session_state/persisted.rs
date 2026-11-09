//! COV-057: the production save/restore pair for message envelopes.
//!
//! `save` mirrors the agent loop's envelope commit
//! (`agent-loop/src/persistence_retry_commit.rs`): large content is
//! externalized with `process_envelope_for_blob_storage`, the `_blobRefs`
//! are recorded, and the envelope fields are appended as message metadata
//! under the envelope's own uuid. `restore` is a cold reload: every
//! in-process store is dropped, the manifest and its stored messages are
//! re-read from disk, and blob references are rehydrated.

use super::*;
use codelet_core::persistence::conversation_commit::append_message_idempotent;
use codelet_core::persistence::{
    process_envelope_for_blob_storage, rehydrate_envelope_blobs, UserContent, UserMessage,
};
use serde_json::{json, Value};

pub type Outcome<T> = Result<T, Box<dyn std::error::Error>>;

/// An assistant message with the defaults a provider response carries.
pub fn assistant(content: Vec<AssistantContent>) -> AssistantMessage {
    AssistantMessage {
        role: "assistant".into(),
        id: None,
        model: Some("claude-opus-4-5-20251101".into()),
        content,
        stop_reason: Some("end_turn".into()),
        usage: None,
    }
}

pub fn text(text: &str) -> AssistantContent {
    AssistantContent::Text { text: text.into() }
}

pub fn user_text(text: &str) -> UserContent {
    UserContent::Text { text: text.into() }
}

pub fn tool_result(tool_use_id: &str, content: &str, is_error: bool) -> UserContent {
    UserContent::ToolResult {
        tool_use_id: tool_use_id.into(),
        call_id: None,
        content: content.into(),
        is_error,
        tool_use_result: None,
    }
}

/// Wrap a payload in a fresh envelope whose type follows the payload role.
pub fn wrap(message: MessagePayload) -> MessageEnvelope {
    let message_type = match &message {
        MessagePayload::User(_) => "user",
        MessagePayload::Assistant(_) => "assistant",
    };
    MessageEnvelope {
        uuid: uuid::Uuid::new_v4(),
        parent_uuid: None,
        timestamp: chrono::Utc::now(),
        message_type: message_type.into(),
        provider: "claude".into(),
        message,
        request_id: None,
    }
}

pub fn user(content: Vec<UserContent>) -> MessageEnvelope {
    wrap(MessagePayload::User(UserMessage {
        role: "user".into(),
        content,
    }))
}

pub fn reply(message: AssistantMessage) -> MessageEnvelope {
    wrap(MessagePayload::Assistant(message))
}

/// The searchable text preview stored beside the envelope.
fn preview(envelope: &MessageEnvelope) -> String {
    let parts: Vec<&str> = match &envelope.message {
        MessagePayload::User(message) => message
            .content
            .iter()
            .filter_map(|part| match part {
                UserContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect(),
        MessagePayload::Assistant(message) => message
            .content
            .iter()
            .filter_map(|part| match part {
                AssistantContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect(),
    };
    parts.join("\n")
}

/// Persist envelopes through the production envelope commit steps.
pub fn save(session: uuid::Uuid, envelopes: &[MessageEnvelope]) -> Outcome<()> {
    for envelope in envelopes {
        let (processed, references) = process_envelope_for_blob_storage(envelope)?;
        let mut metadata: HashMap<String, Value> =
            serde_json::from_value(serde_json::to_value(&processed)?)?;
        if !references.is_empty() {
            let references: Value = references
                .into_iter()
                .map(|(key, hash)| json!({"key": key, "hash": hash}))
                .collect();
            metadata.insert("_blobRefs".into(), references);
        }
        let role = envelope.message_type.as_str();
        append_message_idempotent(session, envelope.uuid, role, &preview(envelope), metadata)?;
    }
    Ok(())
}

/// Cold reload of the stored envelopes exactly as they sit on disk.
pub fn stored(session: uuid::Uuid) -> Outcome<Vec<Value>> {
    reset_stores_for_tests();
    get_session_messages(&load_session(session)?)?
        .into_iter()
        .map(|stored| Ok(serde_json::to_value(stored.metadata)?))
        .collect()
}

/// Cold reload with blob references rehydrated from blob storage.
pub fn restore(session: uuid::Uuid) -> Outcome<Vec<MessageEnvelope>> {
    stored(session)?
        .into_iter()
        .map(|value| {
            Ok(serde_json::from_str(&rehydrate_envelope_blobs(
                &value.to_string(),
            )?)?)
        })
        .collect()
}

pub fn assistant_of(envelope: &MessageEnvelope) -> Outcome<&AssistantMessage> {
    match &envelope.message {
        MessagePayload::Assistant(message) => Ok(message),
        MessagePayload::User(_) => Err(format!("expected assistant: {envelope:?}").into()),
    }
}

pub fn user_of(envelope: &MessageEnvelope) -> Outcome<&[UserContent]> {
    match &envelope.message {
        MessagePayload::User(message) => Ok(&message.content),
        MessagePayload::Assistant(_) => Err(format!("expected user: {envelope:?}").into()),
    }
}

/// Every blob file currently in the data directory's blob store.
pub fn blob_files() -> Outcome<Vec<std::path::PathBuf>> {
    let root = codelet_common::get_data_dir()?.join("blobs");
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    for shard in std::fs::read_dir(root)? {
        let shard = shard?.path();
        if shard.is_dir() {
            for blob in std::fs::read_dir(shard)? {
                files.push(blob?.path());
            }
        }
    }
    Ok(files)
}

/// A new persisted session in a hermetic data directory.
pub async fn fresh() -> Outcome<(
    support::TestDirectory,
    std::sync::Arc<codelet_sessions::SessionManager>,
    SessionId,
    uuid::Uuid,
)> {
    let (directory, manager, session_id, _) = manager().await?;
    let uuid = uuid::Uuid::parse_str(&session_id.value)?;
    Ok((directory, manager, session_id, uuid))
}
