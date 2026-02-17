@bridge
@telegram
@BRIDGE-010
Feature: Telegram Slash Commands for Agent Control
  """
  Slash commands are intercepted in setupTelegramBot's message handler BEFORE forwarding to codelet. Commands are detected by checking if text starts with '/'. The SlashCommandHandler processes commands synchronously and returns responses via bot.sendMessage directly to the same chatId. This does NOT require BRIDGE-008 control channel - responses go directly to Telegram.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Slash commands start with / and are case-insensitive
  #   2. Slash commands are intercepted before being sent to the agent
  #   3. Unknown slash commands show an error message with available commands
  #   4. Command responses are sent via the control channel, not as agent output
  #   5. /help shows all available commands
  #   6. /status shows agent session state (idle, thinking, executing tools)
  #   7. /stop interrupts the current agent operation
  #   8. /clear clears the conversation history and resets the session
  #
  # EXAMPLES:
  #   1. User sends /help, receives list of available commands
  #   2. User sends /status while agent is idle, sees 'Agent is idle'
  #   3. User sends /status while agent is processing, sees 'Agent is thinking...'
  #   4. User sends /stop while agent is running, agent operation is interrupted
  #   5. User sends /stop while agent is idle, sees 'Nothing to stop'
  #   6. User sends /clear, conversation history is wiped and session reset
  #   7. User sends /unknown, sees error with list of valid commands
  #   8. User sends /HELP (uppercase), still triggers help command
  #
  # ========================================
  Background: User Story
    As a Telegram user
    I want to send slash commands to control the agent
    So that manage the agent session without typing natural language

  Scenario: Show available commands with /help
    Given the Telegram bridge is connected to a session
    When I send "/help"
    Then I should receive a message listing all available commands
    And the message should include "/help", "/status", "/stop", and "/clear"

  Scenario: Check status when agent is idle
    Given the Telegram bridge is connected to a session
    And the agent is idle
    When I send "/status"
    Then I should receive a message saying "Agent is idle"

  Scenario: Check status when agent is processing
    Given the Telegram bridge is connected to a session
    And the agent is thinking
    When I send "/status"
    Then I should receive a message saying "Agent is thinking..."

  Scenario: Stop agent when it is running
    Given the Telegram bridge is connected to a session
    And the agent is executing an operation
    When I send "/stop"
    Then the agent operation should be interrupted
    And I should receive confirmation that the operation was stopped

  Scenario: Stop agent when it is already idle
    Given the Telegram bridge is connected to a session
    And the agent is idle
    When I send "/stop"
    Then I should receive a message saying "Nothing to stop"

  Scenario: Clear conversation history
    Given the Telegram bridge is connected to a session
    And the session has conversation history
    When I send "/clear"
    Then the conversation history should be cleared
    And the session should be reset
    And I should receive confirmation that the session was cleared

  Scenario: Handle unknown slash command
    Given the Telegram bridge is connected to a session
    When I send "/unknown"
    Then I should receive an error message
    And the error message should list the available commands

  Scenario: Slash commands are case-insensitive
    Given the Telegram bridge is connected to a session
    When I send "/HELP"
    Then I should receive a message listing all available commands
    And the message should include "/help", "/status", "/stop", and "/clear"

  Scenario: Slash commands are not forwarded to the agent
    Given the Telegram bridge is connected to a session
    When I send "/help"
    Then the message should NOT be forwarded to the agent session
    And the response should come directly from the bridge
