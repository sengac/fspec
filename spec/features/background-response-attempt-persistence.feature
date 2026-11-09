@BUG-582
@done
@phase-1
@agent-core
@provider-abstraction
@regression
@BUG-329
Feature: Background output persists and publishes response attempts
  """
  Architecture notes:
  Physical provider request boundaries reach BackgroundOutput (agent-loop) as
  ResponseAttemptStarted/ResponseAttemptAborted unit events. Each start
  checkpoints only the current attempt; an abort retracts its text and
  reasoning from persisted content without buffering live output or removing
  earlier segments, tool calls or results. Loop detection resumes from the
  state preceding the abandoned attempt. Every event is first admitted to the
  session's transcript journal, so output is published only for a session with
  a persisted manifest; an admission failure is a persistence failure that
  interrupts the turn. The UI side of rollback is specified in
  background-response-attempt-rollback.feature.
  """

  Background: User Story
    As a session owner whose provider retries a response
    I want abandoned attempts dropped from the durable conversation
    So that resumed sessions and loop detection only see output that was kept

  Scenario: Background persistence excludes abandoned text and reasoning
    Given earlier completed assistant output and a tool result
    When a later response streams text and reasoning and is aborted
    And a replacement response completes
    Then persistence contains earlier output and tools and only replacement output
    And subscribers receive attempt boundaries alongside live deltas

  Scenario: Aborted output does not poison loop detection
    Given a response attempt in a persisted session has nearly triggered repetition detection
    When the attempt is aborted and replacement output repeats its words
    Then the replacement is not interrupted by abandoned detector state

  Scenario: Attempt boundaries survive RPC serialization and bridge conversion
    Given response attempt start and abort chunks
    When the chunks are serialized and deserialized and converted for a bridge
    Then both unit boundaries retain their identity
