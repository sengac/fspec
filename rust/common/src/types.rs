//! Shared types used across codelet crates
//!
//! These types represent the core data structures for LLM conversations
//! and are used by both providers and the agent execution layer.

use serde::{Deserialize, Deserializer, Serialize};

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

/// Content part for structured messages.
///
/// Serde note: `ContentPart::ToolResult` is deserialised via
/// [`ContentPartWire`] so that legacy JSON payloads without a `parts`
/// field continue to round-trip (they deserialise into a single
/// [`ToolResultPart::Text`] derived from the `content` string). See
/// [`tool_result_structured_content_tests`] and BUG-140 for details.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
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
    /// Tool call result.
    ///
    /// Carries both the legacy `content` string (for backward-compatible
    /// consumers that read tool output as plain text) and a structured
    /// `parts` vector that can represent mixed text + image content.
    /// For pure-text tool results, `content` and the single
    /// [`ToolResultPart::Text`] part carry the same string. For
    /// image-bearing tool results (e.g. `Read` of a PNG), `parts`
    /// includes [`ToolResultPart::Image`] entries alongside any text
    /// summaries, and `content` contains a best-effort textual
    /// representation for consumers that cannot render images.
    ///
    /// Feature: spec/features/tool-result-structured-content-parts.feature
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
        /// Structured content parts. Always non-empty for values
        /// constructed via [`ContentPart::tool_result_text`] or
        /// [`ContentPart::tool_result_parts`]. When deserialising
        /// legacy JSON that lacks this field, a single
        /// [`ToolResultPart::Text`] is synthesised from `content` to
        /// preserve the invariant.
        parts: Vec<ToolResultPart>,
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

impl<'de> Deserialize<'de> for ContentPart {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ContentPartWire::deserialize(deserializer)?;
        Ok(ContentPart::from(wire))
    }
}

impl From<ContentPartWire> for ContentPart {
    fn from(wire: ContentPartWire) -> Self {
        match wire {
            ContentPartWire::Text { text } => ContentPart::Text { text },
            ContentPartWire::ToolUse { id, name, input } => {
                ContentPart::ToolUse { id, name, input }
            }
            ContentPartWire::ToolResult {
                tool_use_id,
                content,
                is_error,
                mut parts,
            } => {
                // Legacy JSON payloads (and any future writer that
                // omits `parts`) must still produce a non-empty parts
                // vector — synthesise a single Text part from
                // `content` so downstream consumers can walk `parts`
                // uniformly.
                if parts.is_empty() {
                    parts.push(ToolResultPart::Text {
                        text: content.clone(),
                    });
                }
                ContentPart::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                    parts,
                }
            }
            ContentPartWire::Image { source } => ContentPart::Image { source },
        }
    }
}

/// Wire-format shadow of [`ContentPart`] used only for deserialisation.
///
/// Mirrors the public enum exactly except that the `parts` field on
/// the `ToolResult` variant defaults to an empty `Vec` when absent —
/// the conversion in [`ContentPart::from`] re-establishes the
/// "at least one part" invariant by synthesising a [`ToolResultPart::Text`]
/// from the `content` string when needed.
#[derive(Deserialize)]
#[serde(tag = "type")]
enum ContentPartWire {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        #[serde(default)]
        content: String,
        #[serde(default)]
        is_error: bool,
        #[serde(default)]
        parts: Vec<ToolResultPart>,
    },
    #[serde(rename = "image")]
    Image { source: ImageSource },
}

impl ContentPart {
    /// Construct a pure-text [`ContentPart::ToolResult`].
    ///
    /// This is the primary entry point for call sites that previously
    /// wrote `ContentPart::ToolResult { tool_use_id, content, is_error }`.
    /// It keeps `content` and the single [`ToolResultPart::Text`] in
    /// `parts` synchronised so both text-only consumers (reading
    /// `content`) and structured-part consumers (reading `parts`) see
    /// the same payload.
    pub fn tool_result_text(
        tool_use_id: impl Into<String>,
        text: impl Into<String>,
        is_error: bool,
    ) -> Self {
        let text = text.into();
        ContentPart::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: text.clone(),
            is_error,
            parts: vec![ToolResultPart::Text { text }],
        }
    }

    /// Construct a structured [`ContentPart::ToolResult`] from a
    /// non-empty list of parts.
    ///
    /// The `content` string is derived from the text parts for
    /// backward compatibility; image parts are rendered as a bracketed
    /// placeholder so plain-text consumers still see something
    /// meaningful. Downstream providers that support images should read
    /// `parts` directly rather than `content`.
    ///
    /// # Panics
    /// Panics in debug builds if `parts` is empty — the invariant that
    /// a ToolResult always exposes at least one part is enforced at
    /// construction time to match the post-deserialisation contract.
    pub fn tool_result_parts(
        tool_use_id: impl Into<String>,
        parts: Vec<ToolResultPart>,
        is_error: bool,
    ) -> Self {
        debug_assert!(
            !parts.is_empty(),
            "tool_result_parts requires at least one part"
        );
        let content = derive_content_from_parts(&parts);
        ContentPart::ToolResult {
            tool_use_id: tool_use_id.into(),
            content,
            is_error,
            parts,
        }
    }
}

/// Render a text preview of a structured parts list for legacy
/// consumers that only read the `content` string. Text parts are
/// joined with newlines in order; image parts contribute a short
/// bracketed placeholder so the preview is never silently empty.
fn derive_content_from_parts(parts: &[ToolResultPart]) -> String {
    parts
        .iter()
        .map(|p| match p {
            ToolResultPart::Text { text } => text.clone(),
            ToolResultPart::Image { .. } => "[image]".to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Structured content part inside a [`ContentPart::ToolResult`] `parts`
/// list.
///
/// Mirrors the Anthropic tool_result content-block shape, where each
/// entry is either a text block (`{"type":"text","text":"..."}`) or an
/// image block (`{"type":"image","source":{...}}`). Scripts that build
/// provider-native request bodies can forward this structure verbatim.
///
/// Feature: spec/features/tool-result-structured-content-parts.feature
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ToolResultPart {
    /// Plain text content.
    Text { text: String },
    /// Image content (URL or base64). Reuses [`ImageSource`] so the
    /// request-side image shape and tool_result image shape stay
    /// identical.
    Image { source: ImageSource },
}

/// Source of a [`ContentPart::Image`] payload.
///
/// Serialises with an internal `type` tag to match the provider-facing JSON
/// wire shape (`{"type": "url", "url": ...}` or
/// `{"type": "base64", "media_type": ..., "data": ...}`).
///
/// Feature: spec/features/multimodal-image-content-in-providers.feature
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tool_result_structured_content_tests {
    //! Feature: spec/features/tool-result-structured-content-parts.feature
    //!
    //! These tests validate that `ContentPart::ToolResult` carries structured
    //! (text + image) content via a `parts` vector while preserving the legacy
    //! `content` string for backward compatibility with scripts that read
    //! `content` directly.

    use super::*;

    /// Scenario: Serialise text-only ToolResult preserves legacy content field
    #[test]
    fn scenario_text_only_tool_result_preserves_legacy_content_field() {
        // @step Given I build a ContentPart::ToolResult with tool_use_id "tu_1", content "file contents", a single Text part "file contents", and is_error false
        let part = ContentPart::ToolResult {
            tool_use_id: "tu_1".to_string(),
            content: "file contents".to_string(),
            is_error: false,
            parts: vec![ToolResultPart::Text {
                text: "file contents".to_string(),
            }],
        };

        // @step When I serialize the content part to JSON
        let value = serde_json::to_value(&part).expect("serialize ToolResult");

        // @step Then the JSON type field is "tool_result"
        assert_eq!(
            value.get("type").and_then(|v| v.as_str()),
            Some("tool_result")
        );

        // @step Then the JSON content field equals "file contents"
        assert_eq!(
            value.get("content").and_then(|v| v.as_str()),
            Some("file contents")
        );

        // @step Then the JSON tool_use_id field equals "tu_1"
        assert_eq!(
            value.get("tool_use_id").and_then(|v| v.as_str()),
            Some("tu_1")
        );

        // @step Then the JSON is_error field equals false
        assert_eq!(
            value.get("is_error").and_then(serde_json::Value::as_bool),
            Some(false)
        );
    }

    /// Scenario: Serialise text-only ToolResult exposes a single text part
    #[test]
    fn scenario_text_only_tool_result_exposes_single_text_part() {
        // @step Given I build a ContentPart::ToolResult via the text helper with content "hello"
        let part = ContentPart::tool_result_text("tu_any", "hello", false);

        // @step When I serialize the content part to JSON
        let value = serde_json::to_value(&part).expect("serialize ToolResult");

        // @step Then the JSON parts array has exactly one entry
        let parts = value
            .get("parts")
            .and_then(|v| v.as_array())
            .expect("parts array present");
        assert_eq!(parts.len(), 1);

        // @step Then that entry's type field is "text"
        let entry = &parts[0];
        assert_eq!(entry.get("type").and_then(|v| v.as_str()), Some("text"));

        // @step Then that entry's text field equals "hello"
        assert_eq!(entry.get("text").and_then(|v| v.as_str()), Some("hello"));
    }

    /// Scenario: Serialise ToolResult with a base64 image part
    #[test]
    fn scenario_tool_result_with_base64_image_part_serialises_source() {
        // @step Given I build a ContentPart::ToolResult via the parts helper with a single ToolResultPart::Image whose source is ImageSource::Base64 media_type "image/png" and data "AAA"
        let image_part = ToolResultPart::Image {
            source: ImageSource::Base64 {
                media_type: "image/png".to_string(),
                data: "AAA".to_string(),
            },
        };
        let part = ContentPart::tool_result_parts("tu_img", vec![image_part], false);

        // @step When I serialize the content part to JSON
        let value = serde_json::to_value(&part).expect("serialize ToolResult");

        // @step Then the JSON parts array has one entry whose type field is "image"
        let parts = value
            .get("parts")
            .and_then(|v| v.as_array())
            .expect("parts present");
        assert_eq!(parts.len(), 1);
        let entry = &parts[0];
        assert_eq!(entry.get("type").and_then(|v| v.as_str()), Some("image"));

        // @step Then that entry's source.type field is "base64"
        let source = entry.get("source").expect("source present");
        assert_eq!(source.get("type").and_then(|v| v.as_str()), Some("base64"));

        // @step Then that entry's source.media_type field equals "image/png"
        assert_eq!(
            source.get("media_type").and_then(|v| v.as_str()),
            Some("image/png")
        );

        // @step Then that entry's source.data field equals "AAA"
        assert_eq!(source.get("data").and_then(|v| v.as_str()), Some("AAA"));
    }

    /// Scenario: Round-trip mixed text and image parts through JSON
    #[test]
    fn scenario_round_trip_mixed_text_and_image_parts() {
        // @step Given I build a ContentPart::ToolResult whose parts are Text "summary" followed by Image with Base64 source media_type "image/jpeg" and data "BBB"
        let original_parts = vec![
            ToolResultPart::Text {
                text: "summary".to_string(),
            },
            ToolResultPart::Image {
                source: ImageSource::Base64 {
                    media_type: "image/jpeg".to_string(),
                    data: "BBB".to_string(),
                },
            },
        ];
        let original = ContentPart::tool_result_parts("tu_mixed", original_parts.clone(), false);

        // @step When I serialize it to JSON and deserialize the JSON back into a ContentPart
        let json = serde_json::to_string(&original).expect("serialize");
        let round_tripped: ContentPart = serde_json::from_str(&json).expect("deserialize");

        // @step Then the deserialized value's parts equal the original parts in order
        match &round_tripped {
            ContentPart::ToolResult { parts, .. } => {
                assert_eq!(parts, &original_parts);
            }
            other => panic!("expected ToolResult, got {other:?}"),
        }

        // @step Then the deserialized value's tool_use_id, is_error, and content fields equal the original
        match (&original, &round_tripped) {
            (
                ContentPart::ToolResult {
                    tool_use_id: o_id,
                    content: o_content,
                    is_error: o_err,
                    ..
                },
                ContentPart::ToolResult {
                    tool_use_id: r_id,
                    content: r_content,
                    is_error: r_err,
                    ..
                },
            ) => {
                assert_eq!(o_id, r_id);
                assert_eq!(o_content, r_content);
                assert_eq!(o_err, r_err);
            }
            _ => panic!("expected ToolResult on both sides"),
        }
    }

    /// Scenario: Deserialize legacy JSON without parts field yields a single text part
    #[test]
    fn scenario_legacy_json_without_parts_yields_single_text_part() {
        // @step Given I have legacy tool_result JSON {"type":"tool_result","tool_use_id":"tu_x","content":"old output","is_error":false} with no parts field
        let legacy_json = r#"{"type":"tool_result","tool_use_id":"tu_x","content":"old output","is_error":false}"#;

        // @step When I deserialize the JSON into a ContentPart
        let part: ContentPart = serde_json::from_str(legacy_json).expect("legacy deserialize");

        // @step Then the deserialized ContentPart::ToolResult has content equal to "old output"
        match &part {
            ContentPart::ToolResult { content, parts, .. } => {
                assert_eq!(content, "old output");

                // @step Then the deserialized ContentPart::ToolResult has a parts vector containing exactly one ToolResultPart::Text whose text equals "old output"
                assert_eq!(parts.len(), 1);
                match &parts[0] {
                    ToolResultPart::Text { text } => {
                        assert_eq!(text, "old output");
                    }
                    other => panic!("expected Text part, got {other:?}"),
                }
            }
            other => panic!("expected ToolResult, got {other:?}"),
        }
    }

    /// Scenario: ToolResult with URL image part serialises the source verbatim
    #[test]
    fn scenario_tool_result_with_url_image_part_serialises_source_verbatim() {
        // @step Given I build a ContentPart::ToolResult via the parts helper with a single ToolResultPart::Image whose source is ImageSource::Url "https://example.com/a.png"
        let image_part = ToolResultPart::Image {
            source: ImageSource::Url {
                url: "https://example.com/a.png".to_string(),
            },
        };
        let part = ContentPart::tool_result_parts("tu_url", vec![image_part], false);

        // @step When I serialize the content part to JSON
        let value = serde_json::to_value(&part).expect("serialize ToolResult");

        // @step Then the JSON parts array's single entry has type "image"
        let parts = value
            .get("parts")
            .and_then(|v| v.as_array())
            .expect("parts present");
        assert_eq!(parts.len(), 1);
        let entry = &parts[0];
        assert_eq!(entry.get("type").and_then(|v| v.as_str()), Some("image"));

        // @step Then that entry's source.type field equals "url"
        let source = entry.get("source").expect("source present");
        assert_eq!(source.get("type").and_then(|v| v.as_str()), Some("url"));

        // @step Then that entry's source.url field equals "https://example.com/a.png"
        assert_eq!(
            source.get("url").and_then(|v| v.as_str()),
            Some("https://example.com/a.png")
        );
    }
}
