@BUG-214
@phase-1
@agent-core
@compaction
@context-management
Feature: Submitted compaction remains cancellable during admission
  """
  A queued summary is not a durable generation. Deferred application must check
  the target interruption state after selected request serialization as well as
  before admission, sharing the guarded commit used by automatic compaction.
  """

  Scenario: Deferred application vetoes cancellation during replacement admission
    Given a valid summary is queued for its captured session
    When the target is interrupted while the replacement request is serialized
    Then application returns cancellation without changing durable context
    And no completion is emitted and inference remains blocked

  Scenario: A cross-target caller can cancel replacement admission
    Given a caller generates a valid summary for another live session
    When the caller is interrupted while the target replacement request is serialized
    Then generation returns cancellation without changing the target durable context
    And the target emits no compaction completion
