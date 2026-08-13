@done
@agent-execution
@scaffold
@REFAC-004
Feature: Refactor agent loop to use rig with multi-turn
  """
  Replace Runner with rig::agent::Agent. Automatic multi-turn tool calling with depth control. Support streaming (MultiTurnStreamItem) and non-streaming modes. All 7 tools implement rig::tool::Tool. Tool execution happens automatically in agent loop. Configurable max depth (default 10).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Must use rig::agent::Agent instead of custom Runner
  #   2. Must implement multi-turn tool calling with configurable depth (max 10 turns)
  #   3. Must support both streaming and non-streaming modes
  #   4. All 7 tools (Read,Write,Edit,Bash,Grep,Glob,AstGrep) must implement rig::tool::Tool trait
  #   5. Streaming must emit MultiTurnStreamItem with tool execution visibility
  #
  # EXAMPLES:
  #   1. User sends 'Read /tmp/test.txt', agent streams text chunks, then ToolCall, executes tool, streams ToolResult, then final text response
  #   2. Agent needs 3 tool calls to complete task, multi-turn depth set to 5, all 3 tools execute automatically
  #   3. Agent depth limit reached (10 turns), stops with error message about max depth exceeded
  #   4. Non-streaming mode: agent executes tools, returns final string result without intermediate chunks
  #
  # ========================================
  Background: User Story
    As a developer using codelet
    I want to have automatic multi-turn tool calling with streaming
    So that the agent handles tool execution automatically and I see results stream in real-time

  Scenario: Replace Runner with rig Agent for automatic tool execution
    Given the codebase has a custom Runner implementation
    When I refactor to use rig::agent::Agent
    Then the agent should automatically execute tools without manual loop handling
    And tool calling should support multi-turn with configurable depth
    And the default max depth should be 10 turns

  Scenario: Execute multiple tools automatically in multi-turn mode
    Given I have an agent with max depth set to 5
    When the agent needs 3 tool calls to complete a task
    Then all 3 tools should execute automatically without intervention
    And the agent should not exceed the max depth of 5
    And each tool execution should be counted as one turn

  Scenario: Stream tool execution with MultiTurnStreamItem
    Given I have an agent in streaming mode
    When the agent executes a tool during streaming
    Then I should receive a ToolCall item in the stream
    And I should receive a ToolResult item after execution
    And the stream should emit MultiTurnStreamItem variants
    And I should see tool execution visibility in real-time

  Scenario: Stop when max depth is reached
    Given I have an agent with max depth set to 10
    When the agent attempts to make 11 tool calls
    Then the agent should stop at turn 10
    And an error message about max depth exceeded should be returned
    And no further tool calls should be executed

  Scenario: All tools implement rig Tool trait
    Given codelet has 7 tools (Read, Write, Edit, Bash, Grep, Glob, AstGrep)
    When I refactor tools to implement rig::tool::Tool
    Then each tool should provide a tool definition via the Tool trait
    And each tool should implement the call() method
    And tools should integrate with rig's automatic tool execution
    And the agent should be able to use all 7 tools automatically

  Scenario: Non-streaming mode with automatic tool execution
    Given I have an agent in non-streaming mode
    When the agent needs to execute tools to complete a task
    Then tools should execute automatically without streaming
    And the final result should be returned as a string
    And no intermediate chunks should be emitted
