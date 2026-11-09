@providers
@oauth
@authentication
@BUG-634
Feature: Codex browser callback stays on the client
  """
  Architecture: a client-owned listener reuses the Codex callback handler.
  Browser state is random and independent of PKCE. After local validation,
  the client sends code#verifier and verifier through the existing RPC.
  Only the daemon exchanges tokens and commits the login lifecycle.
  The daemon starts the lifecycle before issuing the browser URL and retains
  that original attempt and five-minute deadline until completion or cancellation.
  """

  Scenario: A valid local callback returns authorization material without storing tokens
    Given a client callback listener with a fresh authorization URL
    When the browser returns a code with the matching random state
    Then completion returns code and verifier for the daemon
    And the browser state differs from the verifier and no credentials are written

  Scenario: A malicious callback cannot complete the login
    Given a client callback listener with a fresh authorization URL
    When a callback uses a different state
    Then it is rejected and the listener still accepts the legitimate callback

  Scenario: Cancelling the browser authorization releases its listener
    Given a client callback listener with a fresh authorization URL
    When the browser cancels authorization
    Then completion reports cancellation and the listener is released

  Scenario: Timing out or dropping authorization releases its listener
    Given a client callback listener with a bounded authorization deadline
    When authorization times out or its owning future is dropped
    Then the listener is released without storing credentials

  Scenario: The daemon exchanges the validated code and commits tokens
    Given a daemon with isolated credentials and a local token issuer
    When Codex completion receives a code with matching verifier state
    Then the issuer receives the code and fixed localhost callback redirect
    And the daemon persists tokens and their expiry through the login lifecycle

  Scenario: The daemon rejects invalid completion state before exchanging tokens
    Given a daemon with isolated credentials and a local token issuer
    When Codex completion receives missing or mismatched verifier state
    Then no token request or credential write occurs

  Scenario: The client binds the authorization issued by the daemon
    Given the daemon issued a URL with independent state and a matching PKCE verifier
    When the client receives the matching browser callback
    Then it returns the code with that original verifier without replacing the authorization

  Scenario: Browser authorization expires at its original five-minute deadline
    Given a daemon has started Codex browser authorization
    When five minutes pass without a callback
    Then completion is rejected without token exchange and another login can start
