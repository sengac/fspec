#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/multimodal-image-content-in-providers.feature
//!
//! This test file validates PROV-091: image/multimodal content flowing through
//! the core message model and the Rhai request bridge. It covers:
//!   - adapter::extract_text_from_content skipping Image parts
//!   - custom::request_bridge::messages_to_rhai serializing Image parts verbatim
//!     for both URL and base64 sources
//!   - custom::response_bridge::rhai_to_completion_response still rejecting
//!     {type:"image", ...} entries in parse_response output (request-only bridge)
//!
//! Scenarios map directly to Gherkin scenarios; @step comments are attached
//! to each step.

use codelet_common::{ContentPart, ImageSource, Message, MessageContent, MessageRole};
use codelet_providers::custom::request_bridge::messages_to_rhai;
use codelet_providers::custom::response_bridge::rhai_to_completion_response;
use codelet_providers::{convert_assistant_content, extract_text_from_content, ProviderError};
use rhai::{Array, Dynamic, Map};

// =========================================================================
// Scenario: extract_text_from_content skips Image parts
// =========================================================================
#[test]
fn extract_text_from_content_skips_image_parts() {
    // @step Given a MessageContent::Parts containing a Text "hi", an Image with a URL source, and a Text "bye"
    let content = MessageContent::Parts(vec![
        ContentPart::Text {
            text: "hi".to_string(),
        },
        ContentPart::Image {
            source: ImageSource::Url {
                url: "https://example.com/a.png".to_string(),
            },
        },
        ContentPart::Text {
            text: "bye".to_string(),
        },
    ]);

    // @step When I call extract_text_from_content on the content
    let text = extract_text_from_content(&content);

    // @step Then the returned string is "hi\nbye"
    assert_eq!(text, "hi\nbye");
}

// =========================================================================
// Helper: cast a Dynamic that is supposed to be a Rhai Map.
// =========================================================================
fn as_map(value: &Dynamic) -> Map {
    value
        .clone()
        .try_cast::<Map>()
        .expect("Dynamic was expected to be a Rhai Map")
}

/// Fetch a string field from a Rhai map, panicking if missing or not a string.
fn map_string(map: &Map, key: &str) -> String {
    map.get(key)
        .cloned()
        .unwrap_or(Dynamic::UNIT)
        .into_string()
        .unwrap_or_else(|_| panic!("expected field '{key}' to be a string in map"))
}

// =========================================================================
// Scenario: messages_to_rhai preserves a URL image part verbatim
// =========================================================================
#[test]
fn messages_to_rhai_preserves_url_image_part_verbatim() {
    // @step Given a user Message whose content is Parts containing a Text "look" and an Image with URL "https://example.com/a.png"
    let message = Message {
        role: MessageRole::User,
        content: MessageContent::Parts(vec![
            ContentPart::Text {
                text: "look".to_string(),
            },
            ContentPart::Image {
                source: ImageSource::Url {
                    url: "https://example.com/a.png".to_string(),
                },
            },
        ]),
    };
    let messages = vec![message];

    // @step When I convert the messages slice via messages_to_rhai
    let dyn_value: Dynamic = messages_to_rhai(&messages).expect("messages_to_rhai");
    let array: Array = dyn_value
        .into_typed_array::<Dynamic>()
        .expect("resulting Dynamic is a Rhai Array");

    // @step Then the resulting Rhai array has one message entry
    assert_eq!(array.len(), 1);
    let msg_map = as_map(&array[0]);
    assert_eq!(map_string(&msg_map, "role"), "user");

    let content_dyn = msg_map
        .get("content")
        .cloned()
        .expect("message map has 'content' field");
    let content_array: Array = content_dyn
        .into_typed_array::<Dynamic>()
        .expect("content is an array for Parts");
    assert_eq!(content_array.len(), 2);

    // @step And the message's content array second entry has type "image"
    let image_entry = as_map(&content_array[1]);
    assert_eq!(map_string(&image_entry, "type"), "image");

    // @step And that entry's source map has type "url" and url "https://example.com/a.png"
    let source = image_entry
        .get("source")
        .cloned()
        .expect("image entry has 'source' field");
    let source_map = as_map(&source);
    assert_eq!(map_string(&source_map, "type"), "url");
    assert_eq!(map_string(&source_map, "url"), "https://example.com/a.png");
}

// =========================================================================
// Scenario: messages_to_rhai preserves a base64 image part verbatim
// =========================================================================
#[test]
fn messages_to_rhai_preserves_base64_image_part_verbatim() {
    // @step Given a user Message whose content is Parts containing an Image with Base64 source media_type "image/png" and data "AAA"
    let message = Message {
        role: MessageRole::User,
        content: MessageContent::Parts(vec![ContentPart::Image {
            source: ImageSource::Base64 {
                media_type: "image/png".to_string(),
                data: "AAA".to_string(),
            },
        }]),
    };
    let messages = vec![message];

    // @step When I convert the messages slice via messages_to_rhai
    let dyn_value: Dynamic = messages_to_rhai(&messages).expect("messages_to_rhai");
    let array: Array = dyn_value
        .into_typed_array::<Dynamic>()
        .expect("resulting Dynamic is a Rhai Array");
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

    // @step Then the first content entry has type "image"
    let image_entry = as_map(&content_array[0]);
    assert_eq!(map_string(&image_entry, "type"), "image");

    // @step And its source map has type "base64", media_type "image/png", and data "AAA"
    let source = image_entry
        .get("source")
        .cloned()
        .expect("image entry has 'source' field");
    let source_map = as_map(&source);
    assert_eq!(map_string(&source_map, "type"), "base64");
    assert_eq!(map_string(&source_map, "media_type"), "image/png");
    assert_eq!(map_string(&source_map, "data"), "AAA");
}

// =========================================================================
// Scenario: response_bridge rejects image parts from parse_response
// =========================================================================
#[test]
fn response_bridge_rejects_image_parts_from_parse_response() {
    // @step Given a Rhai response map whose content array contains an entry with type "image"
    let mut source_map = Map::new();
    source_map.insert("type".into(), Dynamic::from("url".to_string()));
    source_map.insert(
        "url".into(),
        Dynamic::from("https://example.com/a.png".to_string()),
    );

    let mut image_part = Map::new();
    image_part.insert("type".into(), Dynamic::from("image".to_string()));
    image_part.insert("source".into(), Dynamic::from_map(source_map));

    let content_array: Array = vec![Dynamic::from_map(image_part)];

    let mut response_map = Map::new();
    response_map.insert("content".into(), Dynamic::from_array(content_array));
    response_map.insert("stop_reason".into(), Dynamic::from("end_turn".to_string()));
    let response_dynamic = Dynamic::from_map(response_map);

    // @step When I call rhai_to_completion_response on that map
    let result = rhai_to_completion_response(response_dynamic);

    // @step Then the call returns Err with a RhaiRuntimeError mentioning unknown content part type "image"
    let err = result.expect_err("response_bridge must reject image parts");
    let msg = err.to_string();
    assert!(
        msg.contains("unknown content part type") && msg.contains("image"),
        "unexpected error message: {msg}"
    );
}

// =========================================================================
// Scenario: Serialize an Image ContentPart with a URL source
// =========================================================================
#[test]
fn image_content_part_serializes_with_url_source() {
    // @step Given a ContentPart::Image whose source is ImageSource::Url "https://example.com/a.png"
    let part = ContentPart::Image {
        source: ImageSource::Url {
            url: "https://example.com/a.png".to_string(),
        },
    };

    // @step When I serialize the content part to JSON
    let value = serde_json::to_value(&part).expect("serialize ContentPart::Image");

    // @step Then the JSON type field is "image"
    assert_eq!(value["type"], "image");
    // @step And the JSON source.type field is "url"
    assert_eq!(value["source"]["type"], "url");
    // @step And the JSON source.url field is "https://example.com/a.png"
    assert_eq!(value["source"]["url"], "https://example.com/a.png");
}

// =========================================================================
// Scenario: Serialize an Image ContentPart with a Base64 source
// =========================================================================
#[test]
fn image_content_part_serializes_with_base64_source() {
    // @step Given a ContentPart::Image whose source is ImageSource::Base64 with media_type "image/png" and data "AAA"
    let part = ContentPart::Image {
        source: ImageSource::Base64 {
            media_type: "image/png".to_string(),
            data: "AAA".to_string(),
        },
    };

    // @step When I serialize the content part to JSON
    let value = serde_json::to_value(&part).expect("serialize ContentPart::Image base64");

    // @step Then the JSON type field is "image"
    assert_eq!(value["type"], "image");
    // @step And the JSON source.type field is "base64"
    assert_eq!(value["source"]["type"], "base64");
    // @step And the JSON source.media_type field is "image/png"
    assert_eq!(value["source"]["media_type"], "image/png");
    // @step And the JSON source.data field is "AAA"
    assert_eq!(value["source"]["data"], "AAA");
}

// =========================================================================
// Scenario: Round-trip an Image ContentPart through JSON
// =========================================================================
#[test]
fn image_content_part_round_trips_through_json() {
    // @step Given a ContentPart::Image with a Base64 source
    let original = ContentPart::Image {
        source: ImageSource::Base64 {
            media_type: "image/jpeg".to_string(),
            data: "QkFS".to_string(),
        },
    };

    // @step When I serialize it to JSON and deserialize the JSON back into a ContentPart
    let json = serde_json::to_string(&original).expect("serialize");
    let decoded: ContentPart = serde_json::from_str(&json).expect("deserialize");

    // @step Then the deserialized value equals the original Image variant
    match decoded {
        ContentPart::Image {
            source: ImageSource::Base64 { media_type, data },
        } => {
            assert_eq!(media_type, "image/jpeg");
            assert_eq!(data, "QkFS");
        }
        other => panic!("round-trip produced unexpected variant: {other:?}"),
    }
}

// =========================================================================
// Scenario: convert_assistant_content still rejects assistant-side images
// =========================================================================
#[test]
fn convert_assistant_content_still_rejects_assistant_side_images() {
    // PROV-091 adds request-side images to ContentPart but explicitly
    // keeps assistant-side images rejected. This regression test locks
    // that invariant in.

    // @step Given a rig OneOrMany of AssistantContent containing an Image variant
    let image_content = rig::completion::AssistantContent::image_base64(
        "AAA",
        Some(rig::completion::message::ImageMediaType::PNG),
        None,
    );
    let choice = rig::OneOrMany::one(image_content);

    // @step When I call convert_assistant_content with a provider name
    let result = convert_assistant_content(choice, "test-provider");

    // @step Then the call returns a ProviderError::Content whose message mentions images not being supported
    let err = result.expect_err("assistant-side images must still be rejected");
    assert!(matches!(err, ProviderError::Content { .. }));
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("image"),
        "unexpected error message: {msg}"
    );
}
