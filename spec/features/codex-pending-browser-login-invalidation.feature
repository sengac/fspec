@high
@providers
@oauth
@authentication
@BUG-634
Feature: Codex pending browser login invalidation
  """
  Preserve the pre-rework LoginAttempt lifetime: the daemon starts its attempt
  before the client opens the browser and completion consumes that attempt.
  A logout or newer login invalidates pending authorization before exchange.
  """

  Background: User Story
    As a Codex user
    I want logout and newer login to invalidate older authorization
    So that a late browser callback cannot restore credentials I disconnected

  Scenario: Logout invalidates a pending browser authorization
    Given a daemon with isolated Codex credentials and a local token issuer
    When browser authorization starts and I log out before the callback
    Then completing that authorization is rejected without token exchange

  Scenario: A newer browser authorization invalidates the older attempt
    Given a daemon with isolated Codex credentials and a local token issuer
    When I start browser authorization twice
    Then the older authorization is rejected without token exchange
