//! Claude Code Message Envelope Types
//!
//! This module defines the exact message format used by Claude Code for session persistence.
//! The format matches Claude Code's JSONL files in ~/.claude/projects/{project}/{session}.jsonl
//!
//! Message Envelope Schema (outer wrapper per message):
//! ```json
//! {
//!   "uuid": "a6bdbefb-902d-4f98-b539-8cbee91ec831",
//!   "parentUuid": "81dc2799-ef52-4923-aa24-5798585aae57",
//!   "timestamp": "2025-12-23T08:51:44.813Z",
//!   "type": "assistant",
//!   "provider": "claude",
//!   "message": { ... },
//!   "requestId": "req_011CWPLKJZcigRWVriKduWSr"
//! }
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Note: BLOB_THRESHOLD constant is defined in blob.rs

/// Message envelope wrapping every message in a session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MessageEnvelope {
    /// Unique identifier for this message
    pub uuid: Uuid,
    /// Parent message UUID for threading
    pub parent_uuid: Option<Uuid>,
    /// When the message was created
    pub timestamp: DateTime<Utc>,
    /// Message type: "user" or "assistant"
    #[serde(rename = "type")]
    pub message_type: String,
    /// Provider: "claude", "openai", "gemini", "codex"
    pub provider: String,
    /// The actual message payload
    pub message: MessagePayload,
    /// Optional API request ID for debugging
    pub request_id: Option<String>,
}

/// The message payload (either user or assistant).
///
/// Serialized untagged (the inner message carries its own `role`). It is
/// deserialized by that `role` (BUG-322): an untagged derive tried `User`
/// first, which accepts any role and drops unknown fields, so text-only
/// assistant messages reloaded as user-shaped messages without their model.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(untagged)]
pub enum MessagePayload {
    /// User message (role: "user")
    User(UserMessage),
    /// Assistant message (role: "assistant")
    Assistant(AssistantMessage),
}

impl<'de> Deserialize<'de> for MessagePayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let assistant = value.get("role").and_then(serde_json::Value::as_str) == Some("assistant");
        if assistant {
            // An assistant-role payload the assistant shape cannot parse
            // still loads the way the untagged derive did: as user-shaped.
            match serde_json::from_value::<AssistantMessage>(value.clone()) {
                Ok(message) => Ok(MessagePayload::Assistant(message)),
                Err(_) => serde_json::from_value(value)
                    .map(MessagePayload::User)
                    .map_err(serde::de::Error::custom),
            }
        } else {
            serde_json::from_value(value)
                .map(MessagePayload::User)
                .map_err(serde::de::Error::custom)
        }
    }
}

/// User message format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserMessage {
    /// Always "user" - used by serde untagged to distinguish from AssistantMessage
    #[serde(default = "default_user_role")]
    pub role: String,
    /// Array of content blocks
    pub content: Vec<UserContent>,
}

fn default_user_role() -> String {
    "user".to_string()
}

/// Content types in user messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserContent {
    /// Plain text content
    Text { text: String },
    /// Tool execution result
    ToolResult {
        tool_use_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        call_id: Option<String>,
        content: String,
        #[serde(default)]
        is_error: bool,
        /// Extended metadata for tool results
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_use_result: Option<ToolUseResultMetadata>,
    },
    /// Image content (base64 or URL)
    Image { source: ImageSource },
    /// Document content (base64 or URL)
    Document {
        source: DocumentSource,
        /// Optional title for the document
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Optional context/description for the document
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
        /// Cache control settings
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
}

/// Assistant message format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantMessage {
    /// Always "assistant" - used by serde untagged to distinguish from UserMessage
    #[serde(default = "default_assistant_role")]
    pub role: String,
    /// Message ID (e.g., "msg_01Wk7SCqoakQaEmx7FHphZRJ")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Model used (e.g., "claude-opus-4-5-20251101")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Array of content blocks
    pub content: Vec<AssistantContent>,
    /// Stop reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Token usage for this message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsagePerMessage>,
}

fn default_assistant_role() -> String {
    "assistant".to_string()
}

/// Content types in assistant messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantContent {
    /// Plain text content
    Text { text: String },
    /// Tool use request
    ToolUse {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        call_id: Option<String>,
        name: String,
        input: serde_json::Value,
    },
    /// Thinking/reasoning content
    Thinking {
        thinking: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
}

/// Token usage per message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenUsagePerMessage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u64>,
}

/// Extended metadata for tool results
/// Fields are optional to handle both raw output and formatted results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolUseResultMetadata {
    /// Raw stdout from command execution
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    /// Raw stderr from command execution
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    /// Whether execution was interrupted (e.g., timeout, Ctrl+C)
    #[serde(default)]
    pub interrupted: bool,
    /// Whether the result contains image data
    #[serde(default)]
    pub is_image: bool,
}

impl ToolUseResultMetadata {
    /// Create metadata with required stdout/stderr fields (for tests)
    pub fn with_output(stdout: impl Into<String>, stderr: impl Into<String>) -> Self {
        Self {
            stdout: Some(stdout.into()),
            stderr: Some(stderr.into()),
            interrupted: false,
            is_image: false,
        }
    }
}

/// Image source (base64 or URL)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    Base64 { media_type: String, data: String },
    Url { url: String },
}

/// Document source (base64 or URL)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocumentSource {
    Base64 { media_type: String, data: String },
    Url { url: String },
}

/// Cache control for documents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CacheControl {
    Ephemeral,
}

// Note: should_use_blob_storage is in blob.rs

#[cfg(test)]
mod tests;

/// BUG-332: the stored envelope fields of a plain-text message, as written for a row
/// that is not produced by the agent loop's typed commit path (the compaction summary
/// row and the `append_message` fixture writers). Replay requires every one of
/// `parentUuid`, `timestamp`, `type`, `provider`, `message` and `requestId`.
pub fn text_envelope_metadata(
    role: &str,
    provider: &str,
    text: &str,
    timestamp: DateTime<Utc>,
) -> std::collections::HashMap<String, serde_json::Value> {
    std::collections::HashMap::from([
        ("parentUuid".to_string(), serde_json::Value::Null),
        ("timestamp".to_string(), serde_json::json!(timestamp)),
        ("type".to_string(), serde_json::json!(role)),
        ("provider".to_string(), serde_json::json!(provider)),
        (
            "message".to_string(),
            serde_json::json!({"role": role, "content": [{"type": "text", "text": text}]}),
        ),
        ("requestId".to_string(), serde_json::Value::Null),
    ])
}
