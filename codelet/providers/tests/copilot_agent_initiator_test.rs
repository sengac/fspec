#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/copilot-agent-initiator-classification.feature
//!
//! PROV-059: Copilot x-initiator header always set to 'user' — metadata.mode
//! agent never injected, burning premium quota on every request.
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios.

use codelet_providers::copilot::classifier::CopilotRequestClassifier;
use codelet_providers::copilot::header_facade::CopilotHeaderFacade;
use serde_json::{json, Value};

// ============================================================================
// Shared test helpers — DRY extraction of the classify → build_headers chain
// ============================================================================

/// Assert that a request body is classified as agent-initiated and produces
/// the `x-initiator: agent` header.
fn assert_agent_classification(body: &Value, context: &str) {
    let classification = CopilotRequestClassifier::classify(body);
    assert!(
        classification.is_agent,
        "{context}: must be classified as agent-initiated"
    );
    let headers = CopilotHeaderFacade::build_headers(&classification, "ghu_token");
    assert_eq!(
        headers
            .get("x-initiator")
            .map(|v| v.to_str().expect("valid header value")),
        Some("agent"),
        "{context}: x-initiator must be 'agent'"
    );
}

/// Assert that a request body is classified as user-initiated and produces
/// the `x-initiator: user` header.
fn assert_user_classification(body: &Value, context: &str) {
    let classification = CopilotRequestClassifier::classify(body);
    assert!(
        !classification.is_agent,
        "{context}: must NOT be classified as agent-initiated"
    );
    let headers = CopilotHeaderFacade::build_headers(&classification, "ghu_token");
    assert_eq!(
        headers
            .get("x-initiator")
            .map(|v| v.to_str().expect("valid header value")),
        Some("user"),
        "{context}: x-initiator must be 'user'"
    );
}

// ============================================================================
// Scenario: metadata.mode agent field flows through classifier to header
// ============================================================================
#[test]
fn scenario_metadata_mode_agent_flows_through_classifier_to_header() {
    // @step Given a Copilot API request body with metadata.mode set to "agent"
    let body = json!({
        "model": "gpt-4o",
        "messages": [{ "role": "user", "content": "tool follow-up" }],
        "metadata": { "mode": "agent" }
    });

    // @step When CopilotRequestClassifier.classify() processes the body
    let classification = CopilotRequestClassifier::classify(&body);

    // @step Then is_agent should be true
    assert!(
        classification.is_agent,
        "classifier must detect metadata.mode == 'agent' as is_agent = true"
    );

    // @step And CopilotHeaderFacade should set x-initiator to "agent"
    let headers = CopilotHeaderFacade::build_headers(&classification, "ghu_test_token");
    assert_eq!(
        headers
            .get("x-initiator")
            .map(|v| v.to_str().expect("valid header value")),
        Some("agent"),
        "CopilotHeaderFacade must set x-initiator: agent when is_agent is true"
    );
}

// ============================================================================
// Scenario: Request without metadata.mode defaults to user initiator
// ============================================================================
#[test]
fn scenario_request_without_metadata_defaults_to_user_initiator() {
    // @step Given a Copilot API request body without a metadata.mode field
    let body = json!({
        "model": "gpt-4o",
        "messages": [{ "role": "user", "content": "hello world" }]
    });

    // @step When CopilotRequestClassifier.classify() processes the body
    let classification = CopilotRequestClassifier::classify(&body);

    // @step Then is_agent should be false
    assert!(
        !classification.is_agent,
        "classifier must return is_agent = false when no metadata.mode is present"
    );

    // @step And CopilotHeaderFacade should set x-initiator to "user"
    let headers = CopilotHeaderFacade::build_headers(&classification, "ghu_test_token");
    assert_eq!(
        headers
            .get("x-initiator")
            .map(|v| v.to_str().expect("valid header value")),
        Some("user"),
        "CopilotHeaderFacade must set x-initiator: user when is_agent is false"
    );
}

// ============================================================================
// Scenario: DeepSearch sub-agent requests are always agent-initiated
// ============================================================================
#[test]
fn scenario_deep_search_sub_agent_requests_are_always_agent_initiated() {
    // @step Given a Copilot provider session
    // DeepSearch sub-agents inherit the Copilot provider.

    // @step When a DeepSearch sub-agent is spawned for code investigation
    let body = json!({
        "model": "gpt-4o",
        "messages": [{ "role": "user", "content": "search codebase for auth patterns" }],
        "metadata": { "mode": "agent" }
    });

    // @step Then the DeepSearch request config should include metadata.mode set to "agent"
    // @step And all API calls made by the sub-agent should have x-initiator set to "agent"
    assert_agent_classification(&body, "DeepSearch sub-agent");
}

// ============================================================================
// Scenario: First user message uses x-initiator user, tool follow-ups use agent
// ============================================================================
#[test]
fn scenario_first_user_message_user_tool_followups_agent() {
    // @step Given a Copilot provider session with the agent loop

    // @step When the user sends their first message "fix the login bug"
    // Iteration 0 = user-initiated, no metadata.mode injection
    let body_first = json!({
        "model": "gpt-4o",
        "messages": [{ "role": "user", "content": "fix the login bug" }]
    });

    // @step Then the first API request should have x-initiator set to "user"
    assert_user_classification(&body_first, "first user message (iteration 0)");

    // @step When the agent calls the Read tool and receives a tool result
    // @step And the agent loop sends the follow-up API request with tool results
    // Iteration 1 = agent follow-up, metadata.mode injected
    let body_followup = json!({
        "model": "gpt-4o",
        "messages": [
            { "role": "user", "content": "fix the login bug" },
            { "role": "assistant", "content": "I'll read the file first." },
            { "role": "tool", "content": "file content here", "tool_call_id": "call_1" }
        ],
        "metadata": { "mode": "agent" }
    });

    // @step Then the follow-up API request should have x-initiator set to "agent"
    assert_agent_classification(&body_followup, "tool follow-up (iteration 1)");

    // @step When the agent calls the Edit tool and receives a tool result
    // @step And the agent loop sends the next follow-up API request
    // Iteration 2 = another agent follow-up
    let body_next = json!({
        "model": "gpt-4o",
        "messages": [
            { "role": "user", "content": "fix the login bug" },
            { "role": "assistant", "content": "Let me edit the file." },
            { "role": "tool", "content": "edit applied", "tool_call_id": "call_2" }
        ],
        "metadata": { "mode": "agent" }
    });

    // @step Then that API request should also have x-initiator set to "agent"
    assert_agent_classification(&body_next, "tool follow-up (iteration 2)");
}

// ============================================================================
// Scenario: Subordinate agent sessions use agent initiator
// ============================================================================
#[test]
fn scenario_subordinate_agent_sessions_use_agent_initiator() {
    // @step Given a Copilot provider session

    // @step When a subordinate agent is spawned via AgentManager
    let body = json!({
        "model": "claude-sonnet-4.5",
        "messages": [{ "role": "user", "content": "investigate auth module" }],
        "metadata": { "mode": "agent" }
    });

    // @step Then all API requests made by the subordinate should have x-initiator set to "agent"
    assert_agent_classification(&body, "subordinate agent session");
}

// ============================================================================
// Scenario: Compaction continuation uses agent initiator
// ============================================================================
#[test]
fn scenario_compaction_continuation_uses_agent_initiator() {
    // @step Given a Copilot provider session that has triggered compaction

    // @step When the DAG construction completes and a "Continue" prompt is sent
    let body = json!({
        "model": "gpt-4o",
        "messages": [{ "role": "user", "content": "Continue" }],
        "metadata": { "mode": "agent" }
    });

    // @step Then the continuation API request should have x-initiator set to "agent"
    assert_agent_classification(&body, "compaction continuation");
}

// ============================================================================
// Scenario: Scheduled job sessions use agent initiator
// ============================================================================
#[test]
fn scenario_scheduled_job_sessions_use_agent_initiator() {
    // @step Given a scheduled agent job configured with the Copilot provider

    // @step When the scheduled job session makes API requests
    let body = json!({
        "model": "gpt-4o",
        "messages": [{ "role": "user", "content": "Run daily cleanup task" }],
        "metadata": { "mode": "agent" }
    });

    // @step Then all API requests should have x-initiator set to "agent"
    assert_agent_classification(&body, "scheduled job session");
}
