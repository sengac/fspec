@done
@BUG-243
@phase-1
@persistence
@context-management
@compaction
Feature: Separate provider continuation state from conversation originals
  """
  Version 1 provider-state sidecars retain the latest native checkpoint per
  provider generation. Durable atomic replacement is serialized by the existing
  session writer. An optional versioned manifest reference and prefix identity
  anchor state without adding a conversation position. Replay verifies that
  anchor and supplies native metadata without manufacturing dialogue.
  Legacy mixed manifests remain immutable: their checkpoint positions return
  explicit versioned provider_state identity markers, not original dialogue.
  Migration adds a reference idempotently; no original or archived generation is deleted.
  Retained sidecar space is bounded by one latest payload per generation, not
  by the sum of every full-history snapshot; at most a displaced root and staging
  file accompany the current root. Staging files are never readable
  checkpoints. Promotion failures are explicit, including post-rename failures.
  """

  Background: User Story
    As an agent recovering conversation evidence and provider continuation
    I want provider checkpoints separated from bounded conversation originals
    So that native continuation and stable original coordinates survive recovery

  Scenario: Persist real provider checkpoints without conversation amplification
    Given a real provider persistence binder and structured tool dialogue
    When repeated native checkpoints are persisted through the binder
    Then bounded SessionSearch originals traverses only the unchanged dialogue
    And retained state contains one latest checkpoint per provider generation
