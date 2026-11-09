@BUG-685
@wip
@BUG-582
@high
@tui
@providers
@oauth
@provider-settings
@BUG-634
Feature: Client Codex browser sign-in
  """
  The TUI owns the Codex loopback listener and opens the client browser.
  The connected backend exchanges the callback code and PKCE verifier.
  Cancelling aborts the listener task; generation fencing rejects late URLs.
  BUG-685: the tests bind an ephemeral callback address through the test-support
  configuration, never the real port 1455, and wait on events, not fixed deadlines.
  """

  Background: User Story
    As a user signing in to Codex
    I want browser authorization on my own computer
    So that daemon connections retain automatic browser sign-in

  Scenario: Browser authorization URL can be copied and reopened
    Given Codex browser authorization is ready
    When I press c and Ctrl+C and o
    Then the complete URL is copied twice and reopened

  Scenario: Browser authorization URL and launch errors wrap
    Given Codex browser authorization is ready but the browser cannot open
    When I view the waiting screen in a narrow terminal
    Then the complete URL and launch failure remain visible across lines

  Scenario: Cancelling browser authorization closes the callback listener
    Given Codex browser authorization is waiting for a local callback
    When I cancel sign-in and a late ready event arrives
    Then the callback port is released and no further browser opens

  Scenario: Daemon browser sign-in receives an automatic callback error
    Given Codex browser sign-in is connected to a WebSocket daemon
    When the browser redirects to the local callback with an authorization error
    Then the TUI reports the authorization error without asking for a pasted code
