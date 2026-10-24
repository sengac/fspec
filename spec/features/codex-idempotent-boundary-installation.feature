@done
@CMPCT-078
@CMPCT-080
@providers
@provider-abstraction
@compaction
@phase-1
Feature: Reinstall a committed Codex boundary without resetting valid continuation
  """
  Restoring a live session installs the durable boundary before replaying saved
  provider envelopes. An already extended checkpoint for the same generation
  and model remains authoritative throughout repeated restoration. Different
  models and older generations remain incompatible.
  """

  Background: User Story
    As a Codex session owner
    I want repeated committed boundary restoration to be idempotent
    So that completed continuation is neither rejected nor erased

  Scenario: Reinstalling the committed boundary preserves a completed continuation
    Given a real Codex response has extended the committed generation checkpoint
    When I reinstall the durable boundary and restore the same replay twice
    Then the complete continuation checkpoint remains unchanged after each restore
    And the next physical request contains the committed DAG and complete continuation exactly once

  Scenario: Reinstalling a different model cannot replace same-generation continuation
    Given a completed continuation belongs to the committed Codex model
    When I reinstall a same-generation boundary for a different model
    Then I receive a typed checkpoint failure with the prior committed state unchanged

  Scenario: Replayed metadata cannot replace the installed same-generation model
    Given a completed continuation belongs to the installed committed model
    When retained same-generation replay metadata names a different model without a boundary envelope
    Then restoration fails with typed lineage error before changing the checkpoint
    And physical inference remains blocked until the authentic replay is restored
