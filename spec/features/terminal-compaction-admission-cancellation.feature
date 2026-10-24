@BUG-214
@phase-1
@agent-core
@compaction
@context-management
Feature: Terminal compaction respects cancellation during admission
  """
  Terminal overflow recovery shares the automatic builder's cancellation-aware
  commit path. A cancellation arriving inside selected request serialization
  must veto promotion, not merely stop the next awaited provider response.
  """

  Scenario: Terminal recovery vetoes cancellation during replacement admission
    Given terminal recovery has a valid builder candidate and unchanged durable history
    When the user interrupts while the replacement request is serialized
    Then terminal recovery returns cancellation without a promoted generation
    And original context remains available without compaction completion
