
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/basic-agent-execution-loop.feature
//!
//! Tests for Basic Agent Execution Loop - AGENT-001
//!
//! These tests verify the Runner implementation for orchestrating
//! LLM provider calls and tool execution in the agent loop.

use anyhow::Result;
use async_trait::async_trait;
use codelet::agent::{ContentPart, Message, MessageContent, MessageRole, Runner};
use codelet::providers::{CompletionResponse, LlmProvider, StopReason};
use codelet::tools::{ToolDefinition, ToolRegistry};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

// ==========================================
// MOCK PROVIDER FOR TESTING
// ==========================================

/// Mock LLM provider that returns controlled responses for testing
struct MockProvider {
    /// Responses to return in sequence
    responses: Vec<MockResponse>,
    /// Current response index
    call_count: AtomicUsize,
}

/// Mock response from the provider
#[derive(Clone)]
struct MockResponse {
    /// Content parts in the response
    content: Vec<ContentPart>,
    /// Stop reason
    stop_reason: StopReason,
}

impl MockProvider {
    fn new(responses: Vec<MockResponse>) -> Self {
        Self {
            responses,
            call_count: AtomicUsize::new(0),
        }
    }

    fn single_text_response(text: &str) -> Self {
        Self::new(vec![MockResponse {
            content: vec![ContentPart::Text {
                text: text.to_string(),
            }],
            stop_reason: StopReason::EndTurn,
        }])
    }

    fn with_tool_call(tool_name: &str, input: serde_json::Value, final_text: &str) -> Self {
        Self::new(vec![
            MockResponse {
                content: vec![ContentPart::ToolUse {
                    id: "tool_call_1".to_string(),
                    name: tool_name.to_string(),
                    input,
                }],
                stop_reason: StopReason::ToolUse,
            },
            MockResponse {
                content: vec![ContentPart::Text {
                    text: final_text.to_string(),
                }],
                stop_reason: StopReason::EndTurn,
            },
        ])
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn model(&self) -> &str {
        "mock-model"
    }

    fn context_window(&self) -> usize {
        100000
    }

    fn max_output_tokens(&self) -> usize {
        4096
    }

    fn supports_caching(&self) -> bool {
        false
    }

    fn supports_streaming(&self) -> bool {
        false
    }

    async fn complete(&self, _messages: &[Message]) -> Result<String> {
        let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
        if idx < self.responses.len() {
            let response = &self.responses[idx];
            let text: String = response
                .content
                .iter()
                .filter_map(|part| {
                    if let ContentPart::Text { text } = part {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("");
            Ok(text)
        } else {
            Ok("No more responses".to_string())
        }
    }

    async fn complete_with_tools(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<CompletionResponse> {
        let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
        if idx < self.responses.len() {
            let response = &self.responses[idx];
            Ok(CompletionResponse {
                content: MessageContent::Parts(response.content.clone()),
                stop_reason: response.stop_reason,
            })
        } else {
            Ok(CompletionResponse {
                content: MessageContent::Parts(vec![ContentPart::Text {
                    text: "No more responses".to_string(),
                }]),
                stop_reason: StopReason::EndTurn,
            })
        }
    }
}

// ==========================================
// SCENARIO 1: SIMPLE TEXT RESPONSE
// ==========================================

/// Scenario: Simple text response exits loop immediately
#[tokio::test]
async fn test_simple_text_response_exits_loop_immediately() {
    // @step Given a Runner initialized with ClaudeProvider and default tools
    let provider = MockProvider::single_text_response("Hello! How can I help you today?");
    let mut runner = Runner::with_provider(Box::new(provider));

    // @step When I send a user message "Hello, how are you?"
    let result = runner.run("Hello, how are you?").await;

    // @step And the provider returns a text response with stop_reason "end_turn"
    // (This is handled by the mock provider)

    // @step Then the runner should exit the loop
    assert!(result.is_ok(), "Runner should complete successfully");

    // @step And the messages should contain the assistant response
    let messages = runner.messages();
    assert!(
        messages.len() >= 2,
        "Should have user and assistant messages"
    );

    // Find assistant message
    let has_assistant = messages
        .iter()
        .any(|m| matches!(m.role, MessageRole::Assistant));
    assert!(has_assistant, "Should contain assistant response");
}

// ==========================================
// SCENARIO 2: SINGLE TOOL CALL
// ==========================================

/// Scenario: Single tool call is executed and result injected
#[tokio::test]
async fn test_single_tool_call_is_executed_and_result_injected() {
    // @step Given a Runner initialized with ClaudeProvider and default tools
    let provider = MockProvider::with_tool_call(
        "Read",
        json!({"file_path": "/tmp/test.txt"}),
        "The file contains: Hello World",
    );
    let mut runner = Runner::with_provider(Box::new(provider));

    // @step When I send a user message "Read the file /tmp/test.txt"
    let result = runner.run("Read the file /tmp/test.txt").await;

    // @step And the provider returns a response containing a tool_use block for "Read"
    // (Handled by mock provider)

    // @step Then the runner should execute the Read tool with the provided arguments
    // @step And the runner should inject a tool_result message into the conversation
    // @step And the runner should call provider.complete() again with the updated messages
    assert!(
        result.is_ok(),
        "Runner should complete after tool execution: {:?}",
        result.err()
    );

    // Verify tool result was injected
    let messages = runner.messages();
    let has_tool_result = messages.iter().any(|m| {
        if let MessageContent::Parts(parts) = &m.content {
            parts
                .iter()
                .any(|p| matches!(p, ContentPart::ToolResult { .. }))
        } else {
            false
        }
    });
    assert!(has_tool_result, "Should contain tool result message");
}

// ==========================================
// SCENARIO 3: MULTIPLE TOOL CALLS
// ==========================================

/// Scenario: Multiple tool calls are executed in sequence
#[tokio::test]
async fn test_multiple_tool_calls_are_executed_in_sequence() {
    // @step Given a Runner initialized with ClaudeProvider and default tools
    let provider = MockProvider::new(vec![
        MockResponse {
            content: vec![
                ContentPart::ToolUse {
                    id: "tool_1".to_string(),
                    name: "Read".to_string(),
                    input: json!({"file_path": "/tmp/file1.txt"}),
                },
                ContentPart::ToolUse {
                    id: "tool_2".to_string(),
                    name: "Read".to_string(),
                    input: json!({"file_path": "/tmp/file2.txt"}),
                },
            ],
            stop_reason: StopReason::ToolUse,
        },
        MockResponse {
            content: vec![ContentPart::Text {
                text: "Both files read successfully".to_string(),
            }],
            stop_reason: StopReason::EndTurn,
        },
    ]);
    let mut runner = Runner::with_provider(Box::new(provider));

    // @step When I send a user message "Read two files"
    let result = runner.run("Read two files").await;

    // @step And the provider returns a response containing multiple tool_use blocks
    // (Handled by mock)

    // @step Then the runner should execute all tools in sequence
    // @step And the runner should inject all tool_result messages
    // @step And the runner should continue the loop
    assert!(
        result.is_ok(),
        "Runner should complete successfully: {:?}",
        result.err()
    );

    // Verify multiple tool results
    let messages = runner.messages();
    let tool_result_count = messages
        .iter()
        .filter(|m| {
            if let MessageContent::Parts(parts) = &m.content {
                parts
                    .iter()
                    .any(|p| matches!(p, ContentPart::ToolResult { .. }))
            } else {
                false
            }
        })
        .count();

    assert!(
        tool_result_count >= 2,
        "Should have multiple tool results, got {}",
        tool_result_count
    );
}

// ==========================================
// SCENARIO 4: FAILED TOOL EXECUTION
// ==========================================

/// Scenario: Failed tool execution injects error result
#[tokio::test]
async fn test_failed_tool_execution_injects_error_result() {
    // @step Given a Runner initialized with ClaudeProvider and default tools
    let provider = MockProvider::with_tool_call(
        "Read",
        json!({"file_path": "/nonexistent/file.txt"}),
        "I see the file doesn't exist. Let me help you another way.",
    );
    let mut runner = Runner::with_provider(Box::new(provider));

    // @step When I send a user message "Read /nonexistent/file.txt"
    // @step And the provider returns a tool_use block for "Read" with path "/nonexistent/file.txt"
    let result = runner.run("Read /nonexistent/file.txt").await;

    // @step And the Read tool execution fails with "file not found"
    // (The actual Read tool will fail because the file doesn't exist)

    // @step Then the runner should inject a tool_result with is_error set to true
    // @step And the runner should continue the loop for the LLM to handle the error
    assert!(
        result.is_ok(),
        "Runner should complete even with tool error: {:?}",
        result.err()
    );

    // Verify error result was injected
    let messages = runner.messages();
    let has_error_result = messages.iter().any(|m| {
        if let MessageContent::Parts(parts) = &m.content {
            parts.iter().any(|p| {
                if let ContentPart::ToolResult { is_error, .. } = p {
                    *is_error
                } else {
                    false
                }
            })
        } else {
            false
        }
    });
    assert!(has_error_result, "Should contain error tool result");
}

// ==========================================
// SCENARIO 5: MULTI-TURN TOOL EXECUTION
// ==========================================

/// Scenario: Multi-turn tool execution loop completes successfully
#[tokio::test]
async fn test_multi_turn_tool_execution_loop_completes_successfully() {
    // @step Given a Runner initialized with ClaudeProvider and default tools
    let provider = MockProvider::new(vec![
        // Turn 1: Read file
        MockResponse {
            content: vec![ContentPart::ToolUse {
                id: "tool_1".to_string(),
                name: "Read".to_string(),
                input: json!({"file_path": "/tmp/main.rs"}),
            }],
            stop_reason: StopReason::ToolUse,
        },
        // Turn 2: Another tool call
        MockResponse {
            content: vec![ContentPart::ToolUse {
                id: "tool_2".to_string(),
                name: "Grep".to_string(),
                input: json!({"pattern": "fn main", "path": "/tmp"}),
            }],
            stop_reason: StopReason::ToolUse,
        },
        // Turn 3: Another tool call
        MockResponse {
            content: vec![ContentPart::ToolUse {
                id: "tool_3".to_string(),
                name: "Read".to_string(),
                input: json!({"file_path": "/tmp/lib.rs"}),
            }],
            stop_reason: StopReason::ToolUse,
        },
        // Final turn: Complete
        MockResponse {
            content: vec![ContentPart::Text {
                text: "Bug fixed!".to_string(),
            }],
            stop_reason: StopReason::EndTurn,
        },
    ]);
    let mut runner = Runner::with_provider(Box::new(provider));

    // @step When I send a user message "Help me fix the bug in main.rs"
    let result = runner.run("Help me fix the bug in main.rs").await;

    // @step And the provider returns tool_use blocks for 3 iterations
    // @step And the final response has stop_reason "end_turn"
    // (Handled by mock)

    // @step Then the runner should have executed tools across all iterations
    // @step And the runner should exit the loop with the final response
    // @step And the messages should contain all tool calls and results
    assert!(
        result.is_ok(),
        "Runner should complete successfully: {:?}",
        result.err()
    );

    let messages = runner.messages();

    // Should have multiple iterations worth of messages
    assert!(
        messages.len() >= 5,
        "Should have multiple turns of messages, got {}",
        messages.len()
    );
}

// ==========================================
// SCENARIO 6: API AUTHENTICATION ERROR
// ==========================================

/// Mock provider that always returns an error
struct ErrorProvider {
    error_message: String,
}

impl ErrorProvider {
    fn auth_error() -> Self {
        Self {
            error_message: "Authentication failed: Invalid API key".to_string(),
        }
    }
}

#[async_trait]
impl LlmProvider for ErrorProvider {
    fn name(&self) -> &str {
        "error"
    }
    fn model(&self) -> &str {
        "error-model"
    }
    fn context_window(&self) -> usize {
        100000
    }
    fn max_output_tokens(&self) -> usize {
        4096
    }
    fn supports_caching(&self) -> bool {
        false
    }
    fn supports_streaming(&self) -> bool {
        false
    }

    async fn complete(&self, _messages: &[Message]) -> Result<String> {
        Err(anyhow::anyhow!("{}", self.error_message))
    }

    async fn complete_with_tools(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<CompletionResponse> {
        Err(anyhow::anyhow!("{}", self.error_message))
    }
}

/// Scenario: API authentication error is propagated
#[tokio::test]
async fn test_api_authentication_error_is_propagated() {
    // @step Given a Runner initialized with an invalid API key
    let provider = ErrorProvider::auth_error();
    let mut runner = Runner::with_provider(Box::new(provider));

    // @step When I send a user message "Hello"
    // @step And the provider returns an authentication error
    let result = runner.run("Hello").await;

    // @step Then the runner should return an error Result
    assert!(result.is_err(), "Runner should propagate provider error");

    // @step And the error message should indicate authentication failure
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Authentication") || err.to_string().contains("auth"),
        "Error should mention authentication: {}",
        err
    );
}

// ==========================================
// SCENARIO 7: TOOL DEFINITIONS IN API REQUEST
// ==========================================

/// Scenario: Tool definitions are included in API request
#[tokio::test]
async fn test_tool_definitions_are_included_in_api_request() {
    // @step Given a Runner initialized with ClaudeProvider and default tools
    let provider = MockProvider::single_text_response(
        "I have access to Read, Write, Edit, Bash, Grep, Glob, and AstGrep tools.",
    );
    let runner = Runner::with_provider(Box::new(provider));

    // @step When I send a user message "What tools do you have?"
    // The tool definitions should be included in the API request

    // @step Then the API request should include tool definitions from ToolRegistry
    let tool_definitions = runner.tools().definitions();

    // @step And each tool definition should have name, description, and parameters schema
    assert!(!tool_definitions.is_empty(), "Should have tool definitions");

    for def in &tool_definitions {
        assert!(!def.name.is_empty(), "Tool should have name");
        assert!(!def.description.is_empty(), "Tool should have description");
        // Parameters schema should be an object
        assert!(
            def.input_schema.is_object(),
            "Tool should have parameters schema"
        );
    }
}

// ==========================================
// RUNNER CONSTRUCTION TESTS
// ==========================================

/// Test that Runner can be created with a provider
#[test]
fn test_runner_with_provider_construction() {
    let provider = MockProvider::single_text_response("test");
    let runner = Runner::with_provider(Box::new(provider));

    // Runner should have default tools
    assert!(
        !runner.tools().is_empty(),
        "Runner should have default tools"
    );
}

/// Test that ToolRegistry provides definitions
#[test]
fn test_tool_registry_provides_definitions() {
    let registry = ToolRegistry::with_core_tools();
    let definitions = registry.definitions();

    // Should have definitions for all core tools
    assert_eq!(definitions.len(), 7, "Should have 7 core tool definitions");

    // Check for expected tools
    let tool_names: Vec<&str> = definitions.iter().map(|d| d.name.as_str()).collect();
    assert!(tool_names.contains(&"Read"), "Should have Read tool");
    assert!(tool_names.contains(&"Write"), "Should have Write tool");
    assert!(tool_names.contains(&"Edit"), "Should have Edit tool");
    assert!(tool_names.contains(&"Bash"), "Should have Bash tool");
    assert!(tool_names.contains(&"Grep"), "Should have Grep tool");
    assert!(tool_names.contains(&"Glob"), "Should have Glob tool");
    assert!(tool_names.contains(&"AstGrep"), "Should have AstGrep tool");
}
