#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Tests for prompt too long error detection and recovery
//!
//! Tests the is_prompt_too_long_error helper and related compaction recovery logic.

use codelet_cli::compaction_threshold::calculate_usable_context;

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

/// Helper function copied from stream_loop.rs for testing
fn is_prompt_too_long_error(error_str: &str) -> bool {
    let error_lower = error_str.to_lowercase();
    error_lower.contains("prompt is too long")
        || error_lower.contains("maximum context length")
        || error_lower.contains("context_length_exceeded")
        || error_lower.contains("too many tokens")
        || error_lower.contains("exceeds the model")
        || (error_lower.contains("invalid_request_error")
            && (error_lower.contains("token") || error_lower.contains("maximum")))
}

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
    let existing_messages: Vec<Message> = (0..10).map(|i| {
        Message::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: format!("Message number {i} with some content"),
            })),
        }
    }).collect();
    
    // Add a tool result that pushes over threshold
    // 50k chars ≈ 12.5k tokens (at ~4 chars/token)
    let large_file_content = "fn main() { println!(\"test\"); }\n".repeat(1500);
    let tool_result_message = Message::User {
        content: OneOrMany::one(UserContent::ToolResult(ToolResult {
            id: "call_1".to_string(),
            call_id: None,
            content: OneOrMany::one(ToolResultContent::Text(Text { 
                text: large_file_content 
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
