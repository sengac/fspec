@done
@RPC-271
Feature: fspec remove-dependency CLI subcommand

  """
  CLI bridge: codelet/fspec/src/remove_dependency.rs — clap-derived struct mirroring TS Commander.js registration (src/commands/remove-dependency.ts:133-200).
  Surface: `fspec remove-dependency <workUnitId> [dependsOnId] [--blocks <id>] [--blocked-by <id>] [--depends-on <id>] [--relates-to <id>]`.
  Shorthand: positional [dependsOnId] equals --depends-on. If both supplied with DIFFERENT values, exit 1 with the canonical conflict message; if same value, succeed without error.
  At-least-one guard: if no relationship arg supplied (after shorthand reconciliation), exit 1 with the canonical 'Must specify at least one relationship to remove' message.
  Stdout (success): '✓ Dependency removed successfully' (singular — NOT pluralised by count, unlike add-dependencies).
  Stderr (failure): '✗ Failed to remove dependency: <message>' prefixed line; exit code 1.
  Two-front-doors invariant: bridge marshals clap args to JSON `{workUnitId, blocks?, blockedBy?, dependsOn?, relatesTo?}` (singular string fields, NOT arrays) and forwards to commands::remove_dependency::run — NO domain logic in the bridge other than the shorthand-reconciliation and at-least-one guards.
  Help fixture captured from `node dist/index.js remove-dependency --help`.
  """

  Background: User Story
    As a fspec maintainer working on RPC-003 Rust port
    I want the standalone Rust fspec binary's remove-dependency subcommand to parse the same flags as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec remove-dependency --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/remove-dependency.txt

  Scenario: Positional shorthand removes the dependsOn edge
    Given a project root tempdir with spec/work-units.json where AUTH-001.dependsOn=['AUTH-002']
    When I run `fspec remove-dependency AUTH-001 AUTH-002` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Dependency removed successfully'
    And spec/work-units.json on disk shows AUTH-001 has no dependsOn field

  Scenario: --depends-on flag removes the same edge as the positional shorthand
    Given a project root tempdir with spec/work-units.json where AUTH-001.dependsOn=['AUTH-002']
    When I run `fspec remove-dependency AUTH-001 --depends-on AUTH-002` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Dependency removed successfully'
    And spec/work-units.json on disk shows AUTH-001 has no dependsOn field

  Scenario: Positional and --depends-on with the same value succeed without conflict
    Given a project root tempdir with spec/work-units.json where AUTH-001.dependsOn=['AUTH-002']
    When I run `fspec remove-dependency AUTH-001 AUTH-002 --depends-on AUTH-002` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Dependency removed successfully'

  Scenario: Positional and --depends-on with different values exits 1 with conflict message
    Given a project root tempdir with spec/work-units.json where AUTH-001.dependsOn=['AUTH-002']
    When I run `fspec remove-dependency AUTH-001 AUTH-002 --depends-on AUTH-003` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'Cannot specify dependency both as argument and --depends-on option'

  Scenario: No relationship args supplied exits 1 with the at-least-one guard message
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    When I run `fspec remove-dependency AUTH-001` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'Must specify at least one relationship to remove'

  Scenario: --blocks flag removes a blocks edge bidirectionally
    Given a project root tempdir with spec/work-units.json where AUTH-001.blocks=['AUTH-002'] and AUTH-002.blockedBy=['AUTH-001']
    When I run `fspec remove-dependency AUTH-001 --blocks AUTH-002` in that tempdir
    Then the exit code is 0
    And spec/work-units.json on disk shows AUTH-001 has no blocks field
    And spec/work-units.json on disk shows AUTH-002 has no blockedBy field

  Scenario: --blocked-by flag removes a blockedBy edge bidirectionally
    Given a project root tempdir with spec/work-units.json where UI-001.blockedBy=['API-001'] and API-001.blocks=['UI-001']
    When I run `fspec remove-dependency UI-001 --blocked-by API-001` in that tempdir
    Then the exit code is 0
    And spec/work-units.json on disk shows UI-001 has no blockedBy field
    And spec/work-units.json on disk shows API-001 has no blocks field

  Scenario: --relates-to flag removes a symmetric relatesTo edge
    Given a project root tempdir with spec/work-units.json where AUTH-002.relatesTo=['AUTH-003'] and AUTH-003.relatesTo=['AUTH-002']
    When I run `fspec remove-dependency AUTH-002 --relates-to AUTH-003` in that tempdir
    Then the exit code is 0
    And spec/work-units.json on disk shows AUTH-002 has no relatesTo field
    And spec/work-units.json on disk shows AUTH-003 has no relatesTo field

  Scenario: Missing source work unit exits 1 with the canonical error on stderr
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=backlog
    When I run `fspec remove-dependency NOPE-001 --depends-on AUTH-001` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "Work unit 'NOPE-001' does not exist"
