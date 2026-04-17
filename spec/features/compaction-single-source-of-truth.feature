@done
@error-handling
@resilience
@cli
@CMPCT-026
Feature: Eliminate fragile `&& compaction_triggered` guard — single source of truth

  """
  Structural detection of PromptCancelled in the error chain is the authoritative gate for compaction recovery. The TokenState.compaction_needed flag is defense-in-depth — a warning is logged when the two signals disagree, and the flag is defensively set when PromptCancelled fires without it. Unrelated errors with the flag set bypass recovery and propagate normally (with a warning). Mirrored across stream_loop and gemini_continuation cancel sites.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. PromptCancelled in the error chain is authoritative: presence routes to recovery regardless of the TokenState flag
  #   2. When PromptCancelled is present but compaction_needed is false, recovery still runs and the flag is defensively set with a warning logged
  #   3. When compaction_needed is true but the error is not PromptCancelled, recovery does NOT run and a warning is logged
  #   4. The gemini_continuation cancel-recovery path mirrors the same single-source-of-truth policy
  #
  # EXAMPLES:
  #   1. Both signals agree (PromptCancelled present AND compaction_needed=true) — recovery runs, partial state flushed, no warning
  #   2. PromptCancelled with compaction_needed=false — recovery runs, warning logged, flag set as defense-in-depth
  #   3. Unrelated I/O error with compaction_needed=true — falls through to normal error handling, warning logged but no recovery
  #
  # ========================================

  Background: User Story
    As a session orchestrator
    I want to detect a PromptCancelled error structurally as the single source of truth for compaction recovery
    So that a degraded TokenState flag can never silently terminate a recoverable session

  Scenario: PromptCancelled with flag true routes to recovery
    Given the stream has yielded a PromptCancelled error wrapped in the rig StreamingError chain
    When the stream loop classifies the error
    Then the loop breaks into the compaction recovery path
    Given the shared token state has compaction_needed set to true
    Then no disagreement warning is logged


  Scenario: PromptCancelled with flag false still recovers
    Given the stream has yielded a PromptCancelled error wrapped in the rig StreamingError chain
    When the stream loop classifies the error
    Then the loop breaks into the compaction recovery path
    Given the shared token state has compaction_needed set to false
    Then the compaction_needed flag is set to true as defense-in-depth
    Then a warning is emitted that the two signals disagree


  Scenario: Unrelated error with flag true does not trigger recovery
    Given the stream has yielded an error that is NOT a PromptCancelled variant
    When the stream loop classifies the error
    Then the compaction-cancel branch is not taken
    Given the shared token state has compaction_needed set to true
    Then a warning is emitted that the flag was set without a PromptCancelled error
    Then the error continues to the normal error classifier cascade

