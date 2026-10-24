@done
@BUG-243
@phase-1
@persistence
@context-management
Feature: Select authoritative provider state independently from dialogue
  """
  A verified manifest provider-state reference owns native continuation.
  Conversational envelope metadata remains original evidence, not an alternate
  provider-state source. Native metadata is supplied outside dialogue.
  """

  Background: User Story
    As an agent restoring a provider checkpoint
    I want continuation independent from conversation metadata
    So that aliases in original evidence cannot replace authoritative state

  Scenario: Select authoritative provider state independently from original metadata
    Given original dialogue carrying an older native checkpoint and a newer separate checkpoint
    When cold replay restores the committed provider state
    Then the separate checkpoint is authoritative and original dialogue remains unchanged
    And original metadata cannot replace or invalidate the committed native checkpoint
