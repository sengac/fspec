@TUI-133
@BUG-698
@high
@tui
@providers
@oauth
@provider-settings
@BUG-634
Feature: Daemon Anthropic browser sign-in
  """
  The TUI opens the authorization URL on its own computer, not the daemon.
  Anthropic browser sign-in preserves the browser-hosted paste form from
  1b19dc9f2^, with its listener on the client and token exchange on the daemon.
  Sign in with code remains a separate TUI paste flow.
  Credentials remain owned by the connected daemon. Codex keeps its automatic
  client-local callback flow. Generation checks precede launching.
  BUG-698: the tests wait for the browser form by dispatching the app's actions
  under the deadlock guard and await the launch task; no polling or 5 s timeouts.
  """

  Background: User Story
    As a user connected to a local or remote daemon
    I want to sign in using the browser on my computer
    So that transport choice does not prevent Anthropic subscription login

  Scenario: Anthropic browser sign-in opens on the client and submits to the daemon
    Given Provider Settings is connected to a daemon
    When I select Anthropic Sign in with browser
    Then my browser opens the original authorization link and code paste form
    When I submit a code with the wrong state through the browser form
    Then the login reports the state mismatch rather than unsupported login

  Scenario: Code sign-in remains available without automatically opening a browser
    Given Provider Settings is connected to a daemon
    When I select Anthropic Sign in with code
    Then the authorization URL and code field appear without opening a browser
    And Codex also offers browser sign-in

  Scenario: Browser launch failure leaves a usable code sign-in screen
    Given Provider Settings is connected to a daemon and my browser cannot open
    When I select Anthropic Sign in with browser
    Then the launch error explains that I can copy the URL
    When I copy the URL and visit it manually
    Then the clipboard contains the URL and the browser paste form is usable

  Scenario: Cancelling browser sign-in ignores its late authorization URL
    Given Provider Settings is connected to a daemon
    When I start Anthropic browser sign-in and cancel before it is ready
    And the cancelled authorization URL arrives
    Then no browser opens and Provider Settings stays on the list
