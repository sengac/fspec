@done
@BUG-430
@cli
@multi-workspace
@rust
Feature: Combined mode has the daemon switcher
  """
  rust/fspec/src/client.rs installed the daemon directory with
  app.set_daemon_directory, but combined mode (combined.rs: in-process daemon
  plus TUI) never did, so the board's s daemon switcher did nothing there.
  daemon_directory::install_daemon_switcher(app, url, saved_name) is now the
  one place both modes install the switcher; combined mode passes its own
  ws://127.0.0.1:<port>.
  Tests: rust/fspec/src/daemon_directory/tests.rs.
  """

  Background: User Story
    As a user running fspec in combined mode
    I want to open the daemon switcher
    So that I can move to another daemon without restarting in client mode

  Scenario: The switcher opens with the in-process daemon connected
    Given an app with the daemon switcher installed for its own local daemon
    When the daemon switcher is opened
    Then it opens and names the in-process daemon as connected
