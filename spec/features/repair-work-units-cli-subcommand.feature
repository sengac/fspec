@done
@rust
@work-management
@cli
@RPC-284
Feature: fspec repair-work-units CLI subcommand
  """
  CLI bridge: codelet/fspec/src/repair_work_units.rs — clap-derived struct mirroring TS
  Commander.js registration (src/commands/repair-work-units.ts:128-151). Surface:
  `fspec repair-work-units [--dry-run]`.

  Bridge owns ONLY: (a) marshalling the optional {dryRun} flag into JSON; (b) parsing the core
  result's `repaired` count for the success line. All repair logic (state-index rebuild,
  bidirectional-link repair, disk write) lives in fspec_core::commands::repair_work_units::run.

  The --dry-run flag is forwarded but has NO effect — the core always writes (preserving the TS
  parity bug where dryRun is never read by the implementation).

  Stdout (success): '✓ Repaired <n> issues'.
  Stderr (failure): '✗ Failed to repair work units: <message>'; exit code 1.

  Help fixture captured from `node dist/index.js repair-work-units --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-284 to Rust
    I want the standalone Rust fspec binary's repair-work-units subcommand to parse the same flags as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven repair script keeps working after the cutover

  Scenario: Dry-run still writes the rebuilt states
    Given AUTH-001 has status specifying but is listed only in states.testing
    When I run `fspec repair-work-units --dry-run`
    Then the process exits with code 0
    And stdout contains "✓ Repaired 1 issues"
    And states.specifying contains AUTH-001 on disk

  Scenario: CLI repairs a corrupted file and reports the count
    Given AUTH-001 has status specifying but is listed only in states.testing
    When I run `fspec repair-work-units`
    Then the process exits with code 0
    And stdout contains "✓ Repaired 1 issues"

  Scenario: CLI delegates to the same fspec_core function as the dispatcher
    Given the codelet/fspec crate is built
    When I inspect codelet/fspec/src/repair_work_units.rs
    Then the source declares it calls codelet_fspec_core::commands::repair_work_units::run
    And the source does NOT perform any file IO directly on spec/work-units.json

  Scenario: CLI help surface matches the captured TS fixture
    Given the TS help fixture at codelet/fspec/tests/fixtures/help/repair-work-units.txt
    When I run `fspec repair-work-units --help`
    Then the process exits with code 0
    And stdout matches the captured TS fixture byte-for-byte
