//! CopilotRequestClassifier — pure function that inspects a request body and
//! returns a [`RequestClassification`] summarising whether the request carries
//! vision content and whether it was initiated by an autonomous agent workflow.
//!
//! PROV-055: Rule 3 — `CopilotRequestClassifier::classify(body)` must return
//! `{ is_vision, is_agent }` by walking the JSON body for chat/completions,
//! /responses, or Anthropic-messages shapes. It performs **no IO and holds no
//! state** — every decision is derived from the body alone.
//!
//! Downstream, [`CopilotHeaderFacade`](super::header_facade::CopilotHeaderFacade)
//! consumes this classification to decide whether to set the
//! `Copilot-Vision-Request` header and which value to use for `x-initiator`.

use serde_json::Value;

/// The per-request classification derived from a Copilot request body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RequestClassification {
    /// `true` if the body carries image content in any supported shape.
    pub is_vision: bool,
    /// `true` if the request was initiated by an autonomous agent workflow
    /// (as opposed to a user sending a message in the TUI).
    pub is_agent: bool,
}

/// Pure-function facade that classifies Copilot request bodies.
pub struct CopilotRequestClassifier;

impl CopilotRequestClassifier {
    /// Classify a Copilot request body.
    ///
    /// # Arguments
    ///
    /// * `body` - The JSON body that would be sent to the Copilot API. May be
    ///   in OpenAI chat/completions shape, OpenAI `/responses` shape, or
    ///   Anthropic messages shape. Unknown shapes classify as neither.
    ///
    /// # Returns
    ///
    /// A [`RequestClassification`] describing the request.
    #[must_use]
    pub fn classify(body: &Value) -> RequestClassification {
        RequestClassification {
            is_vision: detect_vision_content(body),
            is_agent: detect_agent_mode(body),
        }
    }
}

/// Walk the body in every supported shape and return `true` if any image
/// content is present.
fn detect_vision_content(body: &Value) -> bool {
    // Shape 1: OpenAI chat/completions — `messages[].content[]` where a
    // content item has `type == "image_url"` (vanilla OpenAI) or
    // `type == "image"` (Anthropic messages).
    if let Some(messages) = body.get("messages").and_then(Value::as_array) {
        for msg in messages {
            if let Some(content) = msg.get("content").and_then(Value::as_array) {
                for item in content {
                    if let Some(t) = item.get("type").and_then(Value::as_str) {
                        if matches!(t, "image_url" | "image" | "input_image") {
                            return true;
                        }
                    }
                }
            }
        }
    }

    // Shape 2: OpenAI /responses — `input[]` where an input item has
    // `type == "input_image"`.
    if let Some(input) = body.get("input").and_then(Value::as_array) {
        for item in input {
            if let Some(t) = item.get("type").and_then(Value::as_str) {
                if t == "input_image" {
                    return true;
                }
            }
        }
    }

    false
}

/// Detect whether the request should be classified as agent-initiated.
///
/// A request is agent-initiated when:
/// 1. The caller has explicitly set `metadata.mode = "agent"` (DeepSearch,
///    subordinate sessions, scheduled jobs), OR
/// 2. The last message in the conversation is a tool result (`role: "tool"`
///    for OpenAI format, or `role: "user"` with only `tool_result` content
///    for Anthropic format), which means this is an automated follow-up
///    after a tool call — not a genuine user-initiated message.
///
/// This mirrors the VSCode Copilot Chat reference implementation:
/// `userInitiatedRequest: iterationNumber === 0 && !isContinuation && !isSubagent`
fn detect_agent_mode(body: &Value) -> bool {
    // Check 1: Explicit metadata.mode = "agent" flag
    if body
        .get("metadata")
        .and_then(|m| m.get("mode"))
        .and_then(Value::as_str)
        .is_some_and(|s| s == "agent")
    {
        return true;
    }

    // Check 2: Last message is a tool result (OpenAI format: role = "tool")
    // This catches agent loop follow-up iterations where rig sends tool
    // results back to the API for the next assistant turn.
    if let Some(messages) = body.get("messages").and_then(Value::as_array) {
        if let Some(last) = messages.last() {
            if let Some(role) = last.get("role").and_then(Value::as_str) {
                // OpenAI format: tool results have role = "tool"
                if role == "tool" {
                    return true;
                }
                // Anthropic format: tool results are in a "user" message
                // with content items of type "tool_result" only.
                if role == "user" {
                    if let Some(content) = last.get("content").and_then(Value::as_array) {
                        let all_tool_results = !content.is_empty()
                            && content.iter().all(|item| {
                                item.get("type")
                                    .and_then(Value::as_str)
                                    .is_some_and(|t| t == "tool_result")
                            });
                        if all_tool_results {
                            return true;
                        }
                    }
                }
            }
        }
    }

    // Check 3: OpenAI /responses format — last input item is a function_call_output
    if let Some(input) = body.get("input").and_then(Value::as_array) {
        if let Some(last) = input.last() {
            if let Some(t) = last.get("type").and_then(Value::as_str) {
                if t == "function_call_output" {
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn plain_text_chat_completions_is_neither() {
        let body = json!({
            "model": "gpt-4o",
            "messages": [{ "role": "user", "content": "hi" }]
        });
        let c = CopilotRequestClassifier::classify(&body);
        assert!(!c.is_vision);
        assert!(!c.is_agent);
    }

    #[test]
    fn openai_image_url_is_vision() {
        let body = json!({
            "model": "gpt-4o",
            "messages": [{
                "role": "user",
                "content": [
                    { "type": "text", "text": "describe" },
                    { "type": "image_url", "image_url": { "url": "data:image/png;base64,xxx" } }
                ]
            }]
        });
        assert!(CopilotRequestClassifier::classify(&body).is_vision);
    }

    #[test]
    fn anthropic_image_type_is_vision() {
        let body = json!({
            "model": "claude-sonnet-4.5",
            "messages": [{
                "role": "user",
                "content": [
                    { "type": "text", "text": "describe" },
                    { "type": "image", "source": { "type": "base64", "data": "xxx" } }
                ]
            }]
        });
        assert!(CopilotRequestClassifier::classify(&body).is_vision);
    }

    #[test]
    fn responses_input_image_is_vision() {
        let body = json!({
            "model": "gpt-5",
            "input": [
                { "type": "input_text", "text": "what is this" },
                { "type": "input_image", "image_url": "data:image/png;base64,xxx" }
            ]
        });
        assert!(CopilotRequestClassifier::classify(&body).is_vision);
    }

    #[test]
    fn metadata_mode_agent_marks_agent() {
        let body = json!({
            "model": "gpt-5",
            "messages": [{ "role": "user", "content": "run this" }],
            "metadata": { "mode": "agent" }
        });
        assert!(CopilotRequestClassifier::classify(&body).is_agent);
    }

    #[test]
    fn missing_metadata_is_not_agent() {
        let body = json!({
            "model": "gpt-4",
            "messages": [{ "role": "user", "content": "hi" }]
        });
        assert!(!CopilotRequestClassifier::classify(&body).is_agent);
    }

    // PROV-059: Tool result follow-ups are agent-initiated
    #[test]
    fn last_message_tool_result_is_agent() {
        let body = json!({
            "model": "gpt-4o",
            "messages": [
                { "role": "user", "content": "fix the bug" },
                { "role": "assistant", "content": "I'll read the file." },
                { "role": "tool", "content": "file content here", "tool_call_id": "call_1" }
            ]
        });
        assert!(
            CopilotRequestClassifier::classify(&body).is_agent,
            "tool result as last message must be classified as agent-initiated"
        );
    }

    #[test]
    fn last_message_user_text_is_not_agent() {
        let body = json!({
            "model": "gpt-4o",
            "messages": [
                { "role": "assistant", "content": "How can I help?" },
                { "role": "user", "content": "fix the bug" }
            ]
        });
        assert!(
            !CopilotRequestClassifier::classify(&body).is_agent,
            "user text as last message must NOT be agent-initiated"
        );
    }

    #[test]
    fn anthropic_tool_result_only_user_message_is_agent() {
        let body = json!({
            "model": "claude-sonnet-4.5",
            "messages": [
                { "role": "user", "content": [
                    { "type": "tool_result", "tool_use_id": "tu_1", "content": "result" }
                ]}
            ]
        });
        assert!(
            CopilotRequestClassifier::classify(&body).is_agent,
            "Anthropic tool_result-only user message must be agent-initiated"
        );
    }

    #[test]
    fn anthropic_mixed_user_message_is_not_agent() {
        let body = json!({
            "model": "claude-sonnet-4.5",
            "messages": [
                { "role": "user", "content": [
                    { "type": "tool_result", "tool_use_id": "tu_1", "content": "result" },
                    { "type": "text", "text": "Now fix it" }
                ]}
            ]
        });
        assert!(
            !CopilotRequestClassifier::classify(&body).is_agent,
            "user message with mixed content (text + tool_result) should be user-initiated"
        );
    }

    #[test]
    fn responses_api_function_call_output_is_agent() {
        let body = json!({
            "model": "gpt-5",
            "input": [
                { "type": "message", "role": "user", "content": "run task" },
                { "type": "function_call", "name": "read", "call_id": "c1", "arguments": "{}" },
                { "type": "function_call_output", "call_id": "c1", "output": "file content" }
            ]
        });
        assert!(
            CopilotRequestClassifier::classify(&body).is_agent,
            "Responses API function_call_output as last input must be agent-initiated"
        );
    }
}
