@RPC-447
@cli
@multi-workspace
Feature: Managing saved daemons from the CLI
  """
  fspec daemons add <name> <url> [--token T] / list / remove <name> delegate to
  codelet_common::saved_daemons (list prints name and URL, masking tokens). fspec daemon reads
  FSPEC_DAEMON_TOKEN and arms the service; --connect on client/status accepts a saved daemon name
  and sends its token.
  """

  Background: User Story
    As a developer with several daemons
    I want to manage saved daemons and connect by name from the CLI
    So that I never paste URLs and tokens by hand

  Scenario: Adding and listing saved daemons masks the token
    Given an empty fspec home
    When "fspec daemons add work wss://fspec.example.com --token s3cret" and "fspec daemons list" are run
    Then the list shows "work" and its URL but not the token

  Scenario: Removing a saved daemon from the CLI
    Given a saved daemon "work"
    When "fspec daemons remove work" is run
    Then "fspec daemons list" no longer shows "work"

  Scenario: Status connects to a token-protected daemon by saved name
    Given a daemon started with FSPEC_DAEMON_TOKEN "s3cret" and saved as "local" with that token
    When "fspec status --connect local" is run
    Then the daemon reports alive
    And "fspec status" with a raw URL and no token fails
