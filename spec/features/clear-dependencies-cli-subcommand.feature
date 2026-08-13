@done
@RPC-204
Feature: fspec clear-dependencies CLI subcommand
  """
  CLI bridge: rust/fspec/src/clear_dependencies.rs — clap-derived struct mirroring TS Commander.js registration (src/commands/clear-dependencies.ts:101-123).
  Surface: `fspec clear-dependencies <workUnitId> --confirm`.
  Stdout (success): chalk.green '✓ All dependencies cleared from <workUnitId>' (ANSI tolerated by tests via substring match).
  Stderr (failure): '✗ Failed to clear dependencies: <message>' prefixed line; exit code 1.
  Two-front-doors invariant: bridge marshals clap args to JSON `{workUnitId, confirm}` and forwards to commands::clear_dependencies::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js clear-dependencies --help`.
  """

  Background: User Story
    As a fspec maintainer porting the TypeScript implementation to Rust
    I want the standalone Rust fspec binary's clear-dependencies subcommand to parse the same flags as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec clear-dependencies --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/clear-dependencies.txt

  Scenario: Invocation with --confirm wipes every dependency edge and reports success
    Given a project root tempdir with AUTH-001 having blocks=['AUTH-002'] dependsOn=['API-001'] relatesTo=['UI-001'], AUTH-002.blockedBy=['AUTH-001'], API-001, UI-001.relatesTo=['AUTH-001']
    When I run `fspec clear-dependencies AUTH-001 --confirm` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ All dependencies cleared from AUTH-001'
    And spec/work-units.json on disk shows AUTH-001 has no blocks, blockedBy, dependsOn, or relatesTo fields
    And spec/work-units.json on disk shows AUTH-002 has no blockedBy field
    And spec/work-units.json on disk shows UI-001 has no relatesTo field

  Scenario: Missing --confirm flag exits 1 with the canonical error message on stderr
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog with blocks=['AUTH-002']
    When I run `fspec clear-dependencies AUTH-001` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'Must confirm clearing all dependencies with --confirm flag'

  Scenario: Missing source work unit exits 1 with the canonical error message on stderr
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=backlog
    When I run `fspec clear-dependencies UNKNOWN-001 --confirm` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "Work unit 'UNKNOWN-001' does not exist"
