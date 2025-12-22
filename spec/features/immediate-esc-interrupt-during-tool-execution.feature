@agent-modal
@cli
@agent-integration
@critical
@napi
@tui
@interrupt
@NAPI-004
Feature: Immediate ESC interrupt during tool execution

  """
  Uses tokio::sync::Notify to allow JavaScript interrupt signal to immediately wake blocked stream.next().await in NAPI mode. The interrupt() method calls notify_waiters() which wakes the tokio::select! in run_agent_stream_internal. CLI mode uses existing keyboard event stream via tokio::select! and is unchanged. Implementation requires adding Notify field to CodeletSession and passing it through to stream_loop.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ESC key must interrupt agent execution within 100ms regardless of current operation
  #   2. Interrupt mechanism must use tokio::sync::Notify to wake blocked stream awaits
  #   3. CLI mode interrupt behavior must remain unchanged (uses keyboard event stream)
  #   4. Interrupted agent must emit Interrupted chunk and preserve partial response in history
  #
  # EXAMPLES:
  #   1. User starts agent, agent calls slow file-read tool (5s), user presses ESC after 1s, agent stops within 100ms
  #   2. User starts agent, agent streams text response, user presses ESC, agent stops immediately and partial text is preserved
  #   3. User starts agent, agent is waiting for API response, user presses ESC, tokio::select! wakes via Notify and breaks loop
  #   4. CLI mode: User starts codelet CLI, presses ESC during streaming, existing keyboard event handling still works unchanged
  #
  # ========================================

  Background: User Story
    As a developer using fspec TUI agent
    I want to press ESC to immediately stop agent execution during tool calls
    So that I don't have to wait for slow operations to finish before regaining control

  @critical
  Scenario: Interrupt agent during slow tool execution
    Given the agent is executing a tool call that takes 5 seconds
    When I press ESC after 1 second
    Then the agent should stop within 100 milliseconds
    And I should see "Agent interrupted" message
    And partial response should be preserved in conversation history

  @critical
  Scenario: Interrupt agent during text streaming
    Given the agent is streaming a text response
    When I press ESC
    Then the agent should stop immediately
    And I should see "Agent interrupted" message
    And partial text should be preserved in conversation history

  @critical
  Scenario: Interrupt agent waiting for API response
    Given the agent is waiting for an API response
    When I press ESC
    Then the tokio select should wake via Notify
    And the stream loop should break immediately
    And I should see "Agent interrupted" message

  @regression
  Scenario: CLI mode interrupt behavior unchanged
    Given I am using the codelet CLI directly
    And the agent is streaming a response
    When I press ESC
    Then the existing keyboard event handling should work
    And the agent should stop immediately
    And this behavior should be unchanged from before the fix