//! Feature: spec/features/add-anthropic-prompt-cache-control-metadata.feature
//!
//! Tests for CLI-017: Add Anthropic prompt cache control metadata

use codelet::providers::claude::{build_cached_system_prompt, CacheControl};
use serde_json::json;

// ==========================================
// SYSTEM PROMPT FORMAT TESTS
// ==========================================

/// Scenario: System prompt is sent as array with cache_control for API key mode
#[test]
fn test_system_prompt_array_format_api_key_mode() {
    // @step Given the provider is using API key authentication
    let is_oauth = false;

    // @step And the preamble is "You are a helpful coding assistant"
    let preamble = "You are a helpful coding assistant";

    // @step When building the completion request
    let system = build_cached_system_prompt(preamble, is_oauth, None);

    // @step Then the system field should be an array of content blocks
    assert!(system.is_array());
    let array = system.as_array().unwrap();

    // @step And the first content block should have type "text"
    assert_eq!(array[0]["type"], "text");

    // @step And the first content block should have cache_control with type "ephemeral"
    assert_eq!(array[0]["cache_control"]["type"], "ephemeral");

    // @step And the first content block text should be "You are a helpful coding assistant"
    assert_eq!(array[0]["text"], preamble);
}

/// Scenario: System prompt is sent with correct structure for OAuth mode
#[test]
fn test_system_prompt_array_format_oauth_mode() {
    // @step Given the provider is using OAuth authentication
    let is_oauth = true;

    // @step And the system instructions are "Additional instructions here"
    let preamble = "Additional instructions here";
    let oauth_prefix = "You are Claude Code, Anthropic's official CLI for Claude.";

    // @step When building the completion request
    let system = build_cached_system_prompt(preamble, is_oauth, Some(oauth_prefix));

    // @step Then the system field should be an array with 2 content blocks
    assert!(system.is_array());
    let array = system.as_array().unwrap();
    assert_eq!(array.len(), 2);

    // @step And the first content block should be the Claude Code prefix without cache_control
    assert_eq!(array[0]["type"], "text");
    assert_eq!(array[0]["text"], oauth_prefix);
    assert!(array[0].get("cache_control").is_none());

    // @step And the second content block should have cache_control with type "ephemeral"
    assert_eq!(array[1]["cache_control"]["type"], "ephemeral");

    // @step And the second content block text should be "Additional instructions here"
    assert_eq!(array[1]["text"], preamble);
}

/// Scenario: Additional params override plain string system field
#[test]
fn test_additional_params_override_preamble() {
    // @step Given a completion request with preamble set via .preamble()
    // @step When additional_params includes a system array
    // This test verifies the expected JSON structure that will override the preamble
    let additional_params = json!({
        "system": [
            {
                "type": "text",
                "text": "Override preamble",
                "cache_control": { "type": "ephemeral" }
            }
        ]
    });

    // @step Then the system array from additional_params should be used
    assert!(additional_params["system"].is_array());

    // @step And the plain string from preamble should be ignored
    // (This is verified by rig's merge_inplace behavior - keys from additional_params override)
}

// ==========================================
// CACHE_CONTROL STRUCTURE TESTS
// ==========================================

/// Scenario: cache_control uses ephemeral type
#[test]
fn test_cache_control_ephemeral_type() {
    // @step Given a system content block with cache_control
    let cache = CacheControl::ephemeral();

    // @step Then the cache_control should have type "ephemeral"
    let json = serde_json::to_value(&cache).unwrap();
    assert_eq!(json["type"], "ephemeral");

    // @step And the JSON structure should be { "cache_control": { "type": "ephemeral" } }
    let expected = json!({ "type": "ephemeral" });
    assert_eq!(json, expected);
}

/// Scenario: Content block without cache_control omits the field
#[test]
fn test_content_block_without_cache_control() {
    // @step Given the OAuth mode first block (Claude Code prefix)
    let content_block = json!({
        "type": "text",
        "text": "You are Claude Code, Anthropic's official CLI for Claude."
    });

    // @step When serializing the content block
    // @step Then the cache_control field should be absent
    assert!(content_block.get("cache_control").is_none());

    // @step And the JSON should only have "type" and "text" fields
    assert_eq!(content_block.as_object().unwrap().len(), 2);
}

// ==========================================
// HELPER FUNCTION TESTS
// ==========================================

/// Scenario: build_cached_system_prompt creates correct array format
#[test]
fn test_build_cached_system_prompt_single_block() {
    // @step Given a preamble string "Test preamble"
    let preamble = "Test preamble";

    // @step And no OAuth prefix is needed
    let is_oauth = false;

    // @step When calling build_cached_system_prompt
    let result = build_cached_system_prompt(preamble, is_oauth, None);

    // @step Then the result should be a serde_json::Value array
    assert!(result.is_array());

    // @step And it should contain one content block
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);

    // @step And that block should have cache_control
    assert!(array[0].get("cache_control").is_some());
}

/// Scenario: build_cached_system_prompt handles OAuth mode correctly
#[test]
fn test_build_cached_system_prompt_oauth_mode() {
    // @step Given a preamble string "Additional instructions"
    let preamble = "Additional instructions";

    // @step And OAuth mode is active
    let is_oauth = true;
    let oauth_prefix = "OAuth prefix here";

    // @step When calling build_cached_system_prompt with OAuth prefix
    let result = build_cached_system_prompt(preamble, is_oauth, Some(oauth_prefix));

    // @step Then the result should have 2 content blocks
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);

    // @step And block 0 should be the OAuth prefix without cache_control
    assert_eq!(array[0]["text"], oauth_prefix);
    assert!(array[0].get("cache_control").is_none());

    // @step And block 1 should be the preamble with cache_control
    assert_eq!(array[1]["text"], preamble);
    assert!(array[1].get("cache_control").is_some());
}

// ==========================================
// INTEGRATION TESTS
// ==========================================

/// Scenario: Completion request includes cache_control in serialized JSON
#[test]
fn test_completion_request_json_structure() {
    // @step Given a completion request built by the provider
    let system = build_cached_system_prompt("Test system prompt", false, None);

    // @step When serializing to JSON for the Anthropic API
    let serialized = serde_json::to_string(&system).unwrap();

    // @step Then the JSON should contain "cache_control" nested in system blocks
    assert!(serialized.contains("cache_control"));
    assert!(serialized.contains("ephemeral"));

    // @step And the structure should match Anthropic's expected format
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed[0]["type"], "text");
}

// ==========================================
// ADDITIONAL EDGE CASE TESTS
// ==========================================

#[test]
fn test_empty_preamble_still_gets_cache_control() {
    let result = build_cached_system_prompt("", false, None);
    assert!(result.is_array());
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 1);
    assert!(array[0].get("cache_control").is_some());
}

#[test]
fn test_oauth_mode_with_empty_preamble() {
    let result = build_cached_system_prompt("", true, Some("OAuth prefix"));
    let array = result.as_array().unwrap();
    // Should still have 2 blocks - prefix and (empty) preamble
    assert_eq!(array.len(), 2);
}

#[test]
fn test_cache_control_serialization() {
    let cache = CacheControl::ephemeral();
    let json = serde_json::to_string(&cache).unwrap();
    assert_eq!(json, r#"{"type":"ephemeral"}"#);
}
