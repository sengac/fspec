#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/tool-result-image-rhai-bridge.feature
//!
//! BUG-141 integration tests. Two surfaces are exercised:
//!
//! 1. `messages_to_rhai` must serialise `ContentPart::ToolResult.parts`
//!    verbatim so a Rhai script can inspect image entries with the
//!    Anthropic-shaped `{type, source}` wire format.
//!
//! 2. The updated `claude_rhai.rhai` `build_request` logic must walk
//!    structured `parts` and emit Anthropic `tool_result` blocks whose
//!    inner `content` array carries text/image entries verbatim. The
//!    test embeds the relevant `build_request` body inline (mirroring
//!    the production script at `~/.fspec/providers/claude_rhai.rhai`)
//!    so the regression is locked in independently of any user-private
//!    file on disk.

#[path = "custom_http_test_helpers.rs"]
mod helpers;

use std::sync::Arc;

use codelet_common::{ContentPart, ImageSource, Message, MessageContent, MessageRole, ToolResultPart};
use codelet_providers::custom::request_bridge::messages_to_rhai;
use codelet_providers::custom::rig_message_convert::rig_messages_to_internal;
use codelet_providers::custom::{RhaiCustomProvider, ScriptLoader};
use rhai::{Array, Dynamic, Map};
use rig::completion::message::{
    DocumentSourceKind, Image as RigImage, ImageMediaType, ToolResultContent, UserContent,
};
use rig::completion::Message as RigMessage;
use rig::OneOrMany;

use helpers::config_with_script;

// =========================================================================
// Helpers
// =========================================================================

/// Build a fresh `RhaiCustomProvider` from an inline script.
fn build_provider(script: &str) -> RhaiCustomProvider {
    let (_tmp, cfg) = config_with_script("img-bridge", script);
    let loader = Arc::new(ScriptLoader::with_default_engine());
    // Leak the tempdir so the script file outlives the provider — each
    // test only constructs one provider, so this is bounded.
    Box::leak(Box::new(_tmp));
    RhaiCustomProvider::new(Arc::new(cfg), loader, "smart".to_string())
        .expect("construct RhaiCustomProvider")
}

/// Re-extract a Rhai `Map` from a `Dynamic`, panicking with a useful
/// message if the value is not a map.
fn as_map(value: &Dynamic) -> Map {
    value
        .clone()
        .try_cast::<Map>()
        .expect("Dynamic was expected to be a Rhai Map")
}

/// Extract a string field from a Rhai map.
fn map_string(map: &Map, key: &str) -> String {
    map.get(key)
        .cloned()
        .unwrap_or(Dynamic::UNIT)
        .into_string()
        .unwrap_or_else(|_| panic!("expected field '{key}' to be a string in map"))
}

/// The `build_request` body shipped in the production
/// `~/.fspec/providers/claude_rhai.rhai` after BUG-141. Embedded here so
/// the test exercises the same logic. Other lifecycle functions are
/// minimal stubs since these tests only invoke `build_request`.
const CLAUDE_RHAI_BUILD_REQUEST_SCRIPT: &str = r#"
fn anthropic_image_block(part) {
    #{ type: "image", source: part.source }
}

fn anthropic_text_block(part) {
    #{ type: "text", text: part.text }
}

fn anthropic_tool_result_inner(part) {
    if type_of(part.parts) == "array" && part.parts.len() > 0 {
        let inner = [];
        for entry in part.parts {
            if entry.type == "image" {
                inner.push(anthropic_image_block(entry));
            } else if entry.type == "text" {
                inner.push(anthropic_text_block(entry));
            }
        }
        return inner;
    }
    // Fallback for legacy/text-only payloads where `parts` is missing
    // or empty: keep `content` as the plain string we always emitted.
    part.content
}

fn convert_user_part(part) {
    if part.type == "text" {
        return anthropic_text_block(part);
    }
    if part.type == "image" {
        return anthropic_image_block(part);
    }
    if part.type == "tool_use" {
        return #{ type: "tool_use", id: part.id, name: part.name, input: part.input };
    }
    if part.type == "tool_result" {
        let is_error = false;
        if type_of(part.is_error) == "bool" {
            is_error = part.is_error;
        }
        return #{
            type: "tool_result",
            tool_use_id: part.tool_use_id,
            is_error: is_error,
            content: anthropic_tool_result_inner(part)
        };
    }
    part
}

fn convert_user_content(raw) {
    if type_of(raw) == "string" {
        return raw;
    }
    if type_of(raw) == "array" {
        let out = [];
        for part in raw {
            out.push(convert_user_part(part));
        }
        return out;
    }
    raw
}

fn build_request(request) {
    let conversation = [];
    for msg in request.messages {
        if msg.role == "user" {
            conversation.push(#{ role: "user", content: convert_user_content(msg.content) });
        } else {
            conversation.push(#{ role: msg.role, content: msg.content });
        }
    }
    #{ messages: conversation }
}

fn build_headers(config) { #{} }
fn build_url(config) { "" }
fn parse_response(raw) { #{ content: "", stop_reason: "end_turn" } }
fn parse_stream_chunk(chunk) { #{} }
fn build_stream_request(ctx) { #{} }
fn map_error(status, body) { #{ type: "api", message: "" } }
"#;

// =========================================================================
// Scenario: messages_to_rhai serialises tool_result image parts verbatim into Rhai
// =========================================================================
#[test]
fn messages_to_rhai_serialises_tool_result_image_parts_verbatim() {
    // @step Given an internal Vec<Message> with one User message whose content is Parts containing ContentPart::ToolResult with tool_use_id "tu_x" and parts [Image base64 "CCC" media_type "image/png"]
    let image_part = ToolResultPart::Image {
        source: ImageSource::Base64 {
            media_type: "image/png".to_string(),
            data: "CCC".to_string(),
        },
    };
    let messages = vec![Message {
        role: MessageRole::User,
        content: MessageContent::Parts(vec![ContentPart::tool_result_parts(
            "tu_x",
            vec![image_part],
            false,
        )]),
    }];

    // @step When I serialise it via messages_to_rhai
    let dyn_value: Dynamic = messages_to_rhai(&messages).expect("messages_to_rhai");

    // @step And I round-trip the resulting Rhai Dynamic back into JSON
    let array: Array = dyn_value
        .into_typed_array::<Dynamic>()
        .expect("messages_to_rhai produced a Rhai Array");
    assert_eq!(array.len(), 1);
    let msg_map = as_map(&array[0]);

    let content_dyn = msg_map
        .get("content")
        .cloned()
        .expect("message map has 'content' field");
    let content_array: Array = content_dyn
        .into_typed_array::<Dynamic>()
        .expect("content is an array for Parts");
    assert_eq!(content_array.len(), 1);

    let tool_result = as_map(&content_array[0]);

    // @step Then the JSON path messages[0].content[0].type equals "tool_result"
    assert_eq!(map_string(&tool_result, "type"), "tool_result");

    // @step And the JSON path messages[0].content[0].tool_use_id equals "tu_x"
    assert_eq!(map_string(&tool_result, "tool_use_id"), "tu_x");

    // @step And the JSON path messages[0].content[0].parts[0] equals {"type":"image","source":{"type":"base64","media_type":"image/png","data":"CCC"}}
    let parts_dyn = tool_result
        .get("parts")
        .cloned()
        .expect("tool_result map has 'parts' field");
    let parts_array: Array = parts_dyn
        .into_typed_array::<Dynamic>()
        .expect("parts is an array");
    assert_eq!(parts_array.len(), 1);

    let image_entry = as_map(&parts_array[0]);
    assert_eq!(map_string(&image_entry, "type"), "image");
    let source = image_entry
        .get("source")
        .cloned()
        .expect("image entry has 'source' field");
    let source_map = as_map(&source);
    assert_eq!(map_string(&source_map, "type"), "base64");
    assert_eq!(map_string(&source_map, "media_type"), "image/png");
    assert_eq!(map_string(&source_map, "data"), "CCC");
}

// =========================================================================
// Helper: invoke the embedded claude_rhai build_request body and return
// the JSON value of `body.messages[0].content[0]` for the first user
// message. This is the surface every BUG-141 build_request scenario
// asserts on.
// =========================================================================
async fn build_first_user_content(messages: &[Message]) -> serde_json::Value {
    let provider = build_provider(CLAUDE_RHAI_BUILD_REQUEST_SCRIPT);
    let body = provider
        .invoke_build_request(messages, &[], None)
        .await
        .expect("build_request returns JSON");
    body.get("messages")
        .and_then(|m| m.as_array())
        .expect("body.messages array")
        .first()
        .cloned()
        .expect("at least one message")
}

// =========================================================================
// Scenario: claude_rhai build_request emits Anthropic tool_result block with embedded image for image-only parts
// =========================================================================
#[tokio::test]
async fn claude_rhai_build_request_emits_image_tool_result_block() {
    // @step Given the updated claude_rhai.rhai script loaded into a Rhai engine
    // (loaded inline via build_first_user_content)

    // @step And a request map whose messages contain one user message with a single tool_result part where tool_use_id is "tu_x" and parts is [Image base64 "CCC" media_type "image/png"]
    let messages = vec![Message {
        role: MessageRole::User,
        content: MessageContent::Parts(vec![ContentPart::tool_result_parts(
            "tu_x",
            vec![ToolResultPart::Image {
                source: ImageSource::Base64 {
                    media_type: "image/png".to_string(),
                    data: "CCC".to_string(),
                },
            }],
            false,
        )]),
    }];

    // @step When I invoke build_request with that request
    let first = build_first_user_content(&messages).await;

    // @step Then the returned body.messages has length 1
    // @step And body.messages[0].role equals "user"
    assert_eq!(first.get("role").and_then(|v| v.as_str()), Some("user"));

    // @step And body.messages[0].content is an array with one entry
    let content = first
        .get("content")
        .and_then(|v| v.as_array())
        .expect("content is array");
    assert_eq!(content.len(), 1);

    // @step And body.messages[0].content[0] equals {"type":"tool_result","tool_use_id":"tu_x","is_error":false,"content":[{"type":"image","source":{"type":"base64","media_type":"image/png","data":"CCC"}}]}
    let expected = serde_json::json!({
        "type": "tool_result",
        "tool_use_id": "tu_x",
        "is_error": false,
        "content": [{
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": "image/png",
                "data": "CCC"
            }
        }]
    });
    assert_eq!(content[0], expected, "tool_result block mismatch");
}

// =========================================================================
// Scenario: claude_rhai build_request emits mixed text and image blocks inside tool_result content
// =========================================================================
#[tokio::test]
async fn claude_rhai_build_request_emits_mixed_text_and_image_blocks() {
    // @step Given the updated claude_rhai.rhai script loaded into a Rhai engine
    // @step And a request map whose messages contain one user message with a single tool_result part where tool_use_id is "tu_y" and parts is [Text "done", Image base64 "DDD" media_type "image/jpeg"]
    let messages = vec![Message {
        role: MessageRole::User,
        content: MessageContent::Parts(vec![ContentPart::tool_result_parts(
            "tu_y",
            vec![
                ToolResultPart::Text {
                    text: "done".to_string(),
                },
                ToolResultPart::Image {
                    source: ImageSource::Base64 {
                        media_type: "image/jpeg".to_string(),
                        data: "DDD".to_string(),
                    },
                },
            ],
            false,
        )]),
    }];

    // @step When I invoke build_request with that request
    let first = build_first_user_content(&messages).await;

    let content = first
        .get("content")
        .and_then(|v| v.as_array())
        .expect("content array");
    assert_eq!(content.len(), 1);
    let block = &content[0];

    // @step Then body.messages[0].content[0].type equals "tool_result"
    assert_eq!(block.get("type").and_then(|v| v.as_str()), Some("tool_result"));

    // @step And body.messages[0].content[0].tool_use_id equals "tu_y"
    assert_eq!(
        block.get("tool_use_id").and_then(|v| v.as_str()),
        Some("tu_y")
    );

    // @step And body.messages[0].content[0].content equals [{"type":"text","text":"done"},{"type":"image","source":{"type":"base64","media_type":"image/jpeg","data":"DDD"}}] in that order
    let inner = block
        .get("content")
        .and_then(|v| v.as_array())
        .expect("inner content array");
    let expected_inner = serde_json::json!([
        {"type": "text", "text": "done"},
        {
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": "image/jpeg",
                "data": "DDD"
            }
        }
    ]);
    let inner_value = serde_json::Value::Array(inner.clone());
    assert_eq!(inner_value, expected_inner, "inner content mismatch");
}

// =========================================================================
// Scenario: claude_rhai build_request leaves plain-text user messages unchanged
// =========================================================================
#[tokio::test]
async fn claude_rhai_build_request_passes_through_plain_text_user_messages() {
    // @step Given the updated claude_rhai.rhai script loaded into a Rhai engine
    // @step And a request map whose messages contain one user message whose content is the plain string "hello world"
    let messages = vec![Message::user("hello world")];

    // @step When I invoke build_request with that request
    let first = build_first_user_content(&messages).await;

    // @step Then body.messages[0].role equals "user"
    assert_eq!(first.get("role").and_then(|v| v.as_str()), Some("user"));

    // @step And body.messages[0].content equals "hello world"
    assert_eq!(
        first.get("content").and_then(|v| v.as_str()),
        Some("hello world")
    );
}

// =========================================================================
// Conversion-side scenarios — exercise rig_messages_to_internal directly so
// the BUG-141 fix in convert_user_message is locked in independently of the
// Rhai script. Each scenario maps 1:1 to a Gherkin scenario in the feature
// file.
// =========================================================================

/// Pull the single `ContentPart::ToolResult` out of a converted user
/// message, panicking with a descriptive message if the conversion
/// produced anything else.
fn extract_tool_result(message: &Message) -> &ContentPart {
    match &message.content {
        MessageContent::Parts(parts) => {
            assert_eq!(
                parts.len(),
                1,
                "expected exactly one ContentPart, got {parts:?}"
            );
            let part = &parts[0];
            assert!(
                matches!(part, ContentPart::ToolResult { .. }),
                "expected ContentPart::ToolResult, got {part:?}"
            );
            part
        }
        other => panic!("expected MessageContent::Parts, got {other:?}"),
    }
}

// =========================================================================
// Scenario: Convert tool_result with single base64 image to structured ToolResultPart::Image
// =========================================================================
#[test]
fn convert_tool_result_with_single_base64_image_becomes_structured_part() {
    // @step Given a rig user message whose content is a single ToolResult with id "call_img" and one ToolResultContent::Image carrying a base64 payload "AAA" with media_type PNG
    let msg = RigMessage::User {
        content: OneOrMany::one(UserContent::tool_result(
            "call_img",
            OneOrMany::one(ToolResultContent::image_base64(
                "AAA",
                Some(ImageMediaType::PNG),
                None,
            )),
        )),
    };

    // @step When I convert the rig history slice via rig_messages_to_internal with no preamble
    let out = rig_messages_to_internal(None, &[msg]);

    // @step Then the resulting Vec<Message> has exactly one User message
    assert_eq!(out.len(), 1);
    assert!(matches!(out[0].role, MessageRole::User));

    // @step And that message's MessageContent is Parts containing a single ContentPart::ToolResult
    let part = extract_tool_result(&out[0]);

    match part {
        ContentPart::ToolResult { tool_use_id, parts, .. } => {
            // @step And that ContentPart::ToolResult has tool_use_id "call_img"
            assert_eq!(tool_use_id, "call_img");

            // @step And that ContentPart::ToolResult parts vector equals [ToolResultPart::Image with ImageSource::Base64 media_type "image/png" data "AAA"]
            assert_eq!(
                parts,
                &vec![ToolResultPart::Image {
                    source: ImageSource::Base64 {
                        media_type: "image/png".to_string(),
                        data: "AAA".to_string(),
                    },
                }]
            );
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }
}

// =========================================================================
// Scenario: Convert tool_result mixing text and base64 image preserves order and derives legacy content
// =========================================================================
#[test]
fn convert_tool_result_mixing_text_and_image_preserves_order() {
    // @step Given a rig user message whose ToolResult content is in order Text "analysis complete" followed by Image base64 "BBB" media_type JPEG
    let mut content = OneOrMany::one(ToolResultContent::text("analysis complete"));
    content.push(ToolResultContent::image_base64(
        "BBB",
        Some(ImageMediaType::JPEG),
        None,
    ));
    let msg = RigMessage::User {
        content: OneOrMany::one(UserContent::tool_result("call_mix", content)),
    };

    // @step When I convert the rig history slice via rig_messages_to_internal with no preamble
    let out = rig_messages_to_internal(None, &[msg]);

    // @step Then the resulting User message contains a single ContentPart::ToolResult
    let part = extract_tool_result(&out[0]);

    match part {
        ContentPart::ToolResult { parts, content, .. } => {
            // @step And that ContentPart::ToolResult parts vector is [ToolResultPart::Text "analysis complete", ToolResultPart::Image with ImageSource::Base64 media_type "image/jpeg" data "BBB"] in that order
            assert_eq!(
                parts,
                &vec![
                    ToolResultPart::Text {
                        text: "analysis complete".to_string()
                    },
                    ToolResultPart::Image {
                        source: ImageSource::Base64 {
                            media_type: "image/jpeg".to_string(),
                            data: "BBB".to_string(),
                        }
                    },
                ]
            );

            // @step And that ContentPart::ToolResult legacy content string equals "analysis complete\n[image]"
            assert_eq!(content, "analysis complete\n[image]");
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }
}

// =========================================================================
// Scenario: Convert text-only tool_result keeps tool_result_text shape for backcompat
// =========================================================================
#[test]
fn convert_text_only_tool_result_uses_tool_result_text_helper() {
    // @step Given a rig user message whose ToolResult content is a single Text "only text"
    let msg = RigMessage::User {
        content: OneOrMany::one(UserContent::tool_result(
            "call_txt",
            OneOrMany::one(ToolResultContent::text("only text")),
        )),
    };

    // @step When I convert the rig history slice via rig_messages_to_internal with no preamble
    let out = rig_messages_to_internal(None, &[msg]);

    let part = extract_tool_result(&out[0]);
    match part {
        ContentPart::ToolResult { content, parts, .. } => {
            // @step Then the resulting User message's ContentPart::ToolResult legacy content equals "only text"
            assert_eq!(content, "only text");

            // @step And the parts vector equals [ToolResultPart::Text "only text"]
            assert_eq!(
                parts,
                &vec![ToolResultPart::Text {
                    text: "only text".to_string()
                }]
            );
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }
}

// =========================================================================
// Scenario: Convert tool_result with URL image preserves the URL source
// =========================================================================
#[test]
fn convert_tool_result_with_url_image_preserves_url_source() {
    // @step Given a rig user message whose ToolResult content is a single Image whose DocumentSourceKind is Url "https://x/y.png" with media_type PNG
    let msg = RigMessage::User {
        content: OneOrMany::one(UserContent::tool_result(
            "call_url",
            OneOrMany::one(ToolResultContent::image_url(
                "https://x/y.png",
                Some(ImageMediaType::PNG),
                None,
            )),
        )),
    };

    // @step When I convert the rig history slice via rig_messages_to_internal with no preamble
    let out = rig_messages_to_internal(None, &[msg]);

    let part = extract_tool_result(&out[0]);
    match part {
        ContentPart::ToolResult { parts, .. } => {
            // @step Then the resulting User message's ContentPart::ToolResult parts vector equals [ToolResultPart::Image with ImageSource::Url "https://x/y.png"]
            assert_eq!(
                parts,
                &vec![ToolResultPart::Image {
                    source: ImageSource::Url {
                        url: "https://x/y.png".to_string()
                    },
                }]
            );
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }
}

// =========================================================================
// Scenario: Convert tool_result with unsupported image variant skips the image but keeps siblings
// =========================================================================
#[test]
fn convert_tool_result_with_unsupported_image_variant_skips_image_only() {
    // @step Given a rig user message whose ToolResult content is in order Text "context" followed by Image whose DocumentSourceKind is Unknown
    let mut content = OneOrMany::one(ToolResultContent::text("context"));
    let unknown_image = ToolResultContent::Image(RigImage {
        data: DocumentSourceKind::Unknown,
        media_type: None,
        detail: None,
        additional_params: None,
    });
    content.push(unknown_image);
    let msg = RigMessage::User {
        content: OneOrMany::one(UserContent::tool_result("call_skip", content)),
    };

    // @step When I convert the rig history slice via rig_messages_to_internal with no preamble
    let out = rig_messages_to_internal(None, &[msg]);

    let part = extract_tool_result(&out[0]);
    match part {
        ContentPart::ToolResult { parts, content, .. } => {
            // @step Then the resulting User message's ContentPart::ToolResult parts vector equals [ToolResultPart::Text "context"]
            assert_eq!(
                parts,
                &vec![ToolResultPart::Text {
                    text: "context".to_string()
                }]
            );

            // @step And the legacy content string equals "context"
            assert_eq!(content, "context");
        }
        other => panic!("expected ToolResult, got {other:?}"),
    }
}
