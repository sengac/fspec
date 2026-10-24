@done
@session
@rpc
@tui
@persistence
@agent-view
@BUG-244
Feature: Active session first resume
  """
  Manual resume of a live session continues its existing authoritative sources.
  It does not replace live model messages, counters, or transcript generation
  with a second restoration of already installed originals.
  """

  Scenario: First resume of an active session preserves runtime sources
    Given an active real session whose persisted originals already exist in its runtime journal and model history
    When the session is manually resumed for the first time while a tab holds its cursor
    Then the runtime generation transcript order model messages and counters remain unchanged
    And intentional identical messages remain separate events and the saved cursor continues with new output
