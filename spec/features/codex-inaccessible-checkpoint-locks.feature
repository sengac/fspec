@done
@CMPCT-080
@providers
@provider-abstraction
@compaction
@phase-1
Feature: Codex inaccessible checkpoint locks
  """
  Public provider replay integration tests exercise unsupported state and a
  terminated persistence writer while preserving the durable checkpoint. The
  existing private lock-isolation unit regressions remain additional coverage.
  """

  Background: User Story
    As a Codex session owner
    I want invalid checkpoint state to fail explicitly
    So that corrupted continuation cannot masquerade as a genuinely new session

  Scenario Outline: Isolated inaccessible checkpoint locks fail without substituting fresh history
    Given an existing durable Codex checkpoint with only its <lock> lock inaccessible
    When I restore or prepare its continuation
    Then the inaccessible lock returns its specific typed checkpoint failure
    And no provider request or completion is emitted
    And a cold restore retains the last durable checkpoint and generation

    Examples:
      | lock   |
      | state  |
      | writer |
