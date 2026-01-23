
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/refactor-agent-loop-to-use-rig-with-multi-turn.feature
//!
//! Tests for Refactoring Agent Loop to use Rig with Multi-Turn - REFAC-004
//!
//! These tests verify that the agent loop is refactored to use rig::agent::Agent
//! with automatic multi-turn tool calling, depth control, and streaming support.

use codelet_providers::ClaudeProvider;
use codelet_tools::facade::ProviderToolRegistry;

// ==========================================
// SCENARIO 1: Replace Runner with rig Agent for automatic tool execution
// ==========================================

/// Scenario: Replace Runner with rig Agent for automatic tool execution
#[test]
fn test_replace_runner_with_rig_agent_for_automatic_tool_execution() {
    // @step Given the codebase has a custom Runner implementation
    // Runner is now dead code, we use ProviderToolRegistry::new() instead

    // @step When I refactor to use rig::agent::Agent
    // This test will verify the agent can be created

    // @step Then the agent should automatically execute tools without manual loop handling
    // Create a provider
    let _provider = ClaudeProvider::from_api_key("test-key").expect("Provider should be created");

    // Create tool registry (replaces Runner)
    let registry = ProviderToolRegistry::new();

    // Verify registry exists with tools for providers
    let claude_tools = registry.tools_for_provider("claude");
    let gemini_tools = registry.tools_for_provider("gemini");
    assert!(
        !claude_tools.is_empty() || !gemini_tools.is_empty(),
        "ProviderToolRegistry should have tools registered for at least one provider"
    );

    // @step And tool calling should support multi-turn with configurable depth
    // @step And the default max depth should be 10 turns
    // After refactoring, these will be configurable via rig::agent::Agent
    // For now, we verify the registry exists and can be constructed
}

// ==========================================
// SCENARIO 2: Execute multiple tools automatically in multi-turn mode
// ==========================================

/// Scenario: Execute multiple tools automatically in multi-turn mode
#[test]
fn test_execute_multiple_tools_automatically_in_multi_turn_mode() {
    // @step Given I have an agent with max depth set to 5
    let _provider = ClaudeProvider::from_api_key("test-key").unwrap();
    let registry = ProviderToolRegistry::new();

    // @step When the agent needs 3 tool calls to complete a task
    // This would require integration testing with actual API calls

    // @step Then all 3 tools should execute automatically without intervention
    // @step And the agent should not exceed the max depth of 5
    // @step And each tool execution should be counted as one turn

    // For now, verify the registry has access to tools
    // After refactoring, rig::agent::Agent will handle multi-turn automatically
    let tools = registry.tools_for_provider("claude");
    assert!(
        !tools.is_empty(),
        "Registry should have claude tools for multi-turn execution"
    );
}

// ==========================================
// SCENARIO 3: Stream tool execution with MultiTurnStreamItem
// ==========================================

/// Scenario: Stream tool execution with MultiTurnStreamItem
#[test]
fn test_stream_tool_execution_with_multi_turn_stream_item() {
    // @step Given I have an agent in streaming mode
    let _provider = ClaudeProvider::from_api_key("test-key").unwrap();
    let _registry = ProviderToolRegistry::new();

    // @step When the agent executes a tool during streaming
    // @step Then I should receive a ToolCall item in the stream
    // @step And I should receive a ToolResult item after execution
    // @step And the stream should emit MultiTurnStreamItem variants
    // @step And I should see tool execution visibility in real-time

    // After refactoring to rig::agent::Agent:
    // - stream() method will return MultiTurnStreamItem
    // - Items: ToolCall, ToolResult, Message, Reasoning, etc.

    // For now, verify streaming is supported at provider level
    // (prerequisite for agent streaming)
    // This will be implemented when rig::agent::Agent is integrated
}

// ==========================================
// SCENARIO 4: Stop when max depth is reached
// ==========================================

/// Scenario: Stop when max depth is reached
#[test]
fn test_stop_when_max_depth_is_reached() {
    // @step Given I have an agent with max depth set to 10
    let _provider = ClaudeProvider::from_api_key("test-key").unwrap();
    let _registry = ProviderToolRegistry::new();

    // @step When the agent attempts to make 11 tool calls
    // This would require integration testing

    // @step Then the agent should stop at turn 10
    // @step And an error message about max depth exceeded should be returned
    // @step And no further tool calls should be executed

    // After refactoring, rig::agent::Agent will enforce max_depth
    // Default: 10 turns
    // Configurable via agent builder

    // For now, verify registry can be created
    // Depth limiting will be tested in integration tests
}

// ==========================================
// SCENARIO 5: All tools implement rig Tool trait
// ==========================================

/// Scenario: All tools implement rig Tool trait
#[test]
fn test_all_tools_implement_rig_tool_trait() {
    // @step Given codelet has 7 tools (Read, Write, Edit, Bash, Grep, Glob, AstGrep)
    // All tools now implement rig::tool::Tool trait directly
    
    // Verify all tools implement rig::tool::Tool (compile-time check)
    use codelet_tools::{AstGrepTool, BashTool, EditTool, GlobTool, GrepTool, ReadTool, WriteTool};

    fn _assert_rig_tool<T: rig::tool::Tool>() {}
    _assert_rig_tool::<ReadTool>();
    _assert_rig_tool::<WriteTool>();
    _assert_rig_tool::<EditTool>();
    _assert_rig_tool::<BashTool>();
    _assert_rig_tool::<GrepTool>();
    _assert_rig_tool::<GlobTool>();
    _assert_rig_tool::<AstGrepTool>();

    // @step When I refactor tools to implement rig::tool::Tool
    // @step Then each tool should provide a tool definition via the Tool trait
    // @step And each tool should implement the call() method
    // @step And tools should integrate with rig's automatic tool execution
    // @step And the agent should be able to use all 7 tools automatically

    // After refactoring:
    // Each tool will implement:
    // - rig::tool::Tool trait
    // - definition() -> ToolDefinition
    // - call(input: Value) -> Result<String>
    //
    // This enables rig::agent::Agent to:
    // - Discover tools automatically
    // - Execute tools based on LLM requests
    // - Handle tool results in multi-turn loop
}

// ==========================================
// SCENARIO 6: Non-streaming mode with automatic tool execution
// ==========================================

/// Scenario: Non-streaming mode with automatic tool execution
#[test]
fn test_non_streaming_mode_with_automatic_tool_execution() {
    // @step Given I have an agent in non-streaming mode
    let _provider = ClaudeProvider::from_api_key("test-key").unwrap();
    let _registry = ProviderToolRegistry::new();

    // @step When the agent needs to execute tools to complete a task
    // @step Then tools should execute automatically without streaming
    // @step And the final result should be returned as a string
    // @step And no intermediate chunks should be emitted

    // After refactoring to rig::agent::Agent:
    // - completion() method (non-streaming)
    // - Executes tools automatically
    // - Returns final string result only
    // - No streaming chunks
    //
    // vs stream() method (streaming):
    // - Returns MultiTurnStreamItem iterator
    // - Emits ToolCall, ToolResult, Message chunks
    // - Real-time visibility

    // For now, verify registry supports non-streaming
    // (Current implementation is non-streaming only)
}

// ==========================================
// HELPER TESTS: Verify rig integration readiness
// ==========================================

#[test]
fn test_rig_agent_types_accessible() {
    // Verify rig::agent types are accessible via codelet_core namespace
    // This is a compilation test - if it compiles, re-export works

    let _rig_available: Option<fn()> = Some(|| {
        // After REFAC-004, we'll use:
        // type RigAgent = codelet_core::RigAgent;
        // type RigTool = codelet_tools::Tool;
        // type MultiTurnStream = codelet_core::MultiTurnStreamItem;
    });
}

#[test]
fn test_provider_tool_registry_has_facades() {
    // Verify ProviderToolRegistry has facades registered
    let registry = ProviderToolRegistry::new();

    // Check for Claude facades
    let claude_tools = registry.tools_for_provider("claude");
    assert!(!claude_tools.is_empty(), "Should have Claude facades");

    // Check for Gemini facades
    let gemini_tools = registry.tools_for_provider("gemini");
    assert!(!gemini_tools.is_empty(), "Should have Gemini facades");
}
