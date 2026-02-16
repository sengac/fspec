@BRIDGE-009
Feature: User ID Whitelist for Telegram Bridge

  """
  Access control is implemented in setupTelegramBot() message handler. User IDs are stored in a Set<number> for O(1) lookup. Configuration is loaded at startup from TELEGRAM_ALLOWED_USER_IDS environment variable. The msg.from.id field (sender's unique Telegram ID) is used for validation, not msg.chat.id (which differs in groups).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. User whitelist is configured via TELEGRAM_ALLOWED_USER_IDS environment variable as comma-separated numeric IDs
  #   2. When whitelist is configured, only messages from users with IDs in the whitelist are processed
  #   3. Messages from unauthorized users are silently dropped (no response sent)
  #   4. Unauthorized access attempts are logged with user ID for audit purposes
  #   5. When no whitelist is configured (env var unset), all users are allowed
  #   6. User ID is extracted from msg.from.id (the sender), not msg.chat.id (the chat)
  #   7. Messages without a from field (e.g., channel posts) are dropped when whitelist is active
  #
  # EXAMPLES:
  #   1. TELEGRAM_ALLOWED_USER_IDS=123456789 - user 123456789 sends message - message is forwarded to codelet
  #   2. TELEGRAM_ALLOWED_USER_IDS=123456789 - user 999999999 sends message - message is dropped, log shows 'unauthorized user: 999999999'
  #   3. TELEGRAM_ALLOWED_USER_IDS=111,222,333 - user 222 sends message - message is forwarded (multiple IDs supported)
  #   4. TELEGRAM_ALLOWED_USER_IDS unset - any user sends message - message is forwarded (no whitelist = allow all)
  #   5. TELEGRAM_ALLOWED_USER_IDS=123456789 - message with no from field (channel post) - message is dropped
  #   6. TELEGRAM_ALLOWED_USER_IDS='abc,456,xyz' - only 456 is parsed as valid, non-numeric values ignored
  #   7. Startup with whitelist configured - log shows 'User whitelist enabled: N user(s)'
  #   8. Startup without whitelist - log shows 'No user whitelist configured - accepting all users'
  #
  # ========================================

  Background: User Story
    As a bot operator
    I want to restrict Telegram bridge access to specific user IDs
    So that prevent unauthorized users from accessing my codelet session

  # ========================================
  # SCENARIOS
  # ========================================

  @whitelist @authorized
  Scenario: Authorized user message is forwarded to codelet
    Given the endpoint is configured with TELEGRAM_ALLOWED_USER_IDS "123456789"
    And the endpoint is running with a connected codelet session
    When a Telegram message arrives from user ID 123456789
    Then the message should be forwarded to the codelet session

  @whitelist @unauthorized
  Scenario: Unauthorized user message is dropped silently
    Given the endpoint is configured with TELEGRAM_ALLOWED_USER_IDS "123456789"
    And the endpoint is running with a connected codelet session
    When a Telegram message arrives from user ID 999999999
    Then the message should not be forwarded to the codelet session
    And the log should contain "unauthorized user: 999999999"

  @whitelist @multiple-ids
  Scenario: Multiple user IDs can be whitelisted
    Given the endpoint is configured with TELEGRAM_ALLOWED_USER_IDS "111,222,333"
    And the endpoint is running with a connected codelet session
    When a Telegram message arrives from user ID 222
    Then the message should be forwarded to the codelet session

  @no-whitelist
  Scenario: No whitelist configured allows all users
    Given the endpoint is configured without TELEGRAM_ALLOWED_USER_IDS
    And the endpoint is running with a connected codelet session
    When a Telegram message arrives from user ID 999999999
    Then the message should be forwarded to the codelet session

  @whitelist @no-from-field
  Scenario: Message without from field is dropped when whitelist active
    Given the endpoint is configured with TELEGRAM_ALLOWED_USER_IDS "123456789"
    And the endpoint is running with a connected codelet session
    When a Telegram message arrives without a from field
    Then the message should not be forwarded to the codelet session
    And the log should contain "no user ID"

  @whitelist @invalid-ids
  Scenario: Invalid user IDs in environment variable are filtered out
    Given the endpoint is configured with TELEGRAM_ALLOWED_USER_IDS "abc,456,xyz"
    And the endpoint is running with a connected codelet session
    When a Telegram message arrives from user ID 456
    Then the message should be forwarded to the codelet session

  @startup @whitelist
  Scenario: Startup logs whitelist enabled message
    Given the endpoint is configured with TELEGRAM_ALLOWED_USER_IDS "111,222,333"
    When the endpoint starts up
    Then the log should contain "User whitelist enabled: 3 user(s)"

  @startup @no-whitelist
  Scenario: Startup logs no whitelist message
    Given the endpoint is configured without TELEGRAM_ALLOWED_USER_IDS
    When the endpoint starts up
    Then the log should contain "No user whitelist configured - accepting all users"
