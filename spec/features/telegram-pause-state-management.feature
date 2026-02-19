@done
@pause-integration
@bridge
@telegram
@BRIDGE-014
Feature: Telegram Pause State Management Commands
  """
  Extend telegram-slash-commands.ts AVAILABLE_COMMANDS array with /allowonce, /allowsession, /deny commands
  Add 'isPaused' boolean and 'pauseInfo' object to EndpointState in telegram-endpoint.ts to track pause state
  Add new chunk type 'pause_request' to StreamChunkData with kind, message, and details fields
  Extend SlashCommandResult.action union type to include 'allow_once' | 'allow_session' | 'deny' values
  Handle 'pause_response' control action in bridge_handler.rs by calling session_pause_triple(session_id, choice)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. /allowonce (or /allow), /allowsession, and /deny commands respond to PauseKind::Triple prompts
  #   2. Pause response commands only work when the session is actually paused, otherwise show error
  #   3. /status shows 'paused' state alongside idle/thinking/executing when tool pause is active
  #   4. Telegram endpoint receives pause state notifications from codelet via WebSocket chunks
  #   5. Pause response sent via control channel message with action 'pause_response' and response field
  #
  # EXAMPLES:
  #   1. Agent reads ~/.ssh/config → Telegram shows ⏸ Read: Sensitive file access (.ssh) → User sends /allowonce → File read → Next access prompts again
  #   2. Agent reads ~/.env → User sends /allowsession → File read → Later access to other .env files → No prompt (session allowed)
  #   3. Agent reads ~/.aws/credentials → User sends /deny → Read blocked → AI receives 'User denied access' error
  #   4. User sends /deny when agent is not paused → Telegram shows ⚠️ No pending pause to respond to
  #   5. User sends /status while paused for sensitive file access → Telegram shows ⏸ Paused: Waiting for access decision
  #   6. /help shows all commands including /allowonce, /allowsession, /deny
  #
  # ========================================
  Background: User Story
    As a Telegram user
    I want to respond to sensitive file access prompts remotely
    So that control agent access to sensitive files without needing TUI access

  # Example 1: Allow once flow
  Scenario: User allows sensitive file access once via /allowonce
    Given the agent is connected via Telegram bridge
    And the agent attempts to read "~/.ssh/config"
    And a pause prompt is shown in Telegram "⏸ Read: Sensitive file access (.ssh)"
    When the user sends "/allowonce"
    Then the file read should proceed
    And the next access to "~/.ssh/config" should prompt again

  # Rule 0: /allow is an alias for /allowonce
  Scenario: User allows sensitive file access once via /allow alias
    Given the agent is connected via Telegram bridge
    And a pause prompt is shown in Telegram
    When the user sends "/allow"
    Then the file read should proceed
    And the behavior should be identical to "/allowonce"

  # Example 2: Allow session flow
  Scenario: User allows sensitive file access for session via /allowsession
    Given the agent is connected via Telegram bridge
    And the agent attempts to read "~/.env"
    And a pause prompt is shown in Telegram
    When the user sends "/allowsession"
    Then the file read should proceed
    And later access to other ".env" files should not prompt

  # Example 3: Deny flow
  Scenario: User denies sensitive file access via /deny
    Given the agent is connected via Telegram bridge
    And the agent attempts to read "~/.aws/credentials"
    And a pause prompt is shown in Telegram
    When the user sends "/deny"
    Then the file read should be blocked
    And the AI should receive "User denied access" error

  # Example 4: Error when not paused (applies to all pause commands per Rule 1)
  Scenario: User sends /deny when agent is not paused
    Given the agent is connected via Telegram bridge
    And the agent is not currently paused
    When the user sends "/deny"
    Then Telegram should show "⚠️ No pending pause to respond to"

  # Rule 1: All pause commands require active pause state
  Scenario: User sends /allowonce when agent is not paused
    Given the agent is connected via Telegram bridge
    And the agent is not currently paused
    When the user sends "/allowonce"
    Then Telegram should show "⚠️ No pending pause to respond to"

  Scenario: User sends /allowsession when agent is not paused
    Given the agent is connected via Telegram bridge
    And the agent is not currently paused
    When the user sends "/allowsession"
    Then Telegram should show "⚠️ No pending pause to respond to"

  # Example 5: Status while paused
  Scenario: User checks status while agent is paused
    Given the agent is connected via Telegram bridge
    And the agent is paused for sensitive file access
    When the user sends "/status"
    Then Telegram should show "⏸ Paused: Waiting for access decision"

  # Example 6: Help shows new commands
  Scenario: Help command shows pause management commands
    Given the agent is connected via Telegram bridge
    When the user sends "/help"
    Then the response should include "/allowonce"
    And the response should include "/allow" as an alias
    And the response should include "/allowsession"
    And the response should include "/deny"

  # Integration: Pause request notification via WebSocket
  @integration
  Scenario: Telegram endpoint receives pause request from codelet
    Given the Telegram bridge is connected to a codelet session
    When the codelet sends a "pause_request" chunk with kind "triple"
    Then the endpoint should set isPaused to true
    And the endpoint should store the pause info
    And a pause notification should be sent to Telegram

  # Integration: Pause response sent via control channel (tests all action values)
  @integration
  Scenario: Pause response with allow_once sent through WebSocket control channel
    Given the Telegram bridge is connected to a codelet session
    And the session is currently paused
    When the user sends "/allowonce"
    Then a control message should be sent with action "pause_response"
    And the response field should be "allow_once"
    And the bridge_handler should call session_pause_triple

  @integration
  Scenario: Pause response with allow_session sent through WebSocket control channel
    Given the Telegram bridge is connected to a codelet session
    And the session is currently paused
    When the user sends "/allowsession"
    Then a control message should be sent with action "pause_response"
    And the response field should be "allow_session"
    And the bridge_handler should call session_pause_triple

  @integration
  Scenario: Pause response with deny sent through WebSocket control channel
    Given the Telegram bridge is connected to a codelet session
    And the session is currently paused
    When the user sends "/deny"
    Then a control message should be sent with action "pause_response"
    And the response field should be "deny"
    And the bridge_handler should call session_pause_triple
