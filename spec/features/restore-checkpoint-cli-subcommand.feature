@done
@RPC-288 @wip
Feature: restore-checkpoint CLI subcommand on the standalone fspec Rust binary

  """
  Two-front-doors invariant (RPC-003 §7/§11): the clap subcommand
  `fspec restore-checkpoint <work-unit-id> <checkpoint-name>` and the LLM-facing dispatcher both
  route through codelet_fspec_core::commands::restore_checkpoint::run. The CLI bridge
  (codelet/fspec/src/restore_checkpoint.rs) only marshals the two positionals into JSON and
  resolves project_root from the current working directory. The clap surface carries NO --force
  or --user-choice flags (parity with the TS Commander.js registration). No dirty-check,
  conflict-detection, restore, or rendering logic is duplicated in the bridge.

  Help parity: `fspec restore-checkpoint --help` (NO_COLOR, non-TTY) is byte-for-byte identical to
  codelet/fspec/tests/fixtures/help/restore-checkpoint.txt.
  """

  Background: User Story
    As a AI agent or developer using the fspec Rust binary
    I want to invoke restore-checkpoint from the shell exactly as the agent loop does
    So that the two front doors share one implementation

  Scenario: Clap exposes restore-checkpoint as a subcommand with two positionals
    Given the fspec Rust binary has been compiled
    When I run "fspec restore-checkpoint --help"
    Then the command exits 0
    And stdout describes the restore-checkpoint subcommand
    And stdout does NOT contain the substring "--force"

  Scenario: CLI restores against a clean working tree and exits 0
    Given a git repository with a checkpoint "baseline" for "AUTH-001" and a clean working tree is the current working directory
    When I run "fspec restore-checkpoint AUTH-001 baseline" from that directory
    Then the command exits 0
    And stdout contains "✓ Restored checkpoint \"baseline\" for AUTH-001"

  Scenario: CLI exits 1 with the re-run hint when the working tree is dirty
    Given a git repository with a checkpoint "baseline" for "AUTH-001" and uncommitted changes is the current working directory
    When I run "fspec restore-checkpoint AUTH-001 baseline" from that directory
    Then the command exits 1
    And stdout contains "Re-run with user choice to proceed with restoration"

  Scenario: CLI exits 1 when the checkpoint does not exist
    Given a git repository with no checkpoint named "ghost" for "AUTH-001" is the current working directory
    When I run "fspec restore-checkpoint AUTH-001 ghost" from that directory
    Then the command exits 1

  Scenario: Default combined TUI mode is preserved after adding restore-checkpoint
    Given the fspec Rust binary registers restore-checkpoint alongside the existing subcommands
    When I run "fspec --help"
    Then the command exits 0
    And the help output lists "restore-checkpoint" as an available subcommand

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a git repository with a checkpoint "baseline" for "AUTH-001" and a clean working tree
    When I dispatch restore-checkpoint through fspec_core::dispatch::dispatch_command with format "json"
    Then the dispatcher result succeeds and reports conflictsDetected
    And the CLI bridge module codelet/fspec/src/restore_checkpoint.rs contains NO inline dirty-check, conflict-detection, restore, or rendering logic — its only computation is JSON arg marshalling

  Scenario: restore-checkpoint --help is byte-for-byte identical to TS
    Given the fspec Rust binary has been compiled
    When I run "fspec restore-checkpoint --help" piped to non-TTY with NO_COLOR set
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/restore-checkpoint.txt
    And stdout starts with a blank line followed by "RESTORE-CHECKPOINT"
