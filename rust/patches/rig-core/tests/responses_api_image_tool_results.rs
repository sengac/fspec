//! Feature: spec/features/codex-view-image.feature
//!
//! Tests for OpenAI Responses API handling of image data in tool results.
//! Validates that ToolResultContent::Image is properly converted to structured
//! content items (InputImage) matching the Codex CLI's FunctionCallOutputContentItem format.
//!
//! Since ToolResult/InputItem fields are private, we test via JSON serialization
//! which validates the wire format sent to the API.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use rig::completion::message::{Image, Text, ToolResult, ToolResultContent};
use rig::message::{DocumentSourceKind, ImageMediaType, Message};
use rig::providers::openai::responses_api::{
    InputItem, ToolResultContentItem, ToolResultOutput,
};
use rig::OneOrMany;

// =============================================================================
// Helper: convert a rig completion::Message into Vec<InputItem>
// and serialize to JSON for wire-format validation
// =============================================================================

fn convert_to_json(msg: Message) -> Vec<serde_json::Value> {
    let items: Vec<InputItem> =
        <Vec<InputItem>>::try_from(msg).expect("conversion should succeed");
    items
        .iter()
        .map(|item| serde_json::to_value(item).expect("serialization should succeed"))
        .collect()
}

// =============================================================================
// Tests for TryFrom<crate::completion::Message> for Vec<InputItem>
// (the path that was panicking with "This thing only supports text!")
// =============================================================================

/// Scenario: Text-only tool result produces plain string output
#[test]
fn test_text_only_tool_result_produces_string_output() {
    // @step Given a tool result with only text content
    let msg = Message::User {
        content: OneOrMany::one(rig::message::UserContent::ToolResult(ToolResult {
            id: "tool-1".to_string(),
            call_id: Some("call-1".to_string()),
            content: OneOrMany::one(ToolResultContent::Text(Text {
                text: "Hello, world!".to_string(),
            })),
        })),
    };

    // @step When converting to InputItem and serializing
    let items = convert_to_json(msg);

    // @step Then the output should be a plain text string
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(item["type"], "function_call_output");
    assert_eq!(item["call_id"], "call-1");
    assert!(
        item["output"].is_string(),
        "Text-only output should be a plain string, got: {}",
        item["output"]
    );
    assert_eq!(item["output"], "Hello, world!");
}

/// Scenario: Image tool result produces structured content items with InputImage
/// (this is the exact scenario that was panicking before the fix)
#[test]
fn test_image_tool_result_produces_content_items() {
    // @step Given a tool result with image content (base64 PNG)
    let msg = Message::User {
        content: OneOrMany::one(rig::message::UserContent::ToolResult(ToolResult {
            id: "tool-1".to_string(),
            call_id: Some("call-1".to_string()),
            content: OneOrMany::one(ToolResultContent::Image(Image {
                data: DocumentSourceKind::Base64("iVBORw0KGgoAAAANSUhEUg==".to_string()),
                media_type: Some(ImageMediaType::PNG),
                detail: None,
                additional_params: None,
            })),
        })),
    };

    // @step When converting to InputItem and serializing
    let items = convert_to_json(msg);

    // @step Then the output should be structured content items with an InputImage
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(item["type"], "function_call_output");
    assert_eq!(item["call_id"], "call-1");
    assert_eq!(item["status"], "completed");

    let output = &item["output"];
    assert!(
        output.is_array(),
        "Image output should be array of content items, got: {output}"
    );
    let arr = output.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["type"], "input_image");
    assert!(
        arr[0]["image_url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,"),
        "Expected data URI, got: {}",
        arr[0]["image_url"]
    );
    assert!(arr[0]["image_url"]
        .as_str()
        .unwrap()
        .contains("iVBORw0KGgoAAAANSUhEUg=="));
}

/// Scenario: JPEG image tool result produces correct media type in data URI
#[test]
fn test_jpeg_image_tool_result_has_correct_media_type() {
    // @step Given a tool result with JPEG image content
    let msg = Message::User {
        content: OneOrMany::one(rig::message::UserContent::ToolResult(ToolResult {
            id: "tool-1".to_string(),
            call_id: Some("call-1".to_string()),
            content: OneOrMany::one(ToolResultContent::Image(Image {
                data: DocumentSourceKind::Base64("/9j/4AAQSkZJRg==".to_string()),
                media_type: Some(ImageMediaType::JPEG),
                detail: None,
                additional_params: None,
            })),
        })),
    };

    // @step When converting to InputItem and serializing
    let items = convert_to_json(msg);

    // @step Then the data URI should use image/jpeg media type
    let output = &items[0]["output"];
    let arr = output.as_array().unwrap();
    assert!(
        arr[0]["image_url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/jpeg;base64,"),
        "Expected JPEG data URI, got: {}",
        arr[0]["image_url"]
    );
}

/// Scenario: Mixed text and image tool result produces structured content items
#[test]
fn test_mixed_text_and_image_tool_result() {
    // @step Given a tool result with both text and image content
    let msg = Message::User {
        content: OneOrMany::one(rig::message::UserContent::ToolResult(ToolResult {
            id: "tool-1".to_string(),
            call_id: Some("call-1".to_string()),
            content: OneOrMany::many(vec![
                ToolResultContent::Text(Text {
                    text: "Image description:".to_string(),
                }),
                ToolResultContent::Image(Image {
                    data: DocumentSourceKind::Base64("iVBORw0KGgoAAAANSUhEUg==".to_string()),
                    media_type: Some(ImageMediaType::PNG),
                    detail: None,
                    additional_params: None,
                }),
            ])
            .unwrap(),
        })),
    };

    // @step When converting to InputItem and serializing
    let items = convert_to_json(msg);

    // @step Then the output should be structured content items with both text and image
    assert_eq!(items.len(), 1);
    let output = &items[0]["output"];
    assert!(
        output.is_array(),
        "Mixed output should be array of content items, got: {output}"
    );
    let arr = output.as_array().unwrap();
    assert_eq!(arr.len(), 2, "Should have text + image items");

    assert_eq!(arr[0]["type"], "input_text");
    assert_eq!(arr[0]["text"], "Image description:");

    assert_eq!(arr[1]["type"], "input_image");
    assert!(arr[1]["image_url"]
        .as_str()
        .unwrap()
        .starts_with("data:image/png;base64,"));
}

// =============================================================================
// Tests for ToolResultOutput serialization
// =============================================================================

/// Scenario: Text output serializes as a JSON string
#[test]
fn test_tool_result_output_text_serializes_as_string() {
    let output = ToolResultOutput::Text("hello".to_string());
    let json = serde_json::to_value(&output).unwrap();
    assert!(
        json.is_string(),
        "Text output should serialize as string, got: {json}"
    );
    assert_eq!(json.as_str().unwrap(), "hello");
}

/// Scenario: ContentItems output serializes as a JSON array
#[test]
fn test_tool_result_output_content_items_serializes_as_array() {
    let output = ToolResultOutput::ContentItems(vec![
        ToolResultContentItem::InputText {
            text: "description".to_string(),
        },
        ToolResultContentItem::InputImage {
            image_url: "data:image/png;base64,abc123".to_string(),
            detail: None,
        },
    ]);
    let json = serde_json::to_value(&output).unwrap();
    assert!(
        json.is_array(),
        "ContentItems should serialize as array, got: {json}"
    );
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["type"], "input_text");
    assert_eq!(arr[0]["text"], "description");
    assert_eq!(arr[1]["type"], "input_image");
    assert_eq!(arr[1]["image_url"], "data:image/png;base64,abc123");
}

/// Scenario: ToolResultOutput round-trips through serde for text
#[test]
fn test_tool_result_output_text_roundtrip() {
    let original = ToolResultOutput::Text("test content".to_string());
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ToolResultOutput = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

/// Scenario: ToolResultOutput round-trips through serde for content items
#[test]
fn test_tool_result_output_content_items_roundtrip() {
    let original = ToolResultOutput::ContentItems(vec![
        ToolResultContentItem::InputText {
            text: "hello".to_string(),
        },
        ToolResultContentItem::InputImage {
            image_url: "data:image/png;base64,abc".to_string(),
            detail: None,
        },
    ]);
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ToolResultOutput = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

// =============================================================================
// Test for the TryFrom<Message> -> Vec<Message> path
// (the second conversion path in responses_api)
// =============================================================================

/// Scenario: Image tool result converts via Vec<Message> path
#[test]
fn test_image_tool_result_via_message_conversion() {
    // @step Given a tool result with image content
    let msg = Message::User {
        content: OneOrMany::one(rig::message::UserContent::ToolResult(ToolResult {
            id: "tool-1".to_string(),
            call_id: Some("call-1".to_string()),
            content: OneOrMany::one(ToolResultContent::Image(Image {
                data: DocumentSourceKind::Base64("iVBORw0KGgoAAAANSUhEUg==".to_string()),
                media_type: Some(ImageMediaType::PNG),
                detail: None,
                additional_params: None,
            })),
        })),
    };

    // @step When converting through the Vec<Message> path and serializing
    let messages: Vec<rig::providers::openai::responses_api::Message> =
        msg.try_into().expect("conversion should succeed");

    assert_eq!(messages.len(), 1);
    let json = serde_json::to_value(&messages[0]).unwrap();

    // @step Then the message output should have structured content items
    let output = &json["output"];
    assert!(
        output.is_array(),
        "Image tool result output should be array of content items, got: {output}"
    );
    let arr = output.as_array().unwrap();
    assert_eq!(arr[0]["type"], "input_image");
}

/// Scenario: Text tool result converts via Vec<Message> path with string output
#[test]
fn test_text_tool_result_via_message_conversion() {
    let msg = Message::User {
        content: OneOrMany::one(rig::message::UserContent::ToolResult(ToolResult {
            id: "tool-1".to_string(),
            call_id: Some("call-1".to_string()),
            content: OneOrMany::one(ToolResultContent::Text(Text {
                text: "plain text result".to_string(),
            })),
        })),
    };

    let messages: Vec<rig::providers::openai::responses_api::Message> =
        msg.try_into().expect("conversion should succeed");

    assert_eq!(messages.len(), 1);
    let json = serde_json::to_value(&messages[0]).unwrap();
    assert!(
        json["output"].is_string(),
        "Text tool result output should be a string, got: {}",
        json["output"]
    );
    assert_eq!(json["output"], "plain text result");
}

// =============================================================================
// Test: the .unwrap() on line 614 is gone (no panic on image tool results)
// =============================================================================

/// Scenario: CompletionRequest construction does not panic with image tool results
/// This is a regression test for the `.unwrap()` that caused the panic in the screenshot
#[test]
fn test_completion_request_from_image_tool_result_does_not_panic() {
    use rig::completion::{CompletionRequest as CReq, message as cmsg};

    // Build a CompletionRequest that includes an image tool result in the chat history.
    // Before the fix this would panic at the .unwrap() in TryFrom.
    let tool_result_msg = Message::User {
        content: OneOrMany::one(rig::message::UserContent::ToolResult(cmsg::ToolResult {
            id: "tool-1".to_string(),
            call_id: Some("call-1".to_string()),
            content: OneOrMany::one(cmsg::ToolResultContent::Image(Image {
                data: DocumentSourceKind::Base64("iVBORw0KGgoAAAANSUhEUg==".to_string()),
                media_type: Some(ImageMediaType::PNG),
                detail: None,
                additional_params: None,
            })),
        })),
    };

    let req = CReq {
        preamble: Some("You are a helpful assistant.".to_string()),
        chat_history: OneOrMany::one(tool_result_msg),
        documents: vec![],
        tools: vec![],
        temperature: None,
        max_tokens: None,
        tool_choice: None,
        additional_params: None,
    };

    // This is the exact code path from TryFrom<(String, CompletionRequest)> for CompletionRequest
    // that had the .unwrap() panic. It should now succeed.
    let result = rig::providers::openai::responses_api::CompletionRequest::try_from((
        "gpt-5.3-codex".to_string(),
        req,
    ));

    assert!(
        result.is_ok(),
        "CompletionRequest construction should not fail for image tool results: {:?}",
        result.err()
    );
}
