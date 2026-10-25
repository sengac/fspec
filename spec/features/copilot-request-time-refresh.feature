@done
@BUG-268
@providers
@testing
Feature: Copilot request time refresh
  """
  Actual requests reuse existing freshness, coalescing, persistence and error
  behavior. Fixtures require the separately proven numeric-loopback issuer seam,
  temporary synthetic credentials and mock keyring. No inference retries.
  """

  Background: User Story
    As a Copilot user
    I want actual inference requests to use coordinated fresh credentials
    So that stale construction state cannot send expired authorization

  Scenario: Concurrent independent requests share existing token refresh coordination
    Given independent Copilot sessions share expired synthetic authentication and a validated local issuer
    When both real nonstreaming front doors request completion concurrently
    Then one token exchange precedes both freshly authorized inference requests and persists synthetic credentials

  Scenario: Refreshed effective endpoints reach the actual nonstreaming request
    Given a Copilot model was built before a local issuer refresh changes its effective API origin
    When that public completion model sends its real request
    Then only the refreshed loopback origin receives fresh Authorization and the selected Responses dialect

  Scenario: Rejected or cancelled exchange cannot send expired authorization
    Given expired synthetic authentication and a rejecting or delayed local issuer
    When a real Copilot completion is rejected or cancelled during token exchange
    Then inference is never sent and no credential value reaches the returned diagnostic
