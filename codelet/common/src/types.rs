//! Shared types used across codelet crates
//!
//! These types represent the core data structures for LLM conversations
//! and are used by both providers and the agent execution layer.

use serde::{Deserialize, Serialize};

/// Message role in conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System message
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// Message content types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content
    Text(String),
    /// Structured content with multiple parts
    Parts(Vec<ContentPart>),
}

/// Content part for structured messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    /// Text content
    #[serde(rename = "text")]
    Text { text: String },
    /// Tool call request
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Tool call result
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
    /// Image content (request-side only)
    ///
    /// Mirrors the Anthropic-shaped request body: `{"type": "image",
    /// "source": {...}}`. The [`ImageSource`] payload lets callers choose
    /// between a remote URL and an inline base64-encoded blob with an
    /// explicit `media_type` — Rhai custom providers receive this shape
    /// verbatim via `messages_to_rhai`, which allows each provider's
    /// `build_request` script to reshape it into the native API format.
    ///
    /// Feature: spec/features/multimodal-image-content-in-providers.feature
    #[serde(rename = "image")]
    Image { source: ImageSource },
}

/// Source of a [`ContentPart::Image`] payload.
///
/// Serialises with an internal `type` tag to match the provider-facing JSON
/// wire shape (`{"type": "url", "url": ...}` or
/// `{"type": "base64", "media_type": ..., "data": ...}`).
///
/// Feature: spec/features/multimodal-image-content-in-providers.feature
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ImageSource {
    /// Remote URL reference to an image.
    Url {
        /// Absolute URL where the image can be fetched.
        url: String,
    },
    /// Inline base64-encoded image bytes plus an IANA media type.
    Base64 {
        /// IANA media type (e.g. `image/png`, `image/jpeg`).
        media_type: String,
        /// Base64-encoded image payload.
        data: String,
    },
}

/// Conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message role
    pub role: MessageRole,
    /// Message content
    pub content: MessageContent,
}

impl Message {
    /// Create a user message with text content
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text(text.into()),
        }
    }

    /// Create an assistant message with text content
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text(text.into()),
        }
    }

    /// Create a system message with text content
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: MessageContent::Text(text.into()),
        }
    }
}
