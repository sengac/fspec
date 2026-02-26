#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::uninlined_format_args)]
//! Tests for prompt too long error detection and recovery
//!
//! Tests the is_prompt_too_long_error helper and related compaction recovery logic.
//!
//! Feature: spec/features/prompt-too-long-error-detection.feature
//!
//! IMPORTANT: These tests import the REAL production function. Do NOT copy the function
//! here - always test the actual production code.

use codelet_cli::compaction_threshold::calculate_usable_context;
use codelet_cli::interactive::is_prompt_too_long_error;
use codelet_cli::interactive_helpers::convert_messages_to_turns;

// =============================================================================
// Legacy tests - these were here before PROV-010 and remain for regression coverage
// =============================================================================

/// Test detection of various "prompt too long" error formats from different providers
#[test]
fn test_is_prompt_too_long_error_anthropic() {
    // Anthropic/Claude format from the screenshot
    let error = r#"{"type":"invalid_request_error","message":"prompt is too long: 209834 tokens > 200000 maximum"}"#;
    assert!(is_prompt_too_long_error(error));
}

#[test]
fn test_is_prompt_too_long_error_openai() {
    // OpenAI format
    let error = "This model's maximum context length is 128000 tokens";
    assert!(is_prompt_too_long_error(error));

    let error2 = "context_length_exceeded: Request too large";
    assert!(is_prompt_too_long_error(error2));
}

#[test]
fn test_is_prompt_too_long_error_generic() {
    let error = "Request has too many tokens";
    assert!(is_prompt_too_long_error(error));

    let error2 = "Input exceeds the model maximum";
    assert!(is_prompt_too_long_error(error2));
}

#[test]
fn test_is_prompt_too_long_error_false_positives() {
    // Should NOT match normal errors
    let error = "Network timeout";
    assert!(!is_prompt_too_long_error(error));

    let error2 = "Authentication failed";
    assert!(!is_prompt_too_long_error(error2));

    let error3 = "Rate limit exceeded";
    assert!(!is_prompt_too_long_error(error3));
}

#[test]
fn test_is_prompt_too_long_error_case_insensitive() {
    let error = "PROMPT IS TOO LONG";
    assert!(is_prompt_too_long_error(error));

    let error2 = "Maximum Context Length exceeded";
    assert!(is_prompt_too_long_error(error2));
}

// =============================================================================
// PROV-010: False positive prompt-too-long detection triggers empty compaction
// Feature: spec/features/prompt-too-long-error-detection.feature
// =============================================================================

/// Scenario: Thinking budget configuration error is not classified as prompt-too-long
#[test]
fn test_prov010_thinking_budget_config_error_not_classified_as_prompt_too_long() {
    // @step Given an error message containing "invalid_request_error"
    // @step And the error message contains "max_tokens must be greater than thinking.budget_tokens"
    let error = r#"{"type":"error","error":{"type":"invalid_request_error","message":"`max_tokens` must be greater than `thinking.budget_tokens`"}}"#;

    // @step When the error is checked by is_prompt_too_long_error
    let result = is_prompt_too_long_error(error);

    // @step Then the function should return false
    // @step And the error should NOT trigger compaction
    assert!(
        !result,
        "Thinking budget config error should NOT be classified as prompt-too-long. Error: {}",
        error
    );
}

/// Scenario: Generic budget_tokens configuration error is not classified as prompt-too-long
#[test]
fn test_prov010_generic_budget_tokens_error_not_classified_as_prompt_too_long() {
    // @step Given an error message containing "invalid_request_error"
    // @step And the error message contains "budget_tokens"
    let error = r#"{"type":"invalid_request_error","message":"budget_tokens must be positive"}"#;

    // @step When the error is checked by is_prompt_too_long_error
    let result = is_prompt_too_long_error(error);

    // @step Then the function should return false
    assert!(
        !result,
        "Any budget_tokens config error should NOT be classified as prompt-too-long. Error: {}",
        error
    );
}

/// Scenario: Actual prompt too long error is correctly detected
#[test]
fn test_prov010_actual_prompt_too_long_still_detected() {
    // @step Given an error message "prompt is too long"
    let error = "prompt is too long: 209834 tokens > 200000 maximum";

    // @step When the error is checked by is_prompt_too_long_error
    let result = is_prompt_too_long_error(error);

    // @step Then the function should return true
    assert!(
        result,
        "Actual 'prompt is too long' error should be detected. Error: {}",
        error
    );
}

/// Scenario: Context length exceeded error is correctly detected
#[test]
fn test_prov010_context_length_exceeded_still_detected() {
    // @step Given an error message "context_length_exceeded"
    let error = "context_length_exceeded: Request too large";

    // @step When the error is checked by is_prompt_too_long_error
    let result = is_prompt_too_long_error(error);

    // @step Then the function should return true
    assert!(
        result,
        "context_length_exceeded error should be detected. Error: {}",
        error
    );
}

/// Scenario: Maximum context length error is correctly detected
#[test]
fn test_prov010_maximum_context_length_still_detected() {
    // @step Given an error message "maximum context length"
    let error = "This model's maximum context length is 128000 tokens";

    // @step When the error is checked by is_prompt_too_long_error
    let result = is_prompt_too_long_error(error);

    // @step Then the function should return true
    assert!(
        result,
        "maximum context length error should be detected. Error: {}",
        error
    );
}

/// Scenario: Too many tokens error is correctly detected
#[test]
fn test_prov010_too_many_tokens_still_detected() {
    // @step Given an error message "too many tokens"
    let error = "Request has too many tokens";

    // @step When the error is checked by is_prompt_too_long_error
    let result = is_prompt_too_long_error(error);

    // @step Then the function should return true
    assert!(
        result,
        "too many tokens error should be detected. Error: {}",
        error
    );
}

/// Scenario: Invalid request error with maximum tokens is correctly detected
#[test]
fn test_prov010_invalid_request_with_maximum_tokens_detected() {
    // @step Given an error message containing "invalid_request_error"
    // @step And the error message contains "maximum"
    // @step And the error message does NOT contain "budget_tokens"
    let error = r#"{"type":"invalid_request_error","message":"maximum token limit exceeded"}"#;

    // @step When the error is checked by is_prompt_too_long_error
    let result = is_prompt_too_long_error(error);

    // @step Then the function should return true
    assert!(
        result,
        "invalid_request_error with 'maximum' (no budget_tokens) should be detected. Error: {}",
        error
    );
}

/// Scenario: Error message with both budget_tokens and context_length is NOT classified as prompt-too-long
#[test]
fn test_prov010_budget_tokens_exclusion_takes_precedence() {
    // @step Given an error message containing "invalid_request_error"
    // @step And the error message contains "budget_tokens"
    // @step And the error message contains "context_length"
    let error =
        r#"{"type":"invalid_request_error","message":"budget_tokens exceeds context_length"}"#;

    // @step When the error is checked by is_prompt_too_long_error
    let result = is_prompt_too_long_error(error);

    // @step Then the function should return false
    // budget_tokens exclusion takes precedence
    assert!(
        !result,
        "budget_tokens exclusion should take precedence even with context_length present. Error: {}",
        error
    );
}

// =============================================================================
// PROV-010: Integration Tests - Error Handler Behavior
// These tests validate the LOGIC that the error handler uses, using the same
// functions as production code (convert_messages_to_turns, is_prompt_too_long_error)
// =============================================================================

/// Scenario: Prompt too long with zero conversation turns does not trigger compaction
///
/// This tests Bug 2: The guard for empty turn history
#[test]
fn test_prov010_prompt_too_long_zero_turns_no_compaction() {
    use rig::message::Message;

    // @step Given a session with only system prompt messages
    let messages: Vec<Message> = vec![
        // System message (would be added via system reminders, not in messages vec typically)
        // Simulating system-only session by having no user/assistant turns
    ];

    // @step And the session has zero user/assistant conversation turns
    // Use the SAME function as production code: convert_messages_to_turns
    let has_compactable_turns = !convert_messages_to_turns(&messages).is_empty();
    assert!(
        !has_compactable_turns,
        "Should have zero compactable turns"
    );

    // @step When an API error "prompt is too long" is received
    let error = "prompt is too long: 209834 tokens > 200000 maximum";
    let is_prompt_too_long = is_prompt_too_long_error(error);

    // @step Then the error should be classified as prompt-too-long
    assert!(
        is_prompt_too_long,
        "Should be classified as prompt-too-long"
    );

    // @step But compaction should NOT be triggered
    // @step And the error should propagate to the user
    // This mirrors the EXACT logic in stream_loop.rs:1185
    let should_trigger_compaction = is_prompt_too_long && has_compactable_turns;
    assert!(
        !should_trigger_compaction,
        "Compaction should NOT be triggered when there are no compactable turns"
    );
}

/// Scenario: Prompt too long with conversation turns triggers compaction
#[test]
fn test_prov010_prompt_too_long_with_turns_triggers_compaction() {
    use rig::message::{AssistantContent, Message, Text, UserContent};
    use rig::OneOrMany;

    // @step Given a session with system prompt messages
    // @step And the session has 5 user/assistant conversation turns
    let mut messages: Vec<Message> = Vec::new();
    for i in 0..5 {
        messages.push(Message::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: format!("User message {i}"),
            })),
        });
        messages.push(Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::Text(Text {
                text: format!("Assistant response {i}"),
            })),
        });
    }

    // Use the SAME function as production code: convert_messages_to_turns
    let turns = convert_messages_to_turns(&messages);
    assert_eq!(turns.len(), 5, "Should have 5 compactable turns");

    // @step When an API error "context_length_exceeded" is received
    let error = "context_length_exceeded: Request too large";
    let is_prompt_too_long = is_prompt_too_long_error(error);

    // @step Then the error should be classified as prompt-too-long
    assert!(
        is_prompt_too_long,
        "Should be classified as prompt-too-long"
    );

    // @step And compaction should be triggered
    // @step And the context should be reduced
    // This mirrors the EXACT logic in stream_loop.rs:1185
    let has_compactable_turns = !turns.is_empty();
    let should_trigger_compaction = is_prompt_too_long && has_compactable_turns;
    assert!(
        should_trigger_compaction,
        "Compaction SHOULD be triggered when there are compactable turns"
    );
}

/// Scenario: Configuration error propagates to user with clear message
#[test]
fn test_prov010_config_error_propagates_to_user() {
    use rig::message::{AssistantContent, Message, Text, UserContent};
    use rig::OneOrMany;

    // @step Given a session with any number of conversation turns
    let messages: Vec<Message> = vec![
        Message::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: "Hello".to_string(),
            })),
        },
        Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::Text(Text {
                text: "Hi there!".to_string(),
            })),
        },
    ];

    // Use the SAME function as production code: convert_messages_to_turns
    let has_turns = !convert_messages_to_turns(&messages).is_empty();
    assert!(has_turns, "Should have conversation turns");

    // @step When an API error "max_tokens must be greater than thinking.budget_tokens" is received
    let error = r#"{"type":"error","error":{"type":"invalid_request_error","message":"`max_tokens` must be greater than `thinking.budget_tokens`"}}"#;

    // @step Then the error should NOT be classified as prompt-too-long
    let is_prompt_too_long = is_prompt_too_long_error(error);
    assert!(
        !is_prompt_too_long,
        "Config error should NOT be classified as prompt-too-long"
    );

    // @step And the error should propagate to the user with the original message
    // Since it's not classified as prompt-too-long, compaction won't be triggered,
    // and the error will propagate to the user
    // This mirrors the EXACT logic in stream_loop.rs:1185
    let should_trigger_compaction = is_prompt_too_long && has_turns;
    assert!(
        !should_trigger_compaction,
        "Compaction should NOT be triggered, error should propagate"
    );
}

// =============================================================================
// Additional Tests - Compaction Infrastructure
// =============================================================================

/// Test that compaction threshold calculation works correctly
#[test]
fn test_compaction_threshold_for_claude() {
    // Claude has 200k context window, 8192 max output
    let context_window = 200_000;
    let max_output = 8_192;
    let usable = calculate_usable_context(context_window, max_output);

    // Should be 200,000 - 8,192 = 191,808
    assert_eq!(usable, 191_808);
}

/// Test scenario: payload estimation catches overflow before API call
#[test]
fn test_payload_estimation_prevents_overflow() {
    use codelet_core::estimate_messages_tokens;
    use rig::message::{Message, Text, ToolResult, ToolResultContent, UserContent};
    use rig::OneOrMany;

    let threshold: u64 = 10_000; // Use smaller threshold for faster test

    // Simulate a session with existing messages
    let existing_messages: Vec<Message> = (0..10)
        .map(|i| Message::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: format!("Message number {i} with some content"),
            })),
        })
        .collect();

    // Add a tool result that pushes over threshold
    // 50k chars ≈ 12.5k tokens (at ~4 chars/token)
    let large_file_content = "fn main() { println!(\"test\"); }\n".repeat(1500);
    let tool_result_message = Message::User {
        content: OneOrMany::one(UserContent::ToolResult(ToolResult {
            id: "call_1".to_string(),
            call_id: None,
            content: OneOrMany::one(ToolResultContent::Text(Text {
                text: large_file_content,
            })),
        })),
    };

    let mut all_messages = existing_messages;
    all_messages.push(tool_result_message);

    let estimated = estimate_messages_tokens(&all_messages) as u64;

    // The estimated payload should exceed the threshold
    assert!(
        estimated > threshold,
        "Large tool result should push payload over threshold: {estimated} > {threshold}"
    );
}
