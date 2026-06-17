@done
@RPC-202 @wip
Feature: checkpoint CLI subcommand on the standalone fspec Rust binary

  """
  Two-front-doors invariant (RPC-003 §7/§11): the clap subcommand
  `fspec checkpoint <work-unit-id> <checkpoint-name>` and the LLM-facing dispatcher both
  route through codelet_fspec_core::commands::checkpoint::run. The CLI bridge
  (codelet/fspec/src/checkpoint.rs) only marshals the two positionals into JSON and resolves
  project_root from the current working directory (parity with the TS process.cwd() default).
  No capture, index, or rendering logic is duplicated in the bridge.

  Help parity: `fspec checkpoint --help` (NO_COLOR, non-TTY) is byte-for-byte identical to the
  captured TS fixture at codelet/fspec/tests/fixtures/help/checkpoint.txt.
  """

  Background: User Story
    As a AI agent or developer using the fspec Rust binary
    I want to invoke checkpoint from the shell exactly as the agent loop does
    So that the two front doors share one implementation

  Scenario: Clap exposes checkpoint as a subcommand with two positionals
    Given the fspec Rust binary has been compiled
    When I run "fspec checkpoint --help"
    Then the command exits 0
    And stdout describes the checkpoint subcommand
    And stdout does NOT contain the substring "--workspace"

  Scenario: CLI creates a checkpoint and exits 0
    Given a git repository with uncommitted changes is the current working directory
    When I run "fspec checkpoint AUTH-001 baseline" from that directory
    Then the command exits 0
    And stdout contains "✓ Created checkpoint \"baseline\" for AUTH-001"
    And stdout contains "Captured"

  Scenario: CLI exits 1 when the working tree is clean
    Given a git repository with no uncommitted changes is the current working directory
    When I run "fspec checkpoint AUTH-001 baseline" from that directory
    Then the command exits 1

  Scenario: Default combined TUI mode is preserved after adding checkpoint
    Given the fspec Rust binary registers checkpoint alongside the existing subcommands
    When I run "fspec --help"
    Then the command exits 0
    And the help output lists "checkpoint" as an available subcommand
    And the long-about still documents the combined TUI default

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a git repository with uncommitted changes
    When I dispatch checkpoint through fspec_core::dispatch::dispatch_command with format "json"
    Then the dispatcher result succeeds and reports capturedFiles
    And the CLI bridge module codelet/fspec/src/checkpoint.rs contains NO inline capture, index-write, or rendering logic — its only computation is JSON arg marshalling

  Scenario: checkpoint --help is byte-for-byte identical to TS
    Given the fspec Rust binary has been compiled
    When I run "fspec checkpoint --help" piped to non-TTY with NO_COLOR set
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/checkpoint.txt
    And stdout starts with a blank line followed by "CHECKPOINT"
