@llm-provider
@provider-management
@REFAC-003
Feature: Refactor providers to use rig with streaming
  """
  Replace custom ClaudeProvider with rig::providers::anthropic. Use rig's CompletionModel trait. Support both completion() and stream() methods. Streaming handles text, tool calls, and extended thinking. OAuth via custom headers. Backward compatible with existing tests.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Must use rig::providers::anthropic::CompletionModel instead of custom ClaudeProvider
  #   2. Must implement both completion() and stream() methods from CompletionModel trait
  #   3. OAuth authentication must work with rig's Anthropic client using custom headers
  #   4. Streaming must support text chunks, tool call deltas, and extended thinking
  #   5. All existing ClaudeProvider tests must pass with new implementation
  #
  # EXAMPLES:
  #   1. Developer creates agent with rig Anthropic provider, calls stream_prompt(), receives text chunks in real-time
  #   2. Agent requests Read tool, streaming emits ToolCallDelta chunks, then complete ToolCall, then ToolResult
  #   3. Provider uses CLAUDE_CODE_OAUTH_TOKEN with Bearer auth header, successfully streams response
  #   4. Claude uses extended thinking, streaming emits Reasoning chunks separate from text
  #
  # ========================================
  Background: User Story
    As a developer using codelet
    I want to use rig's Anthropic provider with full streaming support
    So that I can see real-time responses and tool execution as they stream from Claude

  Scenario: Replace ClaudeProvider with rig Anthropic provider
    Given the codebase has a custom ClaudeProvider implementation
    When I refactor to use rig::providers::anthropic::CompletionModel
    Then the ClaudeProvider should use rig's Anthropic client internally
    And both completion() and stream() methods should be implemented
    And all existing ClaudeProvider tests should pass

  Scenario: Stream text chunks in real-time
    Given I have an agent using the rig Anthropic provider
    When I call stream() with a prompt
    Then I should receive text chunks as StreamingCompletionResponse
    And chunks should arrive in real-time as Claude generates them
    And the streaming should emit RawStreamingChoice::Message variants

  Scenario: Stream tool call deltas during tool execution
    Given I have an agent that can use tools
    When Claude requests the Read tool during streaming
    Then I should receive ToolCallDelta chunks as the tool call is built
    And I should receive a complete ToolCall when the tool request is complete
    And I should receive a ToolResult after tool execution
    And the streaming should emit RawStreamingChoice::ToolCallDelta variants

  Scenario: Authenticate using OAuth with custom headers
    Given CLAUDE_CODE_OAUTH_TOKEN environment variable is set
    When I create a rig Anthropic provider
    Then the provider should use Bearer auth header with the OAuth token
    And requests to the Anthropic API should be authenticated
    And streaming responses should succeed with valid OAuth credentials

  Scenario: Support extended thinking with reasoning chunks
    Given I have an agent using the rig Anthropic provider with extended thinking enabled
    When Claude generates a response with reasoning
    Then I should receive Reasoning chunks separate from text chunks
    And the streaming should emit RawStreamingChoice::Reasoning variants
    And reasoning chunks should be distinguishable from message content
