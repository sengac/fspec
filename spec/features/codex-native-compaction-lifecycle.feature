@PROV-158
@providers
@provider-abstraction
@integration
Feature: Codex native compaction lifecycle
  """
  Explicit consent allows opaque successful checkpoints to replace replay
  history while preserving a separate user-history baseline. Failed responses
  never commit and local history replacement clears native continuation.
  """

  Scenario: Opaque compaction replaces history only at a successful opted-in boundary
    Given a Codex session explicitly negotiating native compaction
    When a successful response supplies an opaque compaction item
    Then continuation replaces the old replay prefix with the opaque checkpoint
    And a session reload preserves the checkpoint before new user input
    And neither presentation nor debug output reveals opaque content

  Scenario: Failed compaction and local replacement do not resurrect opaque history
    Given an opted-in Codex session with opaque continuation
    When native compaction fails or local history is replaced
    Then failed output is not committed and local replacement clears native state
    And capability-disabled and non-Codex history behavior remains unchanged
