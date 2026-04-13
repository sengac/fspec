#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for `copilot::prompt_cache` — PROV-058 prompt cache control injection.
//!
//! Feature: spec/features/copilot-prompt-caching.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Scenarios map directly to Gherkin scenarios.

use super::*;
use serde_json::json;

const CACHE_CONTROL: &str = "copilot_cache_control";

// -----------------------------------------------------------------
// Scenario: Claude model multi-turn conversation gets cache control
//           on system, last tool, and last assistant message
// -----------------------------------------------------------------
#[test]
fn claude_multi_turn_gets_cache_control_on_system_last_tool_and_last_assistant() {
    // @step Given a Copilot API request body with model "claude-sonnet-4"
    // @step And the request has a system message, 3 conversation turns, and 5 tool definitions
    let mut body = json!({
        "model": "claude-sonnet-4",
        "messages": [
            { "role": "system", "content": "You are a helpful assistant." },
            { "role": "user", "content": "Hello" },
            { "role": "assistant", "content": "Hi there!" },
            { "role": "user", "content": "What is 2+2?" },
            { "role": "assistant", "content": "4" },
            { "role": "user", "content": "Thanks!" }
        ],
        "tools": [
            { "type": "function", "function": { "name": "read", "description": "Read a file" } },
            { "type": "function", "function": { "name": "write", "description": "Write a file" } },
            { "type": "function", "function": { "name": "edit", "description": "Edit a file" } },
            { "type": "function", "function": { "name": "bash", "description": "Run command" } },
            { "type": "function", "function": { "name": "grep", "description": "Search files" } }
        ]
    });

    // @step When the CopilotHttpClient middleware processes the request
    inject_cache_control(&mut body);

    let messages = body["messages"].as_array().expect("messages must be an array");
    let tools = body["tools"].as_array().expect("tools must be an array");

    // @step Then the system message should have copilot_cache_control set to ephemeral
    assert_eq!(
        messages[0].get(CACHE_CONTROL),
        Some(&json!({ "type": "ephemeral" })),
        "system message must have copilot_cache_control"
    );

    // @step And the last tool definition should have copilot_cache_control set to ephemeral
    assert_eq!(
        tools[4].get(CACHE_CONTROL),
        Some(&json!({ "type": "ephemeral" })),
        "last tool (grep) must have copilot_cache_control"
    );

    // @step And the last assistant message before the final user turn should have copilot_cache_control set to ephemeral
    assert_eq!(
        messages[4].get(CACHE_CONTROL),
        Some(&json!({ "type": "ephemeral" })),
        "last assistant message (index 4) must have copilot_cache_control"
    );

    // @step And no other messages or tools should have copilot_cache_control
    assert!(messages[1].get(CACHE_CONTROL).is_none(), "user msg 1 must not have cache control");
    assert!(messages[2].get(CACHE_CONTROL).is_none(), "assistant msg 1 must not have cache control");
    assert!(messages[3].get(CACHE_CONTROL).is_none(), "user msg 2 must not have cache control");
    assert!(messages[5].get(CACHE_CONTROL).is_none(), "final user msg must not have cache control");
    for (i, tool) in tools.iter().enumerate().take(4) {
        assert!(
            tool.get(CACHE_CONTROL).is_none(),
            "tool {i} must not have cache control"
        );
    }
}

// -----------------------------------------------------------------
// Scenario: GPT model requests are not modified with cache control
// -----------------------------------------------------------------
#[test]
fn gpt_model_requests_not_modified() {
    // @step Given a Copilot API request body with model "gpt-5"
    // @step And the request has a system message, 3 conversation turns, and 5 tool definitions
    let mut body = json!({
        "model": "gpt-5",
        "messages": [
            { "role": "system", "content": "You are helpful." },
            { "role": "user", "content": "Hello" },
            { "role": "assistant", "content": "Hi!" },
            { "role": "user", "content": "Bye" }
        ],
        "tools": [
            { "type": "function", "function": { "name": "read", "description": "Read" } },
            { "type": "function", "function": { "name": "write", "description": "Write" } }
        ]
    });

    let original = body.clone();

    // @step When the CopilotHttpClient middleware processes the request
    inject_cache_control(&mut body);

    // @step Then no messages should have copilot_cache_control
    // @step And no tools should have copilot_cache_control
    assert_eq!(body, original, "GPT body must not be modified");
}

// -----------------------------------------------------------------
// Scenario: Gemini model requests are not modified with cache control
// -----------------------------------------------------------------
#[test]
fn gemini_model_requests_not_modified() {
    // @step Given a Copilot API request body with model "gemini-2.5-pro"
    // @step And the request has a system message and 2 conversation turns
    let mut body = json!({
        "model": "gemini-2.5-pro",
        "messages": [
            { "role": "system", "content": "You are helpful." },
            { "role": "user", "content": "Hello" },
            { "role": "assistant", "content": "Hi!" },
            { "role": "user", "content": "Bye" }
        ]
    });

    let original = body.clone();

    // @step When the CopilotHttpClient middleware processes the request
    inject_cache_control(&mut body);

    // @step Then no messages should have copilot_cache_control
    assert_eq!(body, original, "Gemini body must not be modified");
}

// -----------------------------------------------------------------
// Scenario: Single-turn Claude conversation only caches system message
// -----------------------------------------------------------------
#[test]
fn single_turn_claude_only_caches_system() {
    // @step Given a Copilot API request body with model "claude-sonnet-4.5"
    // @step And the request has only a system message and one user message
    let mut body = json!({
        "model": "claude-sonnet-4.5",
        "messages": [
            { "role": "system", "content": "You are a helpful assistant." },
            { "role": "user", "content": "Hello!" }
        ],
        "tools": [
            { "type": "function", "function": { "name": "read", "description": "Read" } }
        ]
    });

    // @step When the CopilotHttpClient middleware processes the request
    inject_cache_control(&mut body);

    let messages = body["messages"].as_array().expect("messages must be an array");

    // @step Then the system message should have copilot_cache_control set to ephemeral
    assert_eq!(
        messages[0].get(CACHE_CONTROL),
        Some(&json!({ "type": "ephemeral" })),
        "system message must have copilot_cache_control"
    );

    // @step And the user message should not have copilot_cache_control
    assert!(
        messages[1].get(CACHE_CONTROL).is_none(),
        "user message must not have cache control"
    );
}

// -----------------------------------------------------------------
// Scenario: Claude request with empty tools array does not crash
// -----------------------------------------------------------------
#[test]
fn claude_empty_tools_no_crash() {
    // @step Given a Copilot API request body with model "claude-opus-4.5"
    // @step And the request has a system message, 2 conversation turns, and no tools
    let mut body = json!({
        "model": "claude-opus-4.5",
        "messages": [
            { "role": "system", "content": "You are helpful." },
            { "role": "user", "content": "Hello" },
            { "role": "assistant", "content": "Hi!" },
            { "role": "user", "content": "What is 2+2?" }
        ],
        "tools": []
    });

    // @step When the CopilotHttpClient middleware processes the request
    inject_cache_control(&mut body);

    let messages = body["messages"].as_array().expect("messages must be an array");

    // @step Then the system message should have copilot_cache_control set to ephemeral
    assert_eq!(
        messages[0].get(CACHE_CONTROL),
        Some(&json!({ "type": "ephemeral" }))
    );

    // @step And the last assistant message should have copilot_cache_control set to ephemeral
    assert_eq!(
        messages[2].get(CACHE_CONTROL),
        Some(&json!({ "type": "ephemeral" }))
    );

    // @step And no error should occur from the empty tools array
    assert!(
        body["tools"].as_array().expect("tools must be an array").is_empty(),
        "tools array should remain empty without error"
    );
}

// -----------------------------------------------------------------
// Additional edge cases
// -----------------------------------------------------------------
#[test]
fn missing_model_field_is_noop() {
    let mut body = json!({
        "messages": [{ "role": "system", "content": "test" }]
    });
    let original = body.clone();
    inject_cache_control(&mut body);
    assert_eq!(body, original);
}
