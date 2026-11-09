@BUG-582
@RPC-447
@cli
@multi-workspace
Feature: Saved remote daemons
  """
  codelet_common::saved_daemons stores SavedDaemon { name, url, token } in ~/.fspec/daemons.json
  (atomic temp + rename, owner-only 0600 on unix). validate(url, token) allows ws:// only for
  loopback hosts (127.0.0.0/8, ::1, localhost) and requires a token for wss://; any other scheme
  is rejected. save replaces an entry with the same name; remove deletes by name.
  """

  Background: User Story
    As a developer who works against fspec daemons on other machines
    I want to save each remote daemon's address and token once
    So that clients can connect to it by name

  Scenario: Saving a remote daemon stores it owner-only
    Given an empty fspec home
    When the daemon "work" is saved with URL "wss://fspec.example.com" and token "s3cret"
    Then daemons.json lists "work" with that URL and token
    And daemons.json is readable and writable only by its owner

  Scenario: A plain ws URL to a non-loopback host is rejected
    Given an empty fspec home
    When the daemon "lan" is saved with URL "ws://10.0.0.5:9000"
    Then saving fails because remote daemons must use wss://
    And nothing is written

  Scenario: A wss URL without a token is rejected
    Given an empty fspec home
    When the daemon "work" is saved with URL "wss://fspec.example.com" and no token
    Then saving fails because remote daemons need a token

  Scenario: A loopback ws URL needs no token
    Given an empty fspec home
    When the daemon "local" is saved with URL "ws://127.0.0.1:9000" and no token
    Then daemons.json lists "local"

  Scenario: Removing a saved daemon
    Given saved daemons "work" and "local"
    When "work" is removed
    Then only "local" remains
