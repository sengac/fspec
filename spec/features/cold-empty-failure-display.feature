@done
@session
@rpc
@persistence
@agent-view
@BUG-245
Feature: Cold empty failure display
  """
  Cold session display projects canonical archival pages through the shared
  Replay decoder. Private request anchors order failures but never become
  visible conversation or model input.
  """

  Scenario: Cold display preserves consecutive empty provider failures in request order
    Given two requests fail without assistant output before a third request succeeds
    When fresh session managers repeatedly restore the archived conversation
    Then each safe failure appears once after its input and before the next request
    And the successful response appears once without private anchors or failure prose in model history
