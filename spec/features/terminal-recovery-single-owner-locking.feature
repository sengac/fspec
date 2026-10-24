@done
@agent-core
@phase-1
@context-management
@compaction
@CMPCT-053
Feature: Terminal recovery single-owner locking
  """
  The background agent loop owns mutable session state through terminal resolution.
  Terminal recovery must not acquire the owner's mutex again. Failed compaction
  preserves the committed messages and returns a typed failure, not a fallback DAG.
  Tests drive the real outer loop and the failed recovery boundary with deadlines.
  """

  Scenario: Ordinary terminal errors release the session lock
    Given a background session running the real outer agent loop
    When provider construction returns an ordinary terminal error
    Then the error is published within the harness deadline
    And a later status operation can acquire the session lock

  Scenario: Failed overflow recovery preserves committed context
    Given a session with committed conversation and a failing DAG builder
    When terminal overflow recovery runs under the caller ownership boundary
    Then recovery returns a typed failure within the harness deadline
    And committed context is unchanged without compaction completion

  Scenario: Cancellation releases recovery ownership without replacing context
    Given a session whose admitted DAG builder is waiting
    When the user cancels terminal recovery
    Then recovery returns a cancellation within the harness deadline
    And the committed conversation remains available without completion
