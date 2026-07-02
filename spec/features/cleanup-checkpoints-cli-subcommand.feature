@done
@RPC-203
@wip
Feature: cleanup-checkpoints CLI subcommand on the standalone fspec Rust binary
  """
  Two-front-doors invariant (RPC-003 §7/§11): the clap subcommand
  `fspec cleanup-checkpoints <work-unit-id> --keep-last <N>` and the LLM-facing dispatcher both
  route through codelet_fspec_core::commands::cleanup_checkpoints::run. The CLI bridge
  (codelet/fspec/src/cleanup_checkpoints.rs) parses the positional and --keep-last, validates that
  keepLast is a positive integer, marshals them into JSON, and resolves project_root from the
  current working directory. No list/sort/delete/render logic is duplicated in the bridge.

  Help parity: `fspec cleanup-checkpoints --help` (NO_COLOR, non-TTY) is byte-for-byte identical to
  codelet/fspec/tests/fixtures/help/cleanup-checkpoints.txt.
  """

  Background: User Story
    As a AI agent or developer using the fspec Rust binary
    I want to invoke cleanup-checkpoints from the shell exactly as the agent loop does
    So that the two front doors share one implementation

  Scenario: Clap exposes cleanup-checkpoints with a required --keep-last flag
    Given the fspec Rust binary has been compiled
    When I run "fspec cleanup-checkpoints --help"
    Then the command exits 0
    And stdout describes the cleanup-checkpoints subcommand
    And stdout advertises the "--keep-last" option

  Scenario: CLI cleans up and exits 0
    Given a git repository with several checkpoints for "AUTH-001" is the current working directory
    When I run "fspec cleanup-checkpoints AUTH-001 --keep-last 1" from that directory
    Then the command exits 0
    And stdout contains "✓ Cleanup complete:"

  Scenario: CLI rejects a non-positive --keep-last
    Given a git repository is the current working directory
    When I run "fspec cleanup-checkpoints AUTH-001 --keep-last 0" from that directory
    Then the command exits 1
    And stderr contains "--keep-last must be a positive number"

  Scenario: CLI rejects a non-numeric --keep-last
    Given a git repository is the current working directory
    When I run "fspec cleanup-checkpoints AUTH-001 --keep-last abc" from that directory
    Then the command exits 1
    And stderr contains "--keep-last must be a positive number"

  Scenario: Default combined TUI mode is preserved after adding cleanup-checkpoints
    Given the fspec Rust binary registers cleanup-checkpoints alongside the existing subcommands
    When I run "fspec --help"
    Then the command exits 0
    And the help output lists "cleanup-checkpoints" as an available subcommand

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a git repository with several checkpoints for "AUTH-001"
    When I dispatch cleanup-checkpoints through fspec_core::dispatch::dispatch_command with format "json"
    Then the dispatcher result succeeds and reports deletedCount and preservedCount
    And the CLI bridge module codelet/fspec/src/cleanup_checkpoints.rs contains NO inline list, sort, delete, or rendering logic — its only computation is arg parsing and JSON marshalling

  Scenario: cleanup-checkpoints --help is byte-for-byte identical to TS
    Given the fspec Rust binary has been compiled
    When I run "fspec cleanup-checkpoints --help" piped to non-TTY with NO_COLOR set
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/cleanup-checkpoints.txt
    And stdout starts with a blank line followed by "CLEANUP-CHECKPOINTS"
