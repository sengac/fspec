@done
@BUG-211
@phase-1
@agent-core
@context-management
@compaction
Feature: Compaction explicit retry safety
  """
  A fresh user turn may restart only a failed pre-commit builder. Cancellation
  and subsequent publication failures revoke that permission without clearing
  the inference guard or modifying the durable conversation boundary.
  """

  Scenario: Cancelling an explicit builder retry retains the safety block
    Given fresh user input has restarted a failed builder
    When the user cancels the pending summary request
    Then no summary is committed and ordinary inference does not resume
    And another user message cannot restart the cancelled operation

  Scenario: A later publication failure overrides builder retry permission
    Given automatic compaction failed before committing a summary
    And an unresolved publication failure subsequently blocks inference
    When a fresh user message arrives
    Then neither a builder nor ordinary inference starts
