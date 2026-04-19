@done
@session-management
@compaction
@tui
@bug-fix
@CMPCT-034
Feature: Compaction indicator uses local React state instead of Rust source of truth — persists across session switches

  """
  Rust is the source of truth — SessionStatus::Compacting and CompactionProgress are already set per-session in Rust; useRustSessionState already reads them; no new NAPI calls needed
  Follow the existing pattern: isLoading comes from rustSnapshot.isLoading, isPaused from rustSnapshot.isPaused, isDebugEnabled from rustSnapshot.isDebugEnabled — isCompacting must follow the same pattern
  Strip useCompaction (src/tui/hooks/useCompaction.ts) — remove UnifiedCompactionState, startCompaction, endCompaction, updateProgress, progress polling useEffect; keep only performManualCompaction + retry state
  Remove compactionRef from AgentView — it was only needed for stream handlers to call startCompaction/endCompaction, which are being removed
  Remove Compacting branch from persistentSessionStateHandler — the handler only needs Cleared + the catch-all refreshRustState; remove startCompaction/getCompactionProgress from SessionStateChangeDeps
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The MultiLineInput isCompacting prop must read from rustSnapshot.isCompacting (Rust source of truth via useRustSessionState) — not from local React useState in useCompaction
  #   2. The MultiLineInput compactionProgress prop must read from rustSnapshot.compactionProgress (Rust source of truth) — not from local React useState
  #   3. useCompaction must not maintain isActive/progress/trigger/sessionId state — it must only manage manual compaction operations (performManualCompaction) and retry dialog state
  #   4. Stream chunk handlers must not call startCompaction or endCompaction — Rust already sets SessionStatus::Compacting and Idle; refreshRustState() propagates to the UI via useRustSessionState
  #   5. persistentSessionStateHandler must not handle the Compacting state branch — the handler only needs Cleared logic and the catch-all refreshRustState call
  #
  # EXAMPLES:
  #   1. Session B is compacting; user navigates to Session A via Shift+Left — the Compacting indicator disappears immediately because rustSnapshot reads Session A's status from Rust (not compacting)
  #   2. Session B compacts in the background; user navigates back to Session B — the Compacting indicator appears because rustSnapshot now reads Session B's Compacting status from Rust
  #   3. Manual /compact command still works — performManualCompaction calls sessionCompact via NAPI, Rust sets Compacting status, rustSnapshot picks it up, user sees the Compacting indicator
  #   4. Compaction retry dialog still works after /compact fails — useCompaction retains retryState management independently of the Compacting display state
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to see the Compacting indicator only on the session that is actually compacting
    So that I am not confused by a stale Compacting placeholder following me across session switches

  Scenario: Switching away from a compacting session clears the indicator
    Given Session B is compacting in the background
    And I am currently viewing Session B with the Compacting indicator visible
    When I navigate to Session A via Shift+Left
    Then the Compacting indicator is not visible on Session A
    And the input placeholder shows the normal prompt text

  Scenario: Switching back to a compacting session shows the indicator
    Given Session B is compacting in the background
    And I am currently viewing Session A which is not compacting
    When I navigate to Session B via Shift+Right
    Then the Compacting indicator is visible on Session B
    And the compaction progress is displayed

  Scenario: Manual /compact command shows the indicator via Rust state
    Given I am viewing Session A which is idle
    When I run the /compact command on Session A
    Then Rust sets Session A's status to Compacting
    And the Compacting indicator appears on Session A
    And the compaction progress updates as Rust reports progress

  Scenario: Compaction retry dialog works independently of display state
    Given I am viewing Session A
    When I run the /compact command and it fails
    Then the retry dialog appears with the error message
    And I can choose to retry, continue, or cancel
    And the retry dialog state is managed by useCompaction independently of the Compacting display
