@done
@validation
@phase-1
@agent-core
@cli
@context-management
@tokens
@compaction
@BUG-236
Feature: Token input evidence survives settlement and stays request scoped
  """
  Input observations and display output settlement have separate owners. A stale
  display cannot replace accepted input evidence, reset caches, or recalculated
  content estimates. Legacy numeric display snapshots remain usable when input
  evidence is unknown or uninitialized. Every physical request, including an
  internal tool follow-up, starts an independent observation scope. Provider
  attempt aborts, retry streams, compaction restarts and Gemini continuations
  discard provisional input observations rather than inheriting them.
  """

  Background: User Story
    As a session user
    I want input evidence preserved through display settlement and isolated between requests
    So that usage labels and compaction measurements describe the current context

  Scenario: Streamed input survives zero input final output settlement
    Given a response streams 30000 fresh input and 150000 cached input tokens
    When its final response reports zero input and positive output and the turn settles
    Then current input remains 180000 with reported provenance and the original cache split
    And output and reasoning reflect the final display without inventing input billing

  Scenario: Post compaction input survives stale display settlement
    Given content recalculation replaced a reported cached context and reset its cache split
    When the old streaming display settles after compaction
    Then the recalculated input estimate and estimated or reset provenance are preserved
    And stale cache tokens never reappear and session spend is unchanged

  Scenario: Aborted attempt usage cannot supply a replacement response
    Given attempt A streams input usage and is aborted before completion
    When replacement attempt B completes without usable input usage
    Then input accounting is unavailable rather than inherited from attempt A
    And the current content estimate remains usable without a provider omission claim

  Scenario: Internal tool followup starts independent input accounting
    Given one physical request streams input usage and completes a tool call
    When its internal followup request completes without input usage
    Then the followup accounting is unavailable rather than reusing the tool request input

  Scenario: Retry after a failed request discards provisional input
    Given a physical request streams input usage and fails with a retryable connection error
    When its retry completes without input usage
    Then retry accounting is unavailable rather than reusing failed request input

  Scenario: Gemini continuation attempts discard aborted input observations
    Given a Gemini empty tool followup starts a continuation stream
    When continuation attempt A reports input then aborts and replacement B completes without input usage
    Then settled Gemini input is unavailable rather than inherited from the aborted continuation
