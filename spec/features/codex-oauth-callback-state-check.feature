@done
@providers
@oauth
@authentication
@BUG-697
Feature: Codex OAuth callback checks state before any outcome
  """
  Architecture notes:
  - rust/providers/src/codex/oauth_callback.rs handle_request validates the state of every /auth/callback request against the pending login's state before reading code or error; a missing or different state answers 400 with the CSRF page and leaves the pending result untouched.
  - The client listener (codex_browser_authorization.rs, hosted by the TUI) and the daemon-hosted listener (codex_oauth_server.rs) both serve that handler, so one check covers both.
  - /cancel is unchanged: it mirrors the Codex CLI's port-release route and carries no state.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every /auth/callback request is checked against the pending login's state before anything else, error callbacks included: a callback with a missing or different state answers 400 with the CSRF page, and the login keeps waiting for the valid callback
  #   2. An error callback that carries the matching state ends the login with the authorization error, as before
  #   3. The check lives in the shared Codex callback handler (oauth_callback.rs), so it covers both the client (TUI-hosted) listener and the daemon-hosted codex_oauth_server listener
  #   4. /cancel is out of scope: it mirrors the Codex CLI's port-release route and carries no state (reported to the coordinator, not changed here)
  #
  # EXAMPLES:
  #   1. Client listener: /auth/callback?error=access_denied with no state, then with state=wrong, each answers 400 and the authorization keeps waiting; the legitimate code callback then completes it
  #   2. Daemon listener: /auth/callback?error=access_denied&state=wrong answers 400, nothing is exchanged or persisted, and the login keeps waiting; /auth/callback?error=access_denied&state=<expected> ends it with the access_denied error
  #
  # ========================================
  Background: User Story
    As a developer signing in to Codex with the browser
    I want to have the local callback listener ignore any callback whose state does not match my pending login
    So that a forged request from another local page cannot cancel or fail my sign-in

  Scenario: The client listener ignores error callbacks without the login's state
    Given a client callback listener with a fresh authorization URL
    When the browser returns an error callback without a state and then one with a different state
    Then each answers 400 with the CSRF page and the authorization keeps waiting
    And the legitimate callback with the login's state still completes it

  Scenario: The daemon listener ignores an error callback with a different state
    Given the daemon-hosted OAuth server is waiting for its callback with a known state
    When an error callback arrives with a different state
    Then it answers 400 with the CSRF page, nothing is exchanged or persisted and the login keeps waiting
    And an error callback with the login's state ends the login with that authorization error
