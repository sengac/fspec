#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/copilot-agent-initiator-classification.feature
//!
//! PROV-059: Integration tests that verify the classifier correctly
//! detects tool-call follow-up iterations as agent-initiated after the fix.
//!
//! These tests complement the main `copilot_agent_initiator_test.rs` by
//! exercising the classifier's heuristic path (last-message-role detection)
//! instead of the explicit `metadata.mode` path.

use codelet_providers::copilot::classifier::CopilotRequestClassifier;
use codelet_providers::copilot::header_facade::CopilotHeaderFacade;
use serde_json::json;

/// After PROV-059 fix: tool-call follow-up requests are correctly detected
/// as agent-initiated by the enhanced classifier. The body does NOT need
/// an explicit metadata.mode field — the classifier sees that the last
/// message is a tool result and infers agent mode.
#[test]
fn fixed_tool_followup_correctly_classified_as_agent() {
    let body = json!({
        "model": "gpt-4o",
        "messages": [
            { "role": "user", "content": "fix the bug" },
            { "role": "assistant", "content": "I'll read the file." },
            { "role": "tool", "content": "file content", "tool_call_id": "call_1" }
        ]
    });

    let classification = CopilotRequestClassifier::classify(&body);
    assert!(
        classification.is_agent,
        "PROV-059 FIX: tool follow-up must be classified as agent-initiated"
    );

    let headers = CopilotHeaderFacade::build_headers(&classification, "ghu_token");
    assert_eq!(
        headers.get("x-initiator").map(|v| v.to_str().unwrap()),
        Some("agent"),
        "PROV-059 FIX: tool follow-ups must have x-initiator: agent"
    );
}

/// Genuine user messages still correctly get x-initiator: user.
#[test]
fn user_message_still_correctly_classified_as_user() {
    let body = json!({
        "model": "gpt-4o",
        "messages": [
            { "role": "user", "content": "fix the bug" }
        ]
    });

    let classification = CopilotRequestClassifier::classify(&body);
    assert!(
        !classification.is_agent,
        "genuine user messages must NOT be classified as agent-initiated"
    );

    let headers = CopilotHeaderFacade::build_headers(&classification, "ghu_token");
    assert_eq!(
        headers.get("x-initiator").map(|v| v.to_str().unwrap()),
        Some("user"),
        "genuine user messages must have x-initiator: user"
    );
}
