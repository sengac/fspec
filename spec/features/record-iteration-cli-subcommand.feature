@done
@RPC-264
Feature: Port record-iteration command to Rust

  """
  Core impl at codelet/fspec-core/src/commands/record_iteration.rs: signature pub async fn run(args_json:&str, project_root:&Path)->Result<String,FspecCoreError>; direct std::fs::read_to_string of spec/work-units.json (no auto-create), mutate iterations in WorkUnit.extra, bump updated_at via io::time::iso8601_now, write back via io::locked_file::write_json_atomic (2-space pretty, preserve_order)
  CLI bridge codelet/fspec/src/record_iteration.rs marshals name/start/end clap fields but per Framing A passes no workUnitId, so core returns 'Work unit undefined not found'; help config codelet/fspec-core/src/help/configs/record_iteration.rs mirrors record-iteration-help.ts. SUPERVISOR wires canonical.rs/dispatch.rs/commands.mod.rs/help configs.mod.rs/main.rs
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The core run function reads spec/work-units.json directly without auto-creating it (mirrors TS readFile, not ensureWorkUnitsFile)
  #   2. When the target work unit exists, iterations is set to (existing iterations or 0) + 1 and updatedAt is bumped to the current ISO-8601 timestamp
  #   3. When the target work unit does not exist, the command fails with the wrapped message 'Failed to record iteration: Work unit <id> not found'
  #   4. The mutated work-units.json is written back with 2-space-indent pretty JSON preserving original field and insertion order
  #   5. On success the core function returns the JSON envelope {"success": true, "iterations": <n>}
  #   6. Framing A: the TS CLI action passes name/start/end and never wires workUnitId, so the shell record-iteration subcommand always fails with 'Work unit undefined not found' and exit code 1; the Rust CLI bridge mirrors this broken behaviour while the dispatcher implements the useful function contract
  #   7. The --help output is byte-for-byte identical to node dist/index.js record-iteration --help (rendered via the Rust CommandHelpConfig)
  #
  # EXAMPLES:
  #   1. Dispatcher: a work unit AUTH-001 with no iterations field gets iterations incremented to 1 and the function returns {success:true,iterations:1}
  #   2. Dispatcher: a work unit AUTH-001 with iterations:3 gets incremented to 4 and updatedAt is refreshed
  #   3. Dispatcher: recording an iteration for a missing work unit MISSING-999 returns an error containing 'Failed to record iteration: Work unit MISSING-999 not found'
  #   4. CLI: running 'fspec record-iteration "Sprint 1"' exits 1 with stderr containing 'Failed to record iteration' and 'Work unit undefined not found' (Framing A broken-CLI parity)
  #   5. CLI: 'fspec record-iteration --help' prints the formatted help matching the captured fixture
  #   6. Two-front-doors: the CLI bridge and the dispatcher both call commands::record_iteration::run with identical JSON args and produce identical results
  #
  # ========================================

  Background: User Story
    As a fspec maintainer
    I want to record an iteration increment on a work unit via the Rust-ported record-iteration command
    So that the standalone fspec binary tracks iteration counts with TS-parity behaviour through both the dispatcher and the CLI

  Scenario: CLI record-iteration always fails per Framing A broken-CLI parity
    Given a workspace with a valid spec/work-units.json
    When I run "fspec record-iteration Sprint-1" in that workspace
    Then the command exits with code 1
    And stderr contains "Failed to record iteration"
    And stderr contains "Work unit undefined not found"

  Scenario: CLI record-iteration --help matches the TS formatCommandHelp reference
    When I run "fspec record-iteration --help"
    Then stdout is byte-for-byte identical to the captured help fixture
    And the command exits with code 0
