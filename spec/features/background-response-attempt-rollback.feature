@phase-1
@agent-core
@provider-abstraction
@regression
Feature: Output consumers retract abandoned response attempts
  """
  Architecture notes:
  Physical provider request boundaries reach background persistence and UI
  consumers as unit events. Each start checkpoints only the current attempt.
  An abort retracts its text and reasoning without buffering live output or
  removing earlier successful segments, tool calls, results or notifications.
  Loop detection resumes from the state preceding the abandoned attempt.
  Inspecting retracted output closes its modal without disturbing modals
  whose inspected output survives the rollback.
  """

  Scenario: Background persistence excludes abandoned text and reasoning
    Given earlier completed assistant output and a tool result
    When a later response streams text and reasoning and is aborted
    And a replacement response completes
    Then persistence contains earlier output and tools and only replacement output
    And subscribers receive attempt boundaries alongside live deltas

  Scenario: Aborted output does not poison loop detection
    Given a response attempt has nearly triggered repetition detection
    When the attempt is aborted and replacement output repeats its words
    Then the replacement is not interrupted by abandoned detector state

  Scenario: The UI retracts only the latest attempt while preserving streaming
    Given earlier assistant text and reasoning and a completed tool card
    When a later response streams text and reasoning
    Then its deltas are visible before completion
    When that response is aborted and a replacement completes
    Then only the abandoned text and reasoning disappear
    And the earlier output tool card and reconnect notification remain

  Scenario: A start checkpoints existing streaming blocks without clearing them
    Given assistant text and reasoning already occupy streaming blocks
    When another response attempt appends to those blocks and is aborted
    Then the original block contents and streaming cursors are restored
    And an abort without another start changes nothing

  Scenario: Attempt boundaries survive RPC serialization and bridge conversion
    Given response attempt start and abort chunks
    When the chunks are serialized and deserialized and converted for a bridge
    Then both unit boundaries retain their identity

  Scenario: Aborting the inspected attempt closes its turn modal
    Given earlier completed output and an open turn modal showing a live response attempt
    When that response attempt is aborted
    Then the modal and its text selection are dismissed
    And scrollback keyboard and mouse navigation work without pressing Escape
    And replacement output can be inspected normally

  Scenario: Rollback preserves a modal whose inspected output remains
    Given a turn modal showing completed output
    When another response attempt is aborted in either the focused or a background session
    Then the inspected output remains open and unchanged

  Scenario: Turn navigation recovers after the selected attempt is retracted
    Given turn selection is focused on an abandoned response attempt
    When the response attempt is aborted
    And surviving or replacement output is available
    Then either arrow key selects an available turn without toggling selection mode
    And further arrow navigation moves between available turns

  Scenario: Queued input preserves rollback of the active response attempt
    Given earlier completed output and a live response attempt with text and reasoning
    When another user prompt is queued before the response attempt is aborted
    And a replacement response completes
    Then the abandoned text and reasoning disappear
    And the queued prompt earlier output and replacement output remain visible

  Scenario: Aborting inspected output in a reused streaming block closes its modal
    Given a completed tool result followed by a live response in its assistant placeholder
    And a turn modal with selected text from that live response
    When the response attempt is aborted
    Then the empty assistant placeholder is restored
    And the modal and its text selection are dismissed
    And scrollback keyboard navigation resumes without pressing Escape
