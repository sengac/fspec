@done
@CMPCT-080
@providers
@provider-abstraction
@compaction
@phase-1
Feature: Reject invalid Codex checkpoint state without fresh-session substitution
  """
  Public provider replay integration tests exercise unsupported state and a
  terminated persistence writer while preserving the durable checkpoint. The
  existing private lock-isolation unit regressions remain additional coverage.
  """

  Background: User Story
    As a Codex session owner
    I want invalid checkpoint state to fail explicitly
    So that corrupted continuation cannot masquerade as a genuinely new session

  Scenario Outline: Invalid checkpoint state is not treated as an absent checkpoint
    Given an existing Codex session with <failure>
    When I restore or prepare its continuation
    Then I receive an explicit checkpoint failure rather than a fresh session
    And no provider request or completion is emitted
    And the last durable checkpoint and committed generation remain authoritative

    Examples:
      | failure                            |
      | an unsupported checkpoint version  |
      | terminated checkpoint persistence  |
      | a missing required checkpoint      |
      | malformed checkpoint metadata      |
      | an unsupported checkpoint protocol |
