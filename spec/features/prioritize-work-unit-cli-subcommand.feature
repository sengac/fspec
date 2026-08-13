@done
@rust
@work-management
@cli
@RPC-255
Feature: fspec prioritize-work-unit CLI subcommand
  """
  CLI bridge: rust/fspec/src/prioritize_work_unit.rs — clap-derived struct mirroring TS
  Commander.js registration (src/commands/prioritize-work-unit.ts:133-172). Surface:
  `fspec prioritize-work-unit <workUnitId> [--position <position>] [--before <id>] [--after <id>]`.

  Bridge owns ONLY: (a) coercing the --position string into the 'top'/'bottom' literal OR a numeric
  JSON value (parseInt parity); (b) JSON marshalling. All domain logic (existence, done guard,
  cross-column guard, data-integrity check, reordering, disk write) lives in
  fspec_core::commands::prioritize_work_unit::run.

  Stdout (success): '✓ Work unit <id> prioritized successfully'.
  Stderr (failure): '✗ Failed to prioritize work unit: <message>'; exit code 1.

  Help fixture captured from `node dist/index.js prioritize-work-unit --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-255 to Rust
    I want the standalone Rust fspec binary's prioritize-work-unit subcommand to parse the same positional + flag arguments as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven prioritization script keeps working after the cutover

  Scenario: Position top moves a work unit to the front of its column
    Given spec/work-units.json backlog order is AUTH-002, AUTH-003, AUTH-001
    When I run `fspec prioritize-work-unit AUTH-001 --position top`
    Then the process exits with code 0
    And the backlog order becomes AUTH-001, AUTH-002, AUTH-003

  Scenario: Numeric position is 1-based
    Given spec/work-units.json backlog order is AUTH-002, AUTH-003, AUTH-004, AUTH-001
    When I run `fspec prioritize-work-unit AUTH-001 --position 3`
    Then the process exits with code 0
    And the backlog order becomes AUTH-002, AUTH-003, AUTH-001, AUTH-004

  Scenario: Reject numeric position below 1
    Given spec/work-units.json backlog contains AUTH-001
    When I run `fspec prioritize-work-unit AUTH-001 --position 0`
    Then the process exits with code 1
    And stderr contains "Invalid position: 0. Position must be >= 1 (1-based index)"

  Scenario: Detect work unit missing from its own states array
    Given AUTH-001 has status specifying but is listed only in states.testing
    When I run `fspec prioritize-work-unit AUTH-001 --position top`
    Then the process exits with code 1
    And stderr contains "Data integrity error"
    And stderr contains "states.specifying"
    And stderr contains "fspec repair-work-units"

  Scenario: Reject cross-column relative placement
    Given FEAT-017 is in states.specifying and AUTH-001 is in states.testing
    When I run `fspec prioritize-work-unit FEAT-017 --before AUTH-001`
    Then the process exits with code 1
    And stderr contains "Data integrity error"

  Scenario: Reject prioritizing a non-existent work unit
    Given spec/work-units.json does not contain MISSING-999
    When I run `fspec prioritize-work-unit MISSING-999 --position top`
    Then the process exits with code 1
    And stderr contains "Work unit 'MISSING-999' does not exist"
    And spec/work-units.json is byte-identical to its pre-call content

  Scenario: Reject prioritizing a done work unit
    Given DONE-001 has status done and is in states.done
    When I run `fspec prioritize-work-unit DONE-001 --position top`
    Then the process exits with code 1
    And stderr contains "Cannot prioritize work units in done column"

  Scenario: Relative placement with before and after
    Given spec/work-units.json implementing order is AUTH-002, AUTH-001
    When I run `fspec prioritize-work-unit AUTH-001 --before AUTH-002`
    Then the process exits with code 0
    And the implementing order becomes AUTH-001, AUTH-002

  Scenario: CLI delegates to the same fspec_core function as the dispatcher
    Given the rust/fspec crate is built
    When I inspect rust/fspec/src/prioritize_work_unit.rs
    Then the source declares it calls codelet_fspec_core::commands::prioritize_work_unit::run
    And the source does NOT perform any file IO directly on spec/work-units.json

  Scenario: CLI help surface matches the captured TS fixture
    Given the TS help fixture at rust/fspec/tests/fixtures/help/prioritize-work-unit.txt
    When I run `fspec prioritize-work-unit --help`
    Then the process exits with code 0
    And stdout matches the captured TS fixture byte-for-byte
