@done
@agent-core
@session-management
@persistence
@BUG-246
Feature: Refuse recovery admission when retained ownership cannot be verified
  """
  Uses the single recovery owner from recoverable-conversation-persistence.feature.
  Failed reads and invalid ordering coordinates are explicit repair-required
  states, never evidence of empty capacity or permission to invent source order.
  """

  Scenario: Unreadable recovery records never become empty admission capacity
    Given a retained recovery envelope whose durable record cannot be decoded
    When a caller attempts to admit another original envelope
    Then admission fails explicitly and the new envelope remains with its caller
    And repairing the existing record recovers only the previously accepted envelope

  Scenario: Corrupted recovery ordering is never replaced with an invented sequence
    Given a retained recovery envelope whose durable filename has lost its ordering coordinate
    When a caller attempts to admit another original envelope
    Then admission fails explicitly without accepting the new envelope or changing the retained record
