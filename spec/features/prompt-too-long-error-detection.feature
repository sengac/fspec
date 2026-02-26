@done
@error-handling
@cli
@bug-fix
@compaction
@PROV-010
Feature: False positive prompt-too-long detection triggers empty compaction on Opus 4.6

  """
  Fix is_prompt_too_long_error() in codelet/cli/src/interactive/stream_loop.rs to exclude thinking budget errors
  Add guard in error handler (~line 1173) to verify compactable turns exist before triggering compaction
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Thinking budget configuration errors (thinking.budget_tokens) must NOT be classified as prompt-too-long errors
  #   2. Compaction must only be triggered when there are actual user/assistant turns to compact
  #   3. System-only messages (no conversation turns) must NOT trigger compaction even for real prompt-too-long errors
  #   4. Actual context length errors (context_length_exceeded, prompt is too long) must still be correctly detected
  #   5. Configuration errors must propagate to the user with clear error messages, not trigger compaction
  #
  # EXAMPLES:
  #   1. Opus 4.6 returns 'max_tokens must be greater than thinking.budget_tokens' error → error is NOT classified as prompt-too-long → compaction is NOT triggered → error propagates to user
  #   2. API returns 'prompt is too long' error with session containing system prompts only (0 turns) → error IS classified as prompt-too-long → but compaction is NOT triggered (no turns) → error propagates to user
  #   3. API returns 'context_length_exceeded' error with session containing 5 user/assistant turns → error IS classified as prompt-too-long → compaction IS triggered → context is reduced
  #   4. API returns 'invalid_request_error: maximum tokens exceeded' error with 10 turns → error IS classified as prompt-too-long → compaction IS triggered
  #   5. API returns 'invalid_request_error: budget_tokens' error (any config error mentioning tokens) → error is NOT classified as prompt-too-long → error propagates to user
  #
  # ========================================

  Background: User Story
    As a developer
    I want to have accurate prompt-too-long error detection
    So that compaction only triggers for actual context overflow, not configuration errors

  # ========================================
  # Bug Fix 1: False Positive Detection
  # ========================================

  @unit
  Scenario: Thinking budget configuration error is not classified as prompt-too-long
    Given an error message containing "invalid_request_error"
    And the error message contains "max_tokens must be greater than thinking.budget_tokens"
    When the error is checked by is_prompt_too_long_error
    Then the function should return false
    And the error should NOT trigger compaction

  @unit
  Scenario: Generic budget_tokens configuration error is not classified as prompt-too-long
    Given an error message containing "invalid_request_error"
    And the error message contains "budget_tokens"
    When the error is checked by is_prompt_too_long_error
    Then the function should return false

  # ========================================
  # Bug Fix 2: Empty Turn History Guard
  # ========================================

  @integration
  Scenario: Prompt too long with zero conversation turns does not trigger compaction
    Given a session with only system prompt messages
    And the session has zero user/assistant conversation turns
    When an API error "prompt is too long" is received
    Then the error should be classified as prompt-too-long
    But compaction should NOT be triggered
    And the error should propagate to the user

  @integration
  Scenario: Prompt too long with conversation turns triggers compaction
    Given a session with system prompt messages
    And the session has 5 user/assistant conversation turns
    When an API error "context_length_exceeded" is received
    Then the error should be classified as prompt-too-long
    And compaction should be triggered
    And the context should be reduced

  # ========================================
  # Regression: Ensure legitimate errors still detected
  # ========================================

  @unit
  Scenario: Actual prompt too long error is correctly detected
    Given an error message "prompt is too long"
    When the error is checked by is_prompt_too_long_error
    Then the function should return true

  @unit
  Scenario: Context length exceeded error is correctly detected
    Given an error message "context_length_exceeded"
    When the error is checked by is_prompt_too_long_error
    Then the function should return true

  @unit
  Scenario: Maximum context length error is correctly detected
    Given an error message "maximum context length"
    When the error is checked by is_prompt_too_long_error
    Then the function should return true

  @unit
  Scenario: Too many tokens error is correctly detected
    Given an error message "too many tokens"
    When the error is checked by is_prompt_too_long_error
    Then the function should return true

  @unit
  Scenario: Invalid request error with maximum tokens is correctly detected
    Given an error message containing "invalid_request_error"
    And the error message contains "maximum"
    And the error message does NOT contain "budget_tokens"
    When the error is checked by is_prompt_too_long_error
    Then the function should return true

  # ========================================
  # Edge Cases
  # ========================================

  @unit
  Scenario: Error message with both budget_tokens and context_length is NOT classified as prompt-too-long
    Given an error message containing "invalid_request_error"
    And the error message contains "budget_tokens"
    And the error message contains "context_length"
    When the error is checked by is_prompt_too_long_error
    Then the function should return false
    # budget_tokens exclusion takes precedence

  @integration
  Scenario: Configuration error propagates to user with clear message
    Given a session with any number of conversation turns
    When an API error "max_tokens must be greater than thinking.budget_tokens" is received
    Then the error should NOT be classified as prompt-too-long
    And the error should propagate to the user with the original message
