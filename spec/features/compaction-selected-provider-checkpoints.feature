@BUG-212
@done
@cli
@context-management
@compaction
Feature: Compaction selected provider checkpoints
  """
  Provider extensions can outlive a model switch. New durable compaction prepares
  only the selected base provider's checkpoint state. Restore and reconciliation
  include explicit metadata owners, not unrelated inactive transport generations.
  Exact profile identity, selected request admission and publication remain
  mandatory. Selected Codex sessions retain their strict checkpoint validation.
  Persisted boundaries explicitly carrying another provider's checkpoint still
  require that metadata owner's validation; newly prepared boundaries do not
  acquire inactive providers' metadata.
  """

  Scenario: OpenAI API compaction ignores inactive Codex state after switching providers
    Given an OpenAI API selection after switching away from Codex with a newer transport generation
    When successive admitted compactions are committed and published
    Then each boundary retains the exact OpenAI API selection without Codex metadata
    And inactive Codex state is unchanged and inference opens

  Scenario: OpenAI API restore ignores an inactive Codex extension
    Given a committed OpenAI API profile boundary and a runtime previously using Codex
    When the runtime preflights and restores the committed boundary
    Then the OpenAI API generation is restored without requiring Codex metadata
    And inactive Codex state is unchanged and inference opens

  Scenario: Failed OpenAI API publication reconciles without invoking inactive Codex state
    Given an OpenAI API selection after switching away from Codex with a newer transport generation
    When admitted compaction is promoted but publication fails
    Then inference stays blocked until the exact durable identity is reconciled
    And reconciliation leaves inactive Codex state unchanged

  Scenario: Selected Codex compaction still rejects invalid checkpoint lineage
    Given a selected Codex runtime whose transport generation exceeds its durable generation
    When the runtime attempts an admitted compaction
    Then checkpoint lineage rejection prevents promotion and publication
    And ordinary inference remains blocked
