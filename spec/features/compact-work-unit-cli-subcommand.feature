@done
@RPC-206
@rust
@cli
@mutation
Feature: fspec compact-work-unit CLI subcommand (Rust port)
  """
  Clap derive subcommand `compact-work-unit` exposes the same surface as the TS Commander.js registration at src/commands/compact-work-unit.ts:155-203 — a required positional `<workUnitId>` plus an optional `--force` flag. The bridge module at rust/fspec/src/compact_work_unit.rs marshals workUnitId and force into a JSON object and delegates to codelet_fspec_core::commands::compact_work_unit::run.
  Exit codes: 0 on success, 1 on any FspecCoreError. Errors are written to stderr prefixed with '✗ Failed to compact work unit:' (parity with the chalk-red TS error path at src/commands/compact-work-unit.ts:196-201). Success prints either 'No deleted items to remove' OR '✓ Compacted work unit <id>' followed by '  Removed items:' and per-category lines.
  The `fspec compact-work-unit --help` output is byte-for-byte identical to `node dist/index.js compact-work-unit --help` (TS reference) — captured as rust/fspec/tests/fixtures/help/compact-work-unit.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `compact-work-unit` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes compact-work-unit with a positional arg and --force flag in --help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec compact-work-unit --help`
    Then the command exits 0
    And stdout describes the compact-work-unit subcommand
    And stdout mentions the `<workUnitId>` argument
    And stdout advertises the `--force` flag
    And stdout does NOT advertise a `--workspace` global flag

  Scenario: CLI compacts a done work unit and prints the removed-items summary
    Given spec/work-units.json contains work unit AUTH-001 with status='done' having 2 deleted rules and 1 live rule
    When I run `./rust/target/release/fspec compact-work-unit AUTH-001`
    Then the command exits 0
    And stdout contains the line '✓ Compacted work unit AUTH-001'
    And stdout contains the substring 'Rules: 2'

  Scenario: CLI prints the no-op sentinel when there is nothing to remove
    Given spec/work-units.json contains work unit AUTH-001 with status='done' having 2 live rules and no deleted items
    When I run `./rust/target/release/fspec compact-work-unit AUTH-001`
    Then the command exits 0
    And stdout contains the line 'No deleted items to remove'

  Scenario: CLI exits 1 when forcing is required but absent
    Given spec/work-units.json contains work unit AUTH-001 with status='specifying' having 1 deleted rule
    When I run `./rust/target/release/fspec compact-work-unit AUTH-001`
    Then the command exits with code 1
    And stderr contains the substring '✗ Failed to compact work unit:'
    And stderr contains the substring "Cannot compact work unit in 'specifying' status."

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/work-units.json contains work unit AUTH-001 with status='done' having 1 deleted rule
    When I dispatch compact-work-unit via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001'
    Then the dispatcher returns success=true
    And the AUTH-001 rules array in spec/work-units.json contains 0 items
    And the CLI bridge module rust/fspec/src/compact_work_unit.rs contains NO inline file-read, mutation, or rendering logic — its only computation is JSON arg marshalling

  Scenario: compact-work-unit --help is byte-for-byte identical to TS reference
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec compact-work-unit --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/compact-work-unit.txt
    And stdout starts with a blank line followed by 'COMPACT-WORK-UNIT'
