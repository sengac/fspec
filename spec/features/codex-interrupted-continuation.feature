@testing
@providers
@persistence
@provider-abstraction
@PROV-149
Feature: Codex interrupted continuation
  """
  Typed replay requires real cancellation outcomes before sending new input;
  providers must not manufacture successful or cancelled tool results.
  Reminder markup inside ordinary user messages does not exempt those messages
  from conversation replacement detection. CMPCT-078 requires an explicit
  committed generation boundary before conversation replacement.
  """

  Scenario: Interrupted tool calls receive cancellation outputs before new input
    Given a saved Codex response with parallel tool calls
    And only one tool result survived an interruption
    When a new user message continues the session
    Then the missing outcome is rejected without changing checkpoint or input
    When the caller supplies the genuine cancellation outcome and retries
    Then the completed tool result is preserved
    And each unfinished call receives a cancellation output before the new message

  Scenario: Reminder-bearing user messages still invalidate replaced conversations
    Given a saved Codex conversation whose user message includes a system reminder
    When local compaction commits an advancing boundary before replacing the conversation
    Then the old typed continuation is discarded
    And the replacement message is sent without stale reasoning or tool calls
