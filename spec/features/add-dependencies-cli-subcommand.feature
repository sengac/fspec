@done
@RPC-176
Feature: fspec add-dependencies CLI subcommand

  """
  CLI bridge: codelet/fspec/src/add_dependencies.rs — clap-derived struct mirroring TS Commander.js registration (src/commands/add-dependencies.ts:85-125).
  Surface: `fspec add-dependencies <workUnitId> [--blocks <ids...>] [--blocked-by <ids...>] [--depends-on <ids...>] [--relates-to <ids...>]`.
  Stdout (success): chalk.green '✓ Added <n> dependencies successfully' (ANSI tolerated by tests via substring match).
  Stderr (failure): '✗ Failed to add dependencies: <message>' prefixed line; exit code 1.
  Two-front-doors invariant: bridge marshals clap args to JSON `{workUnitId, dependencies: {blocks, blockedBy, dependsOn, relatesTo}}` and forwards to commands::add_dependencies::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js add-dependencies --help`.
  """

  Background: User Story
    As a fspec maintainer working on RPC-003 Rust port
    I want the standalone Rust fspec binary's add-dependencies subcommand to parse the same flags as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec add-dependencies --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-dependencies.txt

  Scenario: Multi-flag invocation marshalls all four arrays into the JSON args
    Given a project root tempdir with AUTH-001, AUTH-002, AUTH-003, FOO-001 all status=backlog and empty dependency arrays
    When I run `fspec add-dependencies AUTH-001 --blocks AUTH-002 AUTH-003 --depends-on FOO-001` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Added 3 dependencies successfully'
    And spec/work-units.json on disk shows AUTH-001.blocks=['AUTH-002', 'AUTH-003']
    And spec/work-units.json on disk shows AUTH-001.dependsOn=['FOO-001']
    And spec/work-units.json on disk shows AUTH-002.blockedBy contains 'AUTH-001' and AUTH-002.status='blocked'

  Scenario: Missing source work unit exits 1 with the canonical error message on stderr
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=backlog
    When I run `fspec add-dependencies NOPE-001 --blocks AUTH-001` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "Work unit 'NOPE-001' does not exist"

  Scenario: Self-dependency exits 1 with canonical message
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    When I run `fspec add-dependencies AUTH-001 --blocks AUTH-001` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'Cannot create self-dependency'

  Scenario: No flags supplied results in zero-added success
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    When I run `fspec add-dependencies AUTH-001` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Added 0 dependencies successfully'
    And spec/work-units.json on disk shows AUTH-001 with no blocks, no blockedBy, no dependsOn, no relatesTo fields
