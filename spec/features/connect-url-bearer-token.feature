@BUG-582
@done
@BUG-429
@cli
@multi-workspace
@rust
Feature: --connect accepts a bearer token for URL daemons
  """
  fspec/src/daemons_cmd.rs resolve_connect connected a ws:// or wss:// URL with
  no token (only a saved daemon carried one), there was no --token flag, and
  daemon.json autodiscovery in client.rs and status.rs also connected without
  one, so a token-protected ad-hoc or local daemon could not be reached
  without saving it first. fspec client and fspec status take --token;
  resolve_connect(value, token) uses --token, else FSPEC_DAEMON_TOKEN, for a
  URL, and --token, else the saved token, for a saved daemon; autodiscovery
  uses --token, else FSPEC_DAEMON_TOKEN.
  Tests: rust/fspec/src/daemons_cmd/tests.rs.
  """

  Background: User Story
    As a user connecting to an ad-hoc daemon by URL
    I want to pass the daemon's bearer token
    So that a token-protected daemon is reachable without saving it first

  Scenario: A URL connection carries the --token bearer
    Given a --connect URL and a --token "s3cret"
    When the connection target is resolved
    Then the bearer is "s3cret"

  Scenario: A URL connection without --token uses FSPEC_DAEMON_TOKEN
    Given a --connect URL with no --token
    And FSPEC_DAEMON_TOKEN is "from-env"
    When the connection target is resolved
    Then the bearer is "from-env"
