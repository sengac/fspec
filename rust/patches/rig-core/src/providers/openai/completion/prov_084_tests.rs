// ================================================================
// PROV-084 TESTS — tool-returned images (Read/PDF/MCP)
//
// Feature: spec/features/openai-provider-tool-result-images.feature
//
// Covers PROV-084 — a rig `ToolResult` containing image parts must
// convert into an OpenAI `tool` message (carrying text or a
// placeholder) followed by a `user` message whose content array
// carries every `image_url` part. Text-only tool results must
// continue to convert into a single `tool` message.
// ================================================================
//! Feature: spec/features/openai-provider-tool-result-images.feature
use super::*;
use crate::OneOrMany;
use crate::completion::message::{
    Message as RigMessage, ToolResult as RigToolResult, ToolResultContent as RigToolResultContent,
    UserContent as RigUserContent,
};
use crate::message::ImageMediaType;

fn png_image_part() -> RigToolResultContent {
    RigToolResultContent::image_base64("iVBORw0KGgoAAAANSUhEUg", Some(ImageMediaType::PNG), None)
}

fn jpeg_image_part() -> RigToolResultContent {
    RigToolResultContent::image_base64(
        "/9j/4AAQSkZJRgABAQEAYABgAAD",
        Some(ImageMediaType::JPEG),
        None,
    )
}

fn text_part(text: &str) -> RigToolResultContent {
    RigToolResultContent::text(text)
}

/// Wrap a `ToolResult` into a rig `Message::User` and convert via the
/// production `TryFrom<message::Message> for Vec<Message>` impl.
fn convert_tool_result(tool_result: RigToolResult) -> Vec<Message> {
    let rig_msg = RigMessage::User {
        content: OneOrMany::one(RigUserContent::ToolResult(tool_result)),
    };
    <Vec<Message> as TryFrom<RigMessage>>::try_from(rig_msg)
        .expect("tool-result → openai Vec<Message> conversion should succeed")
}

#[test]
fn prov_084_single_base64_image_emits_tool_then_user() {
    // @step Given a rig user message whose content is a `ToolResult` with id "call_abc", one `ToolResultContent::Image` (base64, media_type=image/png), and no text parts
    let tr = RigToolResult {
        id: "call_abc".into(),
        call_id: None,
        content: OneOrMany::one(png_image_part()),
    };

    // @step When the provider converts the message via `<Vec<openai::Message> as TryFrom<rig::message::Message>>::try_from`
    let result = convert_tool_result(tr);

    // @step Then the resulting Vec<Message> has exactly 2 elements
    assert_eq!(result.len(), 2, "expected 2 messages, got {}", result.len());

    // @step And the first element is a `tool` role message with tool_call_id "call_abc"
    match &result[0] {
        Message::ToolResult {
            tool_call_id,
            content,
        } => {
            assert_eq!(tool_call_id, "call_abc");
            // @step And the first element's content is a non-empty placeholder string
            let text = content.as_text();
            assert!(
                !text.is_empty(),
                "tool content placeholder must be non-empty"
            );
        }
        other => panic!("expected Message::ToolResult, got {other:?}"),
    }

    // @step And the second element is a `user` role message
    let user_json = match &result[1] {
        user @ Message::User { content, .. } => {
            assert!(
                !content.is_empty(),
                "user message must carry at least one content part"
            );
            serde_json::to_value(user).expect("serialize user message")
        }
        other => panic!("expected Message::User, got {other:?}"),
    };

    // @step And the second element contains exactly one `image_url` content part
    let content_array = user_json["content"]
        .as_array()
        .expect("user content should serialize as an array");
    assert_eq!(content_array.len(), 1, "expected exactly one content part");
    assert_eq!(content_array[0]["type"], "image_url");

    // @step And that image_url.url starts with "data:image/png;base64,"
    let url = content_array[0]["image_url"]["url"]
        .as_str()
        .expect("image_url.url must be a string");
    assert!(
        url.starts_with("data:image/png;base64,"),
        "unexpected url prefix: {url}"
    );

    // @step And that image_url.detail equals "auto"
    assert_eq!(content_array[0]["image_url"]["detail"], "auto");
}

#[test]
fn prov_084_text_only_tool_result_still_single_message() {
    // @step Given a rig user message whose content is a `ToolResult` with id "call_text" and one `ToolResultContent::Text("hello")`
    let tr = RigToolResult {
        id: "call_text".into(),
        call_id: None,
        content: OneOrMany::one(text_part("hello")),
    };

    // @step When the provider converts the message via `<Vec<openai::Message> as TryFrom<rig::message::Message>>::try_from`
    let result = convert_tool_result(tr);

    // @step Then the resulting Vec<Message> has exactly 1 element
    assert_eq!(result.len(), 1, "expected 1 message, got {}", result.len());

    // @step And the element is a `tool` role message with tool_call_id "call_text"
    // @step And the element's content equals "hello"
    match &result[0] {
        Message::ToolResult {
            tool_call_id,
            content,
        } => {
            assert_eq!(tool_call_id, "call_text");
            assert_eq!(content.as_text(), "hello");
        }
        other => panic!("expected Message::ToolResult, got {other:?}"),
    }
}

#[test]
fn prov_084_three_images_yield_tool_plus_user_with_three_parts() {
    // @step Given a rig user message whose content is a `ToolResult` with id "call_pdf" and three `ToolResultContent::Image` parts in page order (page-1, page-2, page-3)
    let page1 = RigToolResultContent::image_base64("UEFHRS0xAAAA", Some(ImageMediaType::PNG), None);
    let page2 = RigToolResultContent::image_base64("UEFHRS0yAAAA", Some(ImageMediaType::PNG), None);
    let page3 = RigToolResultContent::image_base64("UEFHRS0zAAAA", Some(ImageMediaType::PNG), None);
    let content =
        OneOrMany::many(vec![page1, page2, page3]).expect("three-part tool result content");
    let tr = RigToolResult {
        id: "call_pdf".into(),
        call_id: None,
        content,
    };

    // @step When the provider converts the message via `<Vec<openai::Message> as TryFrom<rig::message::Message>>::try_from`
    let result = convert_tool_result(tr);

    // @step Then the resulting Vec<Message> has exactly 2 elements
    assert_eq!(result.len(), 2);

    // @step And the first element is a `tool` role message with tool_call_id "call_pdf"
    match &result[0] {
        Message::ToolResult { tool_call_id, .. } => {
            assert_eq!(tool_call_id, "call_pdf");
        }
        other => panic!("expected Message::ToolResult, got {other:?}"),
    }

    // @step And the second element is a `user` role message
    let user_json = match &result[1] {
        user @ Message::User { .. } => serde_json::to_value(user).expect("serialize user"),
        other => panic!("expected Message::User, got {other:?}"),
    };

    // @step And the second element contains exactly three `image_url` content parts in page order
    let parts = user_json["content"].as_array().expect("user content array");
    assert_eq!(parts.len(), 3, "expected three image parts");
    for part in parts {
        assert_eq!(part["type"], "image_url");
    }
    // Spot-check page ordering using the distinct base64 payloads.
    let url0 = parts[0]["image_url"]["url"].as_str().expect("url0");
    let url1 = parts[1]["image_url"]["url"].as_str().expect("url1");
    let url2 = parts[2]["image_url"]["url"].as_str().expect("url2");
    assert!(
        url0.contains("UEFHRS0xAAAA"),
        "page-1 expected first: {url0}"
    );
    assert!(
        url1.contains("UEFHRS0yAAAA"),
        "page-2 expected second: {url1}"
    );
    assert!(
        url2.contains("UEFHRS0zAAAA"),
        "page-3 expected third: {url2}"
    );
}

#[test]
fn prov_084_mixed_text_and_image_splits_correctly() {
    // @step Given a rig user message whose content is a `ToolResult` with id "call_mcp", one `ToolResultContent::Text("summary text")`, and one `ToolResultContent::Image` (base64, media_type=image/jpeg)
    let content = OneOrMany::many(vec![text_part("summary text"), jpeg_image_part()])
        .expect("mixed tool result content");
    let tr = RigToolResult {
        id: "call_mcp".into(),
        call_id: None,
        content,
    };

    // @step When the provider converts the message via `<Vec<openai::Message> as TryFrom<rig::message::Message>>::try_from`
    let result = convert_tool_result(tr);

    // @step Then the resulting Vec<Message> has exactly 2 elements
    assert_eq!(result.len(), 2);

    // @step And the first element is a `tool` role message with tool_call_id "call_mcp"
    // @step And the first element's content contains the substring "summary text"
    match &result[0] {
        Message::ToolResult {
            tool_call_id,
            content,
        } => {
            assert_eq!(tool_call_id, "call_mcp");
            let text = content.as_text();
            assert!(
                text.contains("summary text"),
                "tool-message content must carry the text part, got: {text}"
            );
        }
        other => panic!("expected Message::ToolResult, got {other:?}"),
    }

    // @step And the second element is a `user` role message
    let user_json = match &result[1] {
        user @ Message::User { .. } => serde_json::to_value(user).expect("serialize user"),
        other => panic!("expected Message::User, got {other:?}"),
    };

    // @step And the second element contains exactly one `image_url` content part
    let parts = user_json["content"].as_array().expect("user content array");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["type"], "image_url");

    // @step And that image_url.url starts with "data:image/jpeg;base64,"
    let url = parts[0]["image_url"]["url"].as_str().expect("url string");
    assert!(
        url.starts_with("data:image/jpeg;base64,"),
        "unexpected url prefix: {url}"
    );
}
