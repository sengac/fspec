@BUG-212
@done
@session
@session-management
@context-management
@compaction
Feature: Compaction switched provider replay
  """
  SessionManager restores the admitted boundary and its provider continuation as
  one selected-provider operation. An inactive provider extension must not be
  restored after preflight has correctly accepted another provider's boundary.
  Ordinary uncommitted replay retains its existing cross-provider validation.
  """

  Scenario: Full OpenAI API replay preserves selected checkpoint ownership
    Given a committed OpenAI API session retaining an inactive Codex extension
    When SessionManager repeatedly restores the authoritative replay
    Then the selected DAG and usage are restored without changing inactive Codex state
    And a fresh process can resume the same durable OpenAI API identity

  Scenario: Direct restore rejects corrupt owned provider checkpoints before mutation
    Given a committed boundary explicitly owning Codex checkpoint metadata
    And its retained replay contains a future checkpoint generation
    When the boundary is restored without a separate preflight call
    Then restore fails without changing messages tokens or checkpoint state
    And ordinary inference remains blocked
