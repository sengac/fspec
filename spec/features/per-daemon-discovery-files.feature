@RPC-446
@BUG-289
@cli
@multi-workspace
Feature: Per-daemon local discovery files
  """
  rust/fspec/src/common/daemon_registry.rs: daemons_dir() is the `daemons/` directory beside
  daemon.json ($XDG_RUNTIME_DIR/fspec or ~/.fspec). write_daemon_entry(dir, port, pid, workspace,
  started_at) writes <pid>.json with the daemon.json schema (atomic temp + rename);
  remove_daemon_entry(dir, pid) deletes it; list_live_daemons(dir) parses every entry, keeps the
  ones whose pid is alive and removes the rest. daemon.rs and combined.rs register the entry next
  to daemon.json at startup and remove it on shutdown. RPC-448's switcher consumes the list.
  """

  Background: User Story
    As a developer running several fspec daemons on one machine
    I want each daemon to advertise itself in its own discovery file
    So that a client can list every local daemon, not only the last one started

  Scenario: A daemon entry is written per process
    Given an empty daemons directory
    When daemon entries are written for two live processes with workspaces "alpha" and "beta"
    Then the directory holds one <pid>.json per process with its port and workspace

  Scenario: Listing returns live daemons and prunes stale entries
    Given a daemons directory with an entry for the current process and one for a dead pid
    When the live daemons are listed
    Then only the current process's daemon is returned
    And the dead pid's file has been removed

  Scenario: Removing an entry deletes only that daemon's file
    Given daemon entries for two live processes
    When one daemon removes its entry on shutdown
    Then only the other daemon's file remains

  Scenario: A running daemon registers itself beside daemon.json
    Given a runtime directory for discovery files
    When fspec daemon starts in a workspace
    Then daemons/<pid>.json names the daemon's port and workspace
    And daemon.json still points at the daemon

  Scenario: fspec status lists every live local daemon
    Given a running fspec daemon advertised in the daemons directory
    When fspec status is run against the same runtime directory
    Then the output lists the daemon with its pid, port and workspace

  Scenario: A second daemon does not replace a live daemon's daemon.json
    Given daemon.json points at a live daemon
    When another daemon starts and claims the default daemon.json
    Then daemon.json still points at the first daemon
    And the second daemon reports that it did not take the default

  Scenario: A dead daemon's daemon.json is replaced
    Given daemon.json points at a process that is no longer running
    When a daemon starts and claims the default daemon.json
    Then daemon.json points at the new daemon

  Scenario: Shutting down hands daemon.json to another live daemon
    Given daemon.json points at this daemon and another live daemon is listed in the daemons directory
    When this daemon shuts down and releases the default daemon.json
    Then daemon.json points at the other live daemon

  Scenario: Shutting down leaves another daemon's daemon.json alone
    Given daemon.json points at another live daemon
    When this daemon shuts down and releases the default daemon.json
    Then daemon.json still points at the other daemon
