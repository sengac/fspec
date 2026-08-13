//! Copilot prompt cache control injection (PROV-058).
//!
//! Pure functions that inject `copilot_cache_control: { type: "ephemeral" }`
//! into the JSON body of outgoing Copilot API requests. This enables
//! server-side prompt caching for Claude models routed through the Copilot
//! proxy, matching the behaviour of the official Copilot CLI and opencode.
//!
//! Cache control is ONLY applied when the model is a Claude-family model
//! (model ID starts with `"claude-"`). GPT and Gemini models do not use
//! `copilot_cache_control`.
//!
//! ## Injection points (matching Copilot CLI behaviour)
//!
//! 1. **System message** — always gets `copilot_cache_control`
//! 2. **Last tool definition** — the last entry in the `tools` array
//! 3. **Last non-user message** — the last assistant/tool message before
//!    the final user turn (the cache breakpoint)
//!
//! ## Response-side cached token tracking
//!
//! The Copilot proxy returns `usage.prompt_tokens_details.cached_tokens`
//! in the standard OpenAI response format. Because `CopilotProvider` is
//! built on rig's OpenAI completion client, the existing OpenAI response
//! parser in rig-core already extracts `cached_tokens` and maps it to
//! `Usage::cache_read_input_tokens` — both for non-streaming responses
//! (via `ProviderResponseExt`) and streaming SSE events (via
//! `StreamingCompletionResponse`). No additional Copilot-specific parsing
//! is needed for cached token propagation to the TUI display.

use serde_json::{json, Value};

use super::model_family::is_claude_model;

/// The cache control annotation injected onto messages and tools.
const CACHE_CONTROL: &str = "copilot_cache_control";

/// Inject `copilot_cache_control` breakpoints into a Copilot chat
/// completions request body **in place**.
///
/// This function is a no-op if:
/// - The `model` field is missing or is not a Claude-family model
/// - The body is not a JSON object
///
/// # Arguments
///
/// * `body` - Mutable reference to the parsed JSON request body. Modified
///   in place to add `copilot_cache_control` fields.
pub fn inject_cache_control(body: &mut Value) {
    let is_eligible = body
        .get("model")
        .and_then(Value::as_str)
        .is_some_and(is_claude_model);

    if !is_eligible {
        return;
    }

    let ephemeral = json!({ "type": "ephemeral" });

    // 1. System message — tag the first system message
    if let Some(messages) = body.get_mut("messages").and_then(Value::as_array_mut) {
        tag_first_system_message(messages, &ephemeral);
        tag_last_non_user_message(messages, &ephemeral);
    }

    // 2. Last tool definition — tag the last entry in the tools array
    tag_last_tool(body, &ephemeral);
}

/// Tag the first message with `role == "system"` with cache control.
fn tag_first_system_message(messages: &mut [Value], ephemeral: &Value) {
    for msg in messages.iter_mut() {
        if msg.get("role").and_then(Value::as_str) == Some("system") {
            if let Some(obj) = msg.as_object_mut() {
                obj.insert(CACHE_CONTROL.to_string(), ephemeral.clone());
            }
            break; // only the first system message
        }
    }
}

/// Tag the last assistant or tool message as the cache breakpoint.
///
/// This is the last message before the final user turn — the ideal
/// position for a cache breakpoint because everything before it is
/// stable across turns.
fn tag_last_non_user_message(messages: &mut [Value], ephemeral: &Value) {
    let last_non_user_idx = messages
        .iter()
        .enumerate()
        .rev()
        .find(|(_, msg)| {
            let role = msg.get("role").and_then(Value::as_str).unwrap_or("");
            matches!(role, "assistant" | "tool")
        })
        .map(|(i, _)| i);

    if let Some(idx) = last_non_user_idx {
        if let Some(obj) = messages[idx].as_object_mut() {
            obj.insert(CACHE_CONTROL.to_string(), ephemeral.clone());
        }
    }
}

/// Tag the last tool definition in the `tools` array with cache control.
fn tag_last_tool(body: &mut Value, ephemeral: &Value) {
    if let Some(tools) = body.get_mut("tools").and_then(Value::as_array_mut) {
        if let Some(last_tool) = tools.last_mut() {
            if let Some(obj) = last_tool.as_object_mut() {
                obj.insert(CACHE_CONTROL.to_string(), ephemeral.clone());
            }
        }
    }
}

#[cfg(test)]
#[path = "prompt_cache_tests.rs"]
mod tests;
