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
/// 2. First user message to include cache_control for context caching
///
/// # Arguments
/// * `body` - The request body as a JSON Value
/// * `is_oauth` - Whether OAuth mode is enabled (affects system prompt structure)
/// * `oauth_prefix` - Optional OAuth prefix that should NOT have cache_control
///
/// # Returns
/// The transformed request body
pub fn transform_request_body(body: &Value, is_oauth: bool, oauth_prefix: Option<&str>) -> Value {
    let mut transformed = body.clone();

    // Transform system prompt
    if let Some(system) = body.get("system") {
        if system.is_string() {
            let system_str = system.as_str().unwrap_or_default();
            transformed["system"] = transform_system_prompt(system_str, is_oauth, oauth_prefix);
        }
    }

    // Transform first user message
    transform_user_message_cache_control(&mut transformed);

    transformed
}

/// Transform system prompt from string to array format with cache_control
///
/// # Arguments
/// * `system_text` - The system prompt text
/// * `is_oauth` - Whether OAuth mode is enabled
/// * `oauth_prefix` - Optional OAuth prefix for OAuth mode
///
/// # Returns
/// JSON array of content blocks with cache_control metadata
pub fn transform_system_prompt(
    system_text: &str,
    is_oauth: bool,
    oauth_prefix: Option<&str>,
) -> Value {
    if is_oauth {
        let prefix =
            oauth_prefix.unwrap_or("You are Claude Code, Anthropic's official CLI for Claude.");

        // OAuth mode: first block is prefix without cache_control,
        // second block is additional text with cache_control (if non-empty)
        let additional_text = system_text
            .strip_prefix(prefix)
            .map(str::trim)
            .unwrap_or(system_text);

        // PROV-006 FIX: Don't create empty text blocks with cache_control
        // Anthropic API rejects: "cache_control cannot be set for empty text blocks"
        if additional_text.is_empty() {
            // Only prefix, put cache_control on the prefix itself
            json!([
                {
                    "type": "text",
                    "text": prefix,
                    "cache_control": { "type": "ephemeral" }
                }
            ])
        } else {
            json!([
                {
                    "type": "text",
                    "text": prefix
                },
                {
                    "type": "text",
                    "text": additional_text,
                    "cache_control": { "type": "ephemeral" }
                }
            ])
        }
    } else {
        // API key mode: single block with cache_control
        json!([
            {
                "type": "text",
                "text": system_text,
                "cache_control": { "type": "ephemeral" }
            }
        ])
    }
}

/// Transform first user message content to include cache_control
///
/// This enables caching of user context (like file contents) that may be
/// repeated across turns.
pub fn transform_user_message_cache_control(body: &mut Value) {
    if let Some(messages) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        for msg in messages.iter_mut() {
            // Only transform first user message
            if msg.get("role").and_then(|r| r.as_str()) == Some("user") {
                if let Some(content) = msg.get("content") {
                    if content.is_string() {
                        let text = content.as_str().unwrap_or_default();
                        msg["content"] = json!([
                            {
                                "type": "text",
                                "text": text,
                                "cache_control": { "type": "ephemeral" }
                            }
                        ]);
                    }
                }
                // Only transform the first user message
                break;
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
        let result = transform_system_prompt("You are helpful", false, None);
        assert!(result.is_array());
        let array = result.as_array().unwrap();
        assert_eq!(array.len(), 1);
        assert_eq!(array[0]["type"], "text");
        assert_eq!(array[0]["text"], "You are helpful");
        assert_eq!(array[0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_transform_system_oauth_mode() {
        let prefix = "OAuth prefix";
        let result = transform_system_prompt("OAuth prefix Additional text", true, Some(prefix));
        assert!(result.is_array());
        let array = result.as_array().unwrap();
        assert_eq!(array.len(), 2);
        // First block: prefix without cache_control
        assert_eq!(array[0]["text"], prefix);
        assert!(array[0].get("cache_control").is_none());
        // Second block: additional text with cache_control
        assert_eq!(array[1]["text"], "Additional text");
        assert_eq!(array[1]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_transform_system_oauth_mode_empty_additional_text() {
        // PROV-006 FIX: When additional text is empty, don't create empty block
        // Anthropic API rejects: "cache_control cannot be set for empty text blocks"
        let prefix = "OAuth prefix";
        let result = transform_system_prompt("OAuth prefix", true, Some(prefix));
        assert!(result.is_array());
        let array = result.as_array().unwrap();
        // Should be single block with cache_control on prefix
        assert_eq!(array.len(), 1);
        assert_eq!(array[0]["text"], prefix);
        assert_eq!(array[0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn test_transform_system_oauth_mode_whitespace_only_additional_text() {
        // Whitespace-only additional text should also result in single block
        let prefix = "OAuth prefix";
        let result = transform_system_prompt("OAuth prefix   ", true, Some(prefix));
        assert!(result.is_array());
        let array = result.as_array().unwrap();
        assert_eq!(array.len(), 1);
        assert_eq!(array[0]["text"], prefix);
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
}
