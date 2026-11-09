@done
@integration
@test-maintenance
@bug-fix
@rust
@BUG-627
Feature: Test daemons never touch the real runtime dir
  """
  Culprit: rust/fspec/tests/common::spawn_fspec_daemon (and spawn_fspec_combined) spawned fspec daemon with the test process's HOME and XDG_RUNTIME_DIR; used by bug539_corrupt_board_daemon_startup (temp-dir workspace, as in the incident), daemon_mode, client_mode, cross_frontend_parity, daemon_lifecycle_rpc011 and cargo_shape. Also unisolated: ad-hoc daemon spawns in daemon_mode, client_mode (643, 677), cross_frontend_parity 705, cargo_shape/fspec_version_prints_the, combined_smoke (4 combined spawns), scrollback_pty_rpc078 (PTY combined mode); stale_daemon_json_rpc011, status_subcommand_rpc011 and daemon_lifecycle_rpc011 isolate XDG_RUNTIME_DIR but keep the real HOME (daemon logs and ~/.fspec data). Only the fspec binary writes discovery files (fspec/src/daemon.rs, combined.rs via common::claim_default_pointer / write_daemon_entry). Harness: tests/common gains a TestHome (temp root with home/ and runtime/), a check over Command::get_envs / CommandBuilder::get_env, and checked spawn helpers; spawn_fspec_daemon/spawn_fspec_combined take the TestHome.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A test that starts fspec as a daemon (fspec daemon) or in combined mode (the TUI with its in-process daemon) gives it a HOME and an XDG_RUNTIME_DIR inside the system temp dir (where the test's temp dirs live), or removes XDG_RUNTIME_DIR so discovery falls under that temp HOME
  #   2. The test harness checks the command's environment before spawning: HOME not set by the test or set outside the temp dir, or XDG_RUNTIME_DIR inherited (neither set nor removed), empty, or set outside the temp dir, refuses the spawn with an error naming the variable and its value; no daemon is started
  #   3. Every daemon, combined-mode and PTY spawn of the fspec binary in rust/fspec/tests goes through the checked harness; a source scan of rust/fspec/tests names file:line of any fspec daemon or PTY spawn built outside it
  #
  # EXAMPLES:
  #   1. fspec daemon spawned with the test process's environment untouched is refused: XDG_RUNTIME_DIR is inherited from the test process
  #   2. HOME set to the user's real home is refused naming HOME and the path
  #   3. spawn_fspec_daemon with a test home: the daemon's daemons/<pid>.json and daemon.json are under <temp>/runtime/fspec and nothing is written to the real runtime dir
  #   4. HOME in one temp dir and XDG_RUNTIME_DIR in another (client_mode), or HOME in a temp dir with XDG_RUNTIME_DIR removed, is accepted
  #   5. A test file building Command::new(fspec_bin()).arg("daemon") and spawning it directly is reported by the source scan with its file:line
  #
  # ========================================
  Background: User Story
    As a developer running fspec's test suites on my own machine
    I want every test that starts an fspec daemon to use its own temporary HOME and runtime dir, and the harness to refuse one that does not
    So that my real daemon.json, discovery entries and ~/.fspec data are never replaced or polluted by a test daemon

  Scenario: A daemon spawn that inherits the test process's runtime dir is refused
    Given an "fspec daemon" command whose environment the test left untouched
    When the harness checks it before spawning
    Then it refuses naming XDG_RUNTIME_DIR as inherited from the test process
    And no daemon process is started

  Scenario: A daemon spawn with HOME outside the temp dir is refused
    Given an "fspec daemon" command with XDG_RUNTIME_DIR in a temp dir and HOME set to the user's real home
    When the harness checks it before spawning
    Then it refuses naming HOME and that path

  Scenario: An isolated test daemon writes its discovery files under the test's temp dir
    Given a test home with home and runtime dirs inside a temp dir
    When spawn_fspec_daemon starts a daemon with that test home
    Then the daemon's daemons/<pid>.json and daemon.json are under the test home's runtime dir

  Scenario: Separate temp dirs, or a temp HOME with XDG_RUNTIME_DIR removed, are accepted
    Given an "fspec daemon" command with HOME and XDG_RUNTIME_DIR in two different temp dirs, and another with a temp HOME and XDG_RUNTIME_DIR removed
    When the harness checks them before spawning
    Then both are accepted

  Scenario: Every daemon and PTY spawn in the fspec tests goes through the harness
    Given the integration test sources under rust/fspec/tests
    When they are scanned for fspec daemon and PTY spawns built outside the checked harness
    Then none is found
    And a spawn of Command::new(fspec_bin()).arg("daemon") outside the harness would be reported with its file and line
