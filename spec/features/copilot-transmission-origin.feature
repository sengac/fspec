@done
@BUG-268
@providers
@session-management
Feature: Copilot transmission origin
  """
  Compare actual transport-time authentication and origin, not only preparation.
  Preserve the approved existing refresh coordinator with no inference retries.
  """

  Background: User Story
    As a Copilot user
    I want opaque ownership checked at actual transmission
    So that refreshed deployment routing cannot replay another origin context

  Scenario: Origin changes during actual transmission invalidate replay before sending
    Given a live Copilot session owns opaque before its next request refreshes
    When the existing refresh coordinator changes the API origin after model preparation but before transmission
    Then the new origin receives fresh authorization without the former origin opaque
    And a compatible follow up retains only opaque settled at the new origin
