@done
@CMPCT-078
@CMPCT-080
@providers
@provider-abstraction
@compaction
@phase-1
Feature: Restore the authoritative committed Codex checkpoint without suffix rollback
  """
  Committed replay carries the durable boundary generation on its synthetic
  summary envelope. Retained immutable suffix records may contain checkpoints
  from before that promotion. Validate every checkpoint before changing live
  state; never infer a generation reset from arbitrary checkpoint ordering.
  """

  Background: User Story
    As a Codex session owner
    I want committed replay to preserve the authoritative provider generation
    So that retained checkpoint metadata cannot resurrect replaced context

  Scenario: Retained pre-promotion checkpoints cannot roll back committed replay
    Given a durable committed Codex boundary followed by a genuine older checkpoint
    When I restore the ordered replay through the public provider session API
    Then the committed generation remains authoritative without modifying the replay
    And the next physical request contains only the committed DAG and complete compatible suffix

  Scenario Outline: Incompatible retained checkpoint metadata fails before state mutation
    Given a committed Codex generation and retained checkpoint with <incompatibility>
    When I restore the ordered replay through the public provider session API
    Then I receive a typed checkpoint failure with the prior committed state unchanged
    And ordinary inference remains blocked without sending an alternate request

    Examples:
      | incompatibility       |
      | unsupported version   |
      | unsupported protocol  |
      | future generation     |
      | same-generation model |
      | absent checkpoint     |

  Scenario: Same-generation checkpoints retain genuine post-promotion continuation
    Given a committed Codex boundary followed by its genuine same-generation checkpoint
    When I restore the ordered replay through the public provider session API
    Then compatible opaque reasoning and recorded tool outcomes survive exactly once
    And the committed generation remains authoritative without modifying the replay
