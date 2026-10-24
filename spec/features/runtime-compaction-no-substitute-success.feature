@cli
@context-management
@compaction
@CMPCT-079
Feature: Runtime compaction without substitute success
  """
  Supersedes the runtime fallback-success policies of CMPCT-016, CMPCT-020 and
  CMPCT-047. One selected agent-owned DAG protocol may commit only a fully
  validated candidate. Diagnostics and partial output are never candidates.
  Failure preserves committed memory, blocks inference and emits no completion.
  """

  Scenario: Missing builder preserves the committed conversation
    Given committed conversation and no registered DAG builder
    When automatic compaction runs
    Then a typed missing-builder failure is returned
    And committed conversation is unchanged without completion
    And the busy indicator clears while inference remains blocked

  Scenario: Builder diagnostics cannot become a replacement DAG
    Given committed conversation and a builder failing with valid-looking DAG diagnostics
    When automatic compaction runs
    Then a typed builder failure is returned
    And committed conversation is unchanged without completion
    And the busy indicator clears while inference remains blocked

  Scenario: Invalid builder output cannot become a replacement DAG
    Given committed conversation and a builder returning invalid output
    When automatic compaction runs
    Then a typed invalid-candidate failure is returned
    And committed conversation is unchanged without completion
    And the busy indicator clears while inference remains blocked

  Scenario: Deadline cancels a stalled builder without changing committed memory
    Given committed conversation and a builder waiting indefinitely
    When the declared compaction deadline expires
    Then a typed deadline failure is returned within the harness deadline
    And committed conversation is unchanged without completion
    And the busy indicator clears while inference remains blocked
