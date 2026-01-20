//! Caching HTTP Client for Anthropic API
//!
//! This module provides request body transformation to enable Anthropic prompt caching.
//! It transforms system prompts from plain strings to array format with cache_control metadata.
//!
//! The rig library sends system prompts as plain strings, but Anthropic's cache_control
//! requires the array format:
//!
//! ```json
//! {
//!   "system": [
//!     { "type": "text", "text": "...", "cache_control": { "type": "ephemeral" } }
//!   ]
//! }
//! ```
//!
//! # TOOL-008: Uses SystemPromptFacade
//! This module uses the SystemPromptFacade trait implementations from codelet_tools
//! for provider-specific system prompt formatting.

use codelet_tools::facade::{select_claude_facade, CLAUDE_CODE_PROMPT_PREFIX};
use serde_json::{json, Value};

/// Check if a URL should have its request body transformed for cache control
///
/// Only Anthropic /v1/messages requests need transformation.
pub fn should_transform_request(url: &str) -> bool {
    // Only transform Anthropic messages endpoint
    url.contains("api.anthropic.com") && url.contains("/v1/messages")
}

/// Transform a request body to enable Anthropic prompt caching
///
/// This function transforms:
/// 1. System prompt from string to array format with cache_control
/// 2. Final message to include cache_control for incremental caching
///    (per Anthropic docs: "mark the final block of the final message")
///
/// # TOOL-008: Uses SystemPromptFacade
/// For OAuth mode, the system prompt is expected to already have the prefix stripped.
/// The facade handles adding the prefix and proper cache_control formatting.
///
/// # Arguments
/// * `body` - The request body as a JSON Value
/// * `is_oauth` - Whether OAuth mode is enabled (affects system prompt structure)
///
/// # Returns
/// The transformed request body
pub fn transform_request_body(body: &Value, is_oauth: bool) -> Value {
    let mut transformed = body.clone();

    // Transform system prompt using facade (TOOL-008)
    if let Some(system) = body.get("system") {
        if system.is_string() {
            let system_str = system.as_str().unwrap_or_default();
            // For OAuth mode, strip the prefix if present (facade will re-add it)
            let preamble = if is_oauth {
                system_str
                    .strip_prefix(CLAUDE_CODE_PROMPT_PREFIX)
                    .map(str::trim)
                    .unwrap_or(system_str)
            } else {
                system_str
            };
            transformed["system"] = transform_system_prompt(preamble, is_oauth);
        }
    }

    // Transform final message for incremental caching (per Anthropic docs)
    transform_user_message_cache_control(&mut transformed);

    transformed
}

/// Transform system prompt from string to array format with cache_control
///
/// # TOOL-008: Uses SystemPromptFacade
/// This function now delegates to the appropriate Claude facade for consistent
/// formatting across the codebase.
///
/// # Arguments
/// * `preamble` - The preamble/additional text (NOT the combined prefix+preamble for OAuth)
/// * `is_oauth` - Whether OAuth mode is enabled
///
/// # Returns
/// JSON array of content blocks with cache_control metadata
pub fn transform_system_prompt(preamble: &str, is_oauth: bool) -> Value {
    let facade = select_claude_facade(is_oauth);
    facade.format_for_api(preamble)
}

/// Transform the final message content to include cache_control for incremental caching
///
/// Per Anthropic's documentation: "During each turn, we mark the final block of the
/// final message with cache_control so the conversation can be incrementally cached."
///
/// This enables caching of the entire conversation prefix on each turn, allowing
/// subsequent requests to reuse the cached context.
pub fn transform_user_message_cache_control(body: &mut Value) {
    if let Some(messages) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        if messages.is_empty() {
            return;
        }

        // Work backwards to find the last message with content
        for i in (0..messages.len()).rev() {
            let msg = &messages[i];
            let content = msg.get("content");

            let Some(content) = content else {
                continue;
            };

            // Handle string content - convert to array with cache_control
            if content.is_string() {
                let text = content.as_str().unwrap_or_default();
                messages[i]["content"] = json!([
                    {
                        "type": "text",
                        "text": text,
                        "cache_control": { "type": "ephemeral" }
                    }
                ]);
                return;
            }

            // Handle array content - add cache_control to the last block
            if let Some(content_array) = content.as_array() {
                if !content_array.is_empty() {
                    let last_idx = content_array.len() - 1;
                    messages[i]["content"][last_idx]["cache_control"] =
                        json!({ "type": "ephemeral" });
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_transform_anthropic_messages() {
        assert!(should_transform_request(
            "https://api.anthropic.com/v1/messages"
        ));
    }

    #[test]
    fn test_should_not_transform_anthropic_models() {
        assert!(!should_transform_request(
            "https://api.anthropic.com/v1/models"
        ));
    }

    #[test]
    fn test_should_not_transform_openai() {
        assert!(!should_transform_request(
            "https://api.openai.com/v1/chat/completions"
        ));
    }

    #[test]
    fn test_transform_system_api_key_mode() {
        // TOOL-008: New signature - just pass preamble and is_oauth
        let result = transform_system_prompt("You are helpful", false);
        assert!(result.is_array());
        let array = result.as_array().unwrap();
        assert_eq!(array.len(), 1);
        assert_eq!(array[0]["type"], "text");
        assert_eq!(array[0]["text"], "You are helpful");
        assert_eq!(array[0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_transform_system_oauth_mode() {
        // TOOL-008: For OAuth, pass ADDITIONAL preamble (facade adds prefix)
        let result = transform_system_prompt("Additional text", true);
        assert!(result.is_array());
        let array = result.as_array().unwrap();
        assert_eq!(array.len(), 2);
        // First block: Claude Code prefix without cache_control
        assert!(array[0]["text"]
            .as_str()
            .unwrap()
            .starts_with("You are Claude Code"));
        assert!(array[0].get("cache_control").is_none());
        // Second block: additional text with cache_control
        assert_eq!(array[1]["text"], "Additional text");
        assert_eq!(array[1]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_transform_system_oauth_mode_empty_additional_text() {
        // PROV-006 FIX: When additional text is empty, don't create empty block
        // Anthropic API rejects: "cache_control cannot be set for empty text blocks"
        let result = transform_system_prompt("", true);
        assert!(result.is_array());
        let array = result.as_array().unwrap();
        // Should be single block with cache_control on prefix
        assert_eq!(array.len(), 1);
        assert!(array[0]["text"]
            .as_str()
            .unwrap()
            .starts_with("You are Claude Code"));
        assert_eq!(array[0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_transform_system_oauth_mode_whitespace_only_additional_text() {
        // Whitespace-only additional text should also result in single block
        let result = transform_system_prompt("   ", true);
        assert!(result.is_array());
        let array = result.as_array().unwrap();
        assert_eq!(array.len(), 1);
        assert!(array[0]["text"]
            .as_str()
            .unwrap()
            .starts_with("You are Claude Code"));
        assert_eq!(array[0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_transform_user_message() {
        let mut body = json!({
            "system": "System prompt",
            "messages": [
                {
                    "role": "user",
                    "content": "Hello"
                }
            ]
        });

        transform_user_message_cache_control(&mut body);

        let messages = body["messages"].as_array().unwrap();
        let content = &messages[0]["content"];
        assert!(content.is_array());
        let content_array = content.as_array().unwrap();
        assert_eq!(content_array[0]["text"], "Hello");
        assert_eq!(content_array[0]["cache_control"]["type"], "ephemeral");
    }

    // Feature: spec/features/optimize-prompt-caching-for-multi-turn-conversations.feature
    // PROV-001: Tests for multi-turn prompt caching optimization

    #[test]
    fn test_single_turn_caches_final_message() {
        // @step Given a request body with system prompt and one user message
        let mut body = json!({
            "system": "System prompt",
            "messages": [
                { "role": "user", "content": "Hello" }
            ]
        });

        // @step When transform_user_message_cache_control is applied
        transform_user_message_cache_control(&mut body);

        // @step Then the final message should have cache_control applied
        let messages = body["messages"].as_array().unwrap();
        let content = messages[0]["content"].as_array().unwrap();
        assert_eq!(content[0]["cache_control"]["type"], "ephemeral");

        // @step And the content should be transformed to array format with cache_control metadata
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[0]["text"], "Hello");
    }

    #[test]
    fn test_user_message_content_transformed_to_array_format() {
        // @step Given a message with string content "Hello"
        let mut body = json!({
            "messages": [
                { "role": "user", "content": "Hello" }
            ]
        });

        // @step When cache_control is applied to that message
        transform_user_message_cache_control(&mut body);

        let messages = body["messages"].as_array().unwrap();
        let content = messages[0]["content"].as_array().unwrap();

        // @step Then the content should be an array with one object
        assert_eq!(content.len(), 1);

        // @step And the object should have type "text"
        assert_eq!(content[0]["type"], "text");

        // @step And the object should have text "Hello"
        assert_eq!(content[0]["text"], "Hello");

        // @step And the object should have cache_control with type "ephemeral"
        assert_eq!(content[0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_two_turn_caches_final_message() {
        // @step Given a request body with system prompt, user1, assistant1, and user2
        let mut body = json!({
            "system": "System prompt",
            "messages": [
                { "role": "user", "content": "First message" },
                { "role": "assistant", "content": "Response 1" },
                { "role": "user", "content": "Second message" }
            ]
        });

        // @step When transform_user_message_cache_control is applied
        transform_user_message_cache_control(&mut body);

        let messages = body["messages"].as_array().unwrap();

        // @step Then the final message (user2) should have cache_control applied
        let content2 = messages[2]["content"].as_array().unwrap();
        assert_eq!(content2[0]["cache_control"]["type"], "ephemeral");

        // @step And the first user message should not have cache_control applied
        assert!(messages[0]["content"].is_string());
    }

    #[test]
    fn test_three_turn_caches_final_message() {
        // @step Given a request body with system prompt, user1, assistant1, user2, assistant2, and user3
        let mut body = json!({
            "system": "System prompt",
            "messages": [
                { "role": "user", "content": "First message" },
                { "role": "assistant", "content": "Response 1" },
                { "role": "user", "content": "Second message" },
                { "role": "assistant", "content": "Response 2" },
                { "role": "user", "content": "Third message" }
            ]
        });

        // @step When transform_user_message_cache_control is applied
        transform_user_message_cache_control(&mut body);

        let messages = body["messages"].as_array().unwrap();

        // @step Then the final message (user3) should have cache_control applied
        let content3 = messages[4]["content"].as_array().unwrap();
        assert_eq!(content3[0]["cache_control"]["type"], "ephemeral");

        // @step And no other messages should have cache_control applied
        assert!(messages[0]["content"].is_string());
        assert!(messages[2]["content"].is_string());
    }

    #[test]
    fn test_multi_turn_with_tool_use_caches_final_message() {
        // @step Given a request body with system prompt, user1, assistant1 with tool_use, user2 with tool_result, assistant2, user3, assistant3, and user4
        let mut body = json!({
            "system": "System prompt",
            "messages": [
                { "role": "user", "content": "First message" },
                { "role": "assistant", "content": [{ "type": "tool_use", "id": "1", "name": "test", "input": {} }] },
                { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": "1", "content": "result" }] },
                { "role": "assistant", "content": "Response 2" },
                { "role": "user", "content": "Third message" },
                { "role": "assistant", "content": "Response 3" },
                { "role": "user", "content": "Fourth message" }
            ]
        });

        // @step When transform_user_message_cache_control is applied
        transform_user_message_cache_control(&mut body);

        let messages = body["messages"].as_array().unwrap();

        // @step Then the final message (user4) should have cache_control applied
        let content4 = messages[6]["content"].as_array().unwrap();
        assert_eq!(content4[0]["cache_control"]["type"], "ephemeral");

        // @step And all other messages should not have cache_control applied
        // First user message - should be string (no cache_control)
        assert!(messages[0]["content"].is_string());
        // Second user message (tool_result) - should remain unchanged array without cache_control
        let tool_result = messages[2]["content"].as_array().unwrap();
        assert!(tool_result[0].get("cache_control").is_none());
        // Third user message - should be string (no cache_control)
        assert!(messages[4]["content"].is_string());
    }

    #[test]
    fn test_array_content_has_cache_control_added_to_last_block() {
        // @step Given a message with array content containing multiple blocks
        let mut body = json!({
            "messages": [
                {
                    "role": "user",
                    "content": [
                        { "type": "text", "text": "First block" },
                        { "type": "text", "text": "Second block" },
                        { "type": "text", "text": "Third block" }
                    ]
                }
            ]
        });

        // @step When cache_control is applied to that message
        transform_user_message_cache_control(&mut body);

        let messages = body["messages"].as_array().unwrap();
        let content = messages[0]["content"].as_array().unwrap();

        // @step Then the last block of the array should have cache_control added
        assert_eq!(content[2]["cache_control"]["type"], "ephemeral");

        // @step And other blocks should not have cache_control added
        assert!(content[0].get("cache_control").is_none());
        assert!(content[1].get("cache_control").is_none());
    }
}
