@BRIDGE-011
Feature: /stop command incorrectly reports agent as idle when actively processing

  """
  agentState in telegram-endpoint.ts must transition to 'thinking' when forwarding message to agent via WebSocket. Fix location: setupTelegramBot() message handler around line 693.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Agent state must be set to 'thinking' immediately when forwarding a message to the agent
  #   2. /stop should return 'Operation stopped' when agent is processing (state != idle)
  #   3. /stop should only return 'Nothing to stop - agent is idle' when no message has been sent to the agent
  #
  # EXAMPLES:
  #   1. User sends message, immediately sends /stop before any chunks arrive -> receives 'Operation stopped'
  #   2. User sends /stop while agent is executing tool (WebSearch) -> receives 'Operation stopped'
  #   3. User sends /stop when agent is truly idle (no pending message) -> receives 'Nothing to stop'
  #
  # ========================================

  Background: User Story
    As a Telegram user
    I want to stop agent processing with /stop command
    So that interrupt long-running operations accurately

  Scenario: Stop immediately after sending message
    Given a Telegram user is connected to the bridge
    And a codelet session is connected via WebSocket
    When the user sends a message to the agent
    And the user immediately sends /stop before any chunks arrive
    Then the user should receive "Operation stopped"

  Scenario: Stop while agent is executing tool
    Given a Telegram user is connected to the bridge
    And the agent is currently executing a tool
    When the user sends /stop
    Then the user should receive "Operation stopped"

  Scenario: Stop when agent is truly idle
    Given a Telegram user is connected to the bridge
    And the agent is idle with no pending messages
    When the user sends /stop
    Then the user should receive "Nothing to stop"
