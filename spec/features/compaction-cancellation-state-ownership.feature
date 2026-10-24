@done
@BUG-235
@phase-1
@agent-core
@context-management
@compaction
Feature: Compaction cancellation retains current turn state ownership
  """
  Retry and continuation streams reuse the current turn's shared token-state
  allocation. Refreshing usage cannot erase pending compaction intent. Typed
  PromptCancelled remains authoritative, including nested streaming errors;
  unrelated errors and independent sessions retain their existing semantics.
  User interruption remains a separate terminal path. Already-set intent is
  expected when recovery follows a threshold hook, not another disagreement.
  """

  Scenario: Retry threshold cancellation agrees with current turn recovery
    Given a current turn and an independent session with separate token states
    When a refreshed retry stream crosses its compaction threshold
    Then nested typed cancellation recovers without a missing-intent disagreement
    And the independent session remains unchanged

  Scenario: Refreshing retry usage preserves pending compaction intent
    Given the current turn already requires compaction
    When retry or continuation usage is refreshed
    Then the shared allocation and pending intent remain intact

  Scenario: Missing intent still receives defensive typed recovery
    Given a typed cancellation whose current turn intent was reset
    When recovery classifies the cancellation
    Then intent is repaired and the mismatch remains observable
    And unrelated errors do not authorize compaction
