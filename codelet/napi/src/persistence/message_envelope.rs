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

/// The message payload (either user or assistant)
/// Uses untagged serde because the inner message has a `role` field that distinguishes them
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessagePayload {
    /// User message (role: "user")
    User(UserMessage),
    /// Assistant message (role: "assistant")
    Assistant(AssistantMessage),
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
    Base64 {
        media_type: String,
        data: String,
    },
    Url {
        url: String,
    },
}

/// Document source (base64 or URL)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocumentSource {
    Base64 {
        media_type: String,
        data: String,
    },
    Url {
        url: String,
    },
}

/// Cache control for documents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CacheControl {
    Ephemeral,
}

// Note: should_use_blob_storage is in blob.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_envelope_serialization() {
        let envelope = MessageEnvelope {
            uuid: Uuid::new_v4(),
            parent_uuid: None,
            timestamp: Utc::now(),
            message_type: "user".to_string(),
            provider: "claude".to_string(),
            message: MessagePayload::User(UserMessage {
                role: "user".to_string(),
                content: vec![UserContent::Text {
                    text: "Hello".to_string(),
                }],
            }),
            request_id: None,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let restored: MessageEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(envelope, restored);
    }

    #[test]
    fn test_tool_use_serialization() {
        let content = AssistantContent::ToolUse {
            id: "toolu_123".to_string(),
            name: "read_file".to_string(),
            input: serde_json::json!({"path": "/foo.ts"}),
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"tool_use\""));
        assert!(json.contains("\"id\":\"toolu_123\""));
    }

    #[test]
    fn test_thinking_with_signature() {
        let content = AssistantContent::Thinking {
            thinking: "Let me think...".to_string(),
            signature: Some("sig_abc123".to_string()),
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"thinking\""));
        assert!(json.contains("\"signature\":\"sig_abc123\""));
    }

    #[test]
    fn test_blob_threshold() {
        use crate::persistence::should_use_blob_storage;

        let small = vec![0u8; 100];
        let large = vec![0u8; 20_000];

        assert!(!should_use_blob_storage(&small));
        assert!(should_use_blob_storage(&large));
    }

    // =========================================================================
    // Extended Message Envelope Tests (NAPI-008)
    // =========================================================================

    #[test]
    fn test_message_uuid_preserved_for_identity() {
        let uuid = Uuid::parse_str("a6bdbefb-902d-4f98-b539-8cbee91ec831").unwrap();
        let envelope = MessageEnvelope {
            uuid,
            parent_uuid: None,
            timestamp: Utc::now(),
            message_type: "assistant".to_string(),
            provider: "claude".to_string(),
            message: MessagePayload::Assistant(AssistantMessage {
                role: "assistant".to_string(),
                id: Some("msg_test".to_string()),
                model: Some("claude-opus-4-5-20251101".to_string()),
                content: vec![AssistantContent::Text {
                    text: "Hello".to_string(),
                }],
                stop_reason: Some("end_turn".to_string()),
                usage: None,
            }),
            request_id: None,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let restored: MessageEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(
            restored.uuid,
            Uuid::parse_str("a6bdbefb-902d-4f98-b539-8cbee91ec831").unwrap()
        );
    }

    #[test]
    fn test_parent_uuid_preserved_for_threading() {
        let msg1_uuid = Uuid::new_v4();
        let msg2 = MessageEnvelope {
            uuid: Uuid::new_v4(),
            parent_uuid: Some(msg1_uuid),
            timestamp: Utc::now(),
            message_type: "assistant".to_string(),
            provider: "claude".to_string(),
            message: MessagePayload::Assistant(AssistantMessage {
                role: "assistant".to_string(),
                id: Some("msg_test".to_string()),
                model: None,
                content: vec![AssistantContent::Text {
                    text: "Hi there!".to_string(),
                }],
                stop_reason: Some("end_turn".to_string()),
                usage: None,
            }),
            request_id: None,
        };

        let json = serde_json::to_string(&msg2).unwrap();
        let restored: MessageEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.parent_uuid, Some(msg1_uuid));
    }

    #[test]
    fn test_tool_result_serialization() {
        let content = UserContent::ToolResult {
            tool_use_id: "toolu_01AyTnm7YLfybhnhEhwwZvAY".to_string(),
            content: r#"{"port": 8080}"#.to_string(),
            is_error: false,
            tool_use_result: None,
        };

        let json = serde_json::to_string(&content).unwrap();
        let restored: UserContent = serde_json::from_str(&json).unwrap();

        match restored {
            UserContent::ToolResult {
                tool_use_id,
                content,
                is_error,
                ..
            } => {
                assert_eq!(tool_use_id, "toolu_01AyTnm7YLfybhnhEhwwZvAY");
                assert_eq!(content, r#"{"port": 8080}"#);
                assert!(!is_error);
            }
            _ => panic!("Expected ToolResult content"),
        }
    }

    #[test]
    fn test_tool_error_result_preserved() {
        let content = UserContent::ToolResult {
            tool_use_id: "toolu_xyz789".to_string(),
            content: "Permission denied".to_string(),
            is_error: true,
            tool_use_result: None,
        };

        let json = serde_json::to_string(&content).unwrap();
        let restored: UserContent = serde_json::from_str(&json).unwrap();

        match restored {
            UserContent::ToolResult { is_error, .. } => {
                assert!(is_error);
            }
            _ => panic!("Expected ToolResult content"),
        }
    }

    #[test]
    fn test_tool_result_with_metadata() {
        let content = UserContent::ToolResult {
            tool_use_id: "toolu_abc123".to_string(),
            content: "formatted output".to_string(),
            is_error: false,
            tool_use_result: Some(ToolUseResultMetadata::with_output(
                "raw stdout content",
                "warning: deprecated",
            )),
        };

        let json = serde_json::to_string(&content).unwrap();
        let restored: UserContent = serde_json::from_str(&json).unwrap();

        match restored {
            UserContent::ToolResult {
                tool_use_result, ..
            } => {
                let metadata = tool_use_result.expect("Expected metadata");
                assert_eq!(metadata.stdout, Some("raw stdout content".to_string()));
                assert_eq!(metadata.stderr, Some("warning: deprecated".to_string()));
                assert!(!metadata.interrupted);
            }
            _ => panic!("Expected ToolResult content"),
        }
    }

    #[test]
    fn test_thinking_without_signature() {
        let content = AssistantContent::Thinking {
            thinking: "Let me think about this problem...".to_string(),
            signature: None,
        };

        let json = serde_json::to_string(&content).unwrap();
        let restored: AssistantContent = serde_json::from_str(&json).unwrap();

        match restored {
            AssistantContent::Thinking { thinking, signature } => {
                assert_eq!(thinking, "Let me think about this problem...");
                assert!(signature.is_none());
            }
            _ => panic!("Expected Thinking content"),
        }
    }

    #[test]
    fn test_base64_image_content() {
        let content = UserContent::Image {
            source: ImageSource::Base64 {
                media_type: "image/png".to_string(),
                data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string(),
            },
        };

        let json = serde_json::to_string(&content).unwrap();
        let restored: UserContent = serde_json::from_str(&json).unwrap();

        match restored {
            UserContent::Image { source } => match source {
                ImageSource::Base64 { media_type, data } => {
                    assert_eq!(media_type, "image/png");
                    assert!(data.starts_with("iVBORw0KGgo"));
                }
                _ => panic!("Expected Base64 source"),
            },
            _ => panic!("Expected Image content"),
        }
    }

    #[test]
    fn test_url_image_reference() {
        let content = UserContent::Image {
            source: ImageSource::Url {
                url: "https://example.com/screenshot.jpg".to_string(),
            },
        };

        let json = serde_json::to_string(&content).unwrap();
        let restored: UserContent = serde_json::from_str(&json).unwrap();

        match restored {
            UserContent::Image { source } => match source {
                ImageSource::Url { url } => {
                    assert_eq!(url, "https://example.com/screenshot.jpg");
                }
                _ => panic!("Expected Url source"),
            },
            _ => panic!("Expected Image content"),
        }
    }

    #[test]
    fn test_document_with_metadata() {
        let content = UserContent::Document {
            source: DocumentSource::Base64 {
                media_type: "application/pdf".to_string(),
                data: "JVBERi0xLjQK".to_string(),
            },
            title: Some("document.pdf".to_string()),
            context: Some("Project documentation".to_string()),
            cache_control: None,
        };

        let json = serde_json::to_string(&content).unwrap();
        let restored: UserContent = serde_json::from_str(&json).unwrap();

        match restored {
            UserContent::Document {
                source,
                title,
                context,
                ..
            } => {
                match source {
                    DocumentSource::Base64 { media_type, .. } => {
                        assert_eq!(media_type, "application/pdf");
                    }
                    _ => panic!("Expected Base64 source"),
                }
                assert_eq!(title, Some("document.pdf".to_string()));
                assert_eq!(context, Some("Project documentation".to_string()));
            }
            _ => panic!("Expected Document content"),
        }
    }

    #[test]
    fn test_multi_part_message_preserves_order() {
        let message = AssistantMessage {
            role: "assistant".to_string(),
            id: Some("msg_multipart".to_string()),
            model: Some("claude-opus-4-5-20251101".to_string()),
            content: vec![
                AssistantContent::Text {
                    text: "Let me check...".to_string(),
                },
                AssistantContent::ToolUse {
                    id: "toolu_read".to_string(),
                    name: "read_file".to_string(),
                    input: serde_json::json!({"path": "/foo.ts"}),
                },
                AssistantContent::Text {
                    text: "Found it.".to_string(),
                },
            ],
            stop_reason: Some("end_turn".to_string()),
            usage: None,
        };

        let json = serde_json::to_string(&message).unwrap();
        let restored: AssistantMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.content.len(), 3);
        assert!(matches!(&restored.content[0], AssistantContent::Text { .. }));
        assert!(matches!(&restored.content[1], AssistantContent::ToolUse { .. }));
        assert!(matches!(&restored.content[2], AssistantContent::Text { .. }));
    }

    #[test]
    fn test_token_usage_preserved() {
        let message = AssistantMessage {
            role: "assistant".to_string(),
            id: Some("msg_usage".to_string()),
            model: Some("claude-opus-4-5-20251101".to_string()),
            content: vec![AssistantContent::Text {
                text: "Response with usage".to_string(),
            }],
            stop_reason: Some("end_turn".to_string()),
            usage: Some(TokenUsagePerMessage {
                input_tokens: 500,
                output_tokens: 150,
                cache_read_input_tokens: Some(200),
                cache_creation_input_tokens: Some(100),
            }),
        };

        let json = serde_json::to_string(&message).unwrap();
        let restored: AssistantMessage = serde_json::from_str(&json).unwrap();

        let usage = restored.usage.expect("Expected usage");
        assert_eq!(usage.input_tokens, 500);
        assert_eq!(usage.output_tokens, 150);
        assert_eq!(usage.cache_read_input_tokens, Some(200));
        assert_eq!(usage.cache_creation_input_tokens, Some(100));
    }

    #[test]
    fn test_request_id_preserved() {
        let envelope = MessageEnvelope {
            uuid: Uuid::new_v4(),
            parent_uuid: None,
            timestamp: Utc::now(),
            message_type: "assistant".to_string(),
            provider: "claude".to_string(),
            message: MessagePayload::Assistant(AssistantMessage {
                role: "assistant".to_string(),
                id: None,
                model: None,
                content: vec![AssistantContent::Text {
                    text: "Test".to_string(),
                }],
                stop_reason: None,
                usage: None,
            }),
            request_id: Some("req_011CWPLKJZcigRWVriKduWSr".to_string()),
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let restored: MessageEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(
            restored.request_id,
            Some("req_011CWPLKJZcigRWVriKduWSr".to_string())
        );
    }

    #[test]
    fn test_stop_reason_preserved() {
        let message = AssistantMessage {
            role: "assistant".to_string(),
            id: None,
            model: None,
            content: vec![AssistantContent::Text {
                text: "Complete response".to_string(),
            }],
            stop_reason: Some("end_turn".to_string()),
            usage: None,
        };

        let json = serde_json::to_string(&message).unwrap();
        let restored: AssistantMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.stop_reason, Some("end_turn".to_string()));
    }

    #[test]
    fn test_assistant_message_without_optional_fields() {
        let message = AssistantMessage {
            role: "assistant".to_string(),
            id: None,
            model: None,
            content: vec![AssistantContent::Text {
                text: "Response without optional fields".to_string(),
            }],
            stop_reason: None,
            usage: None,
        };

        let json = serde_json::to_string(&message).unwrap();
        let restored: AssistantMessage = serde_json::from_str(&json).unwrap();

        assert!(restored.id.is_none());
        assert!(restored.model.is_none());
        assert!(restored.stop_reason.is_none());
        assert!(restored.usage.is_none());
    }

    #[test]
    fn test_interrupted_tool_execution_metadata() {
        let content = UserContent::ToolResult {
            tool_use_id: "toolu_abc123".to_string(),
            content: "partial output".to_string(),
            is_error: true,
            tool_use_result: Some(ToolUseResultMetadata {
                stdout: Some("partial output".to_string()),
                stderr: Some("".to_string()),
                interrupted: true,
                is_image: false,
            }),
        };

        let json = serde_json::to_string(&content).unwrap();
        let restored: UserContent = serde_json::from_str(&json).unwrap();

        match restored {
            UserContent::ToolResult {
                tool_use_result,
                is_error,
                ..
            } => {
                let metadata = tool_use_result.expect("Expected metadata");
                assert!(metadata.interrupted);
                assert!(is_error);
            }
            _ => panic!("Expected ToolResult content"),
        }
    }
}
