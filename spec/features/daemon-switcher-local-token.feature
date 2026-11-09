@BUG-582
@done
@BUG-428
@cli
@multi-workspace
@rust
Feature: The daemon switcher authenticates to token-protected local daemons
  """
  rust/fspec/src/daemon_directory.rs connect looked up a bearer token only for
  saved daemons; local discovery entries (common::daemon_registry, one
  <pid>.json per daemon) connected as ws://127.0.0.1:{port} with no token, so
  a local daemon started with FSPEC_DAEMON_TOKEN (daemon.rs, RPC-447) refused
  the switch. Local entries now use the client's FSPEC_DAEMON_TOKEN (the same
  variable the daemon reads); a non-Unicode value is an error, and a protected
  daemon reached without a token reports the refusal.
  Tests: rust/fspec/src/daemon_directory/tests.rs.
  """

  Background: User Story
    As a user running a token-protected local daemon
    I want to switch to it from the TUI daemon switcher
    So that local daemons started with FSPEC_DAEMON_TOKEN are reachable like saved ones

  Scenario: A local daemon is reached with the FSPEC_DAEMON_TOKEN bearer
    Given a local daemon that requires bearer "s3cret"
    And FSPEC_DAEMON_TOKEN is "s3cret" in the client
    When the switcher connects to the local daemon entry
    Then the connection succeeds

  Scenario: A protected local daemon refuses a switcher without a token
    Given a local daemon that requires bearer "s3cret"
    And FSPEC_DAEMON_TOKEN is not set in the client
    When the switcher connects to the local daemon entry
    Then the connection fails
