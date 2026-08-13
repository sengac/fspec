@BRIDGE-008
Feature: Telegram Stream Control Channel
  """
  BridgeManager in rust/napi handles WebSocket connections and message routing. Control messages need to be handled in the input_receiver mpsc channel handler that processes incoming WebSocket messages.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Control messages have type 'control' to distinguish from 'input' messages
  #   2. Supported actions: interrupt (stop agent), clear (reset session)
  #   3. Control messages are processed in the BridgeManager WebSocket message handler
  #   4. Interrupt action sets is_interrupted atomic flag to stop agent processing
  #   5. Clear action resets session state (messages, tool name map, agent state)
  #
  # EXAMPLES:
  #   1. Bridge receives {type: 'control', action: 'interrupt'}, agent stops processing
  #   2. Bridge receives {type: 'control', action: 'clear'}, session is reset
  #   3. Bridge receives {type: 'control', action: 'unknown'}, error logged but no crash
  #   4. Bridge receives {type: 'input'}, message forwarded to agent (existing behavior)
  #
  # ========================================
  Background: User Story
    As a bridge endpoint
    I want to receive control messages from remote sources
    So that control the agent session (interrupt, clear) separately from conversation input

  Scenario: Handle interrupt control message
    Given the bridge is connected to a session
    And the agent is processing a request
    When the bridge receives a message with type "control" and action "interrupt"
    Then the agent should stop processing
    And the is_interrupted flag should be set to true

  Scenario: Handle clear control message
    Given the bridge is connected to a session
    And the session has conversation history
    When the bridge receives a message with type "control" and action "clear"
    Then the session should be reset
    And the conversation history should be cleared

  Scenario: Handle unknown control action gracefully
    Given the bridge is connected to a session
    When the bridge receives a message with type "control" and action "unknown"
    Then an error should be logged
    And the bridge should not crash
    And the session should remain active

  Scenario: Forward input messages to agent (existing behavior)
    Given the bridge is connected to a session
    When the bridge receives a message with type "input"
    Then the message should be forwarded to the agent
    And the agent should process the input
