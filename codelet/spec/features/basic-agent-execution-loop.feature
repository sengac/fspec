@agent-execution
@tool-execution
@AGENT-001
Feature: Basic Agent Execution Loop

  """
  Implementation:
  - Runner.run() takes user input String, returns Result<Vec<Message>> containing full conversation
  - Uses async loop pattern - while let Some(tool_calls) = parse_tool_calls(response)
  - Tool definitions generated from ToolRegistry.definitions() as JSON schema for API request
  Dependency:
  - Uses existing LlmProvider trait from src/providers/mod.rs and ClaudeProvider from src/providers/claude.rs
  - Uses existing ToolRegistry from src/tools/mod.rs for tool execution
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Runner MUST accept an LlmProvider (via constructor or method) to enable conversation with the LLM
  #   2. Runner.run() MUST call provider.complete() with current message history to get LLM response
  #   3. Runner MUST parse tool_use content blocks from assistant responses to detect tool call requests
  #   4. Runner MUST execute detected tool calls via ToolRegistry.execute() and capture results
  #   5. Runner MUST inject tool_result blocks back into the conversation as user messages after tool execution
  #   6. Runner MUST continue the loop (call provider.complete again) when response contains tool_use blocks
  #   7. Runner MUST exit the loop when response stop_reason is end_turn (no tool calls)
  #   8. Runner MUST include tool definitions in the API request (from ToolRegistry.definitions())
  #
  # EXAMPLES:
  #   1. Runner initialized with ClaudeProvider calls provider.complete() and receives text response, stop_reason=end_turn, loop exits
  #   2. Response contains tool_use block for Read tool, Runner executes ReadTool.execute(), injects tool_result, calls complete() again
  #   3. Response contains multiple tool_use blocks, Runner executes all tools in sequence, injects all results, continues loop
  #   4. Tool execution fails (file not found), Runner injects tool_result with is_error=true, continues loop for LLM to handle error
  #   5. After 3 tool call iterations, LLM returns end_turn with final text response, Runner exits loop and returns accumulated messages
  #   6. API returns error (invalid API key), Runner propagates error with clear message about authentication failure
  #
  # ========================================

  Background: User Story
    As a AI coding agent
    I want to orchestrate conversations with an LLM provider and execute tools when requested
    So that I can actually function as an interactive agent that responds to users and performs tasks

  Scenario: Simple text response exits loop immediately
    Given a Runner initialized with ClaudeProvider and default tools
    When I send a user message "Hello, how are you?"
    And the provider returns a text response with stop_reason "end_turn"
    Then the runner should exit the loop
    And the messages should contain the assistant response

  Scenario: Single tool call is executed and result injected
    Given a Runner initialized with ClaudeProvider and default tools
    When I send a user message "Read the file /tmp/test.txt"
    And the provider returns a response containing a tool_use block for "Read"
    Then the runner should execute the Read tool with the provided arguments
    And the runner should inject a tool_result message into the conversation
    And the runner should call provider.complete() again with the updated messages

  Scenario: Multiple tool calls are executed in sequence
    Given a Runner initialized with ClaudeProvider and default tools
    When I send a user message "Read two files"
    And the provider returns a response containing multiple tool_use blocks
    Then the runner should execute all tools in sequence
    And the runner should inject all tool_result messages
    And the runner should continue the loop

  Scenario: Failed tool execution injects error result
    Given a Runner initialized with ClaudeProvider and default tools
    When I send a user message "Read /nonexistent/file.txt"
    And the provider returns a tool_use block for "Read" with path "/nonexistent/file.txt"
    And the Read tool execution fails with "file not found"
    Then the runner should inject a tool_result with is_error set to true
    And the runner should continue the loop for the LLM to handle the error

  Scenario: Multi-turn tool execution loop completes successfully
    Given a Runner initialized with ClaudeProvider and default tools
    When I send a user message "Help me fix the bug in main.rs"
    And the provider returns tool_use blocks for 3 iterations
    And the final response has stop_reason "end_turn"
    Then the runner should have executed tools across all iterations
    And the runner should exit the loop with the final response
    And the messages should contain all tool calls and results

  Scenario: API authentication error is propagated
    Given a Runner initialized with an invalid API key
    When I send a user message "Hello"
    And the provider returns an authentication error
    Then the runner should return an error Result
    And the error message should indicate authentication failure

  Scenario: Tool definitions are included in API request
    Given a Runner initialized with ClaudeProvider and default tools
    When I send a user message "What tools do you have?"
    Then the API request should include tool definitions from ToolRegistry
    And each tool definition should have name, description, and parameters schema
