@done
@RPC-177
Feature: fspec add-dependency CLI subcommand

  """
  CLI bridge: codelet/fspec/src/add_dependency.rs — clap-derived struct mirroring TS Commander.js registration
  (src/commands/add-dependency.ts:256-321). Surface: `fspec add-dependency <id> [dependsOnId] [--blocks <id>]
  [--blocked-by <id>] [--depends-on <id>] [--relates-to <id>]`.

  Bridge owns ONLY: (a) resolving positional shorthand → dependsOn; (b) conflict-rejecting
  `<id> <B> --depends-on <C>` when B!=C; (c) rejecting invocations with no relationship at all;
  (d) JSON marshalling. All domain logic (existence, self-dep, duplicate, cycle, bidirectional
  edges, auto-transition) lives in fspec_core::commands::add_dependency::run.

  Stdout (success): '✓ Dependency added successfully'.
  Stderr (failure): '✗ Failed to add dependency: <message>'; exit code 1.

  Help fixture captured from `node dist/index.js add-dependency --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-dependency subcommand to parse the same positional + flag arguments as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven dependency-management script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec add-dependency --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-dependency.txt
    And stdout starts with a blank line followed by 'ADD-DEPENDENCY'

  Scenario: CLI successfully adds shorthand dependsOn and prints the success line
    Given a project root tempdir with spec/work-units.json containing AUTH-001 and AUTH-002 both status=specifying
    When I run `fspec add-dependency AUTH-002 AUTH-001` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Dependency added successfully'
    And spec/work-units.json on disk shows AUTH-002.dependsOn=['AUTH-001']

  Scenario: CLI successfully adds --blocks edge
    Given a project root tempdir with spec/work-units.json containing AUTH-001 and API-001 both status=specifying
    When I run `fspec add-dependency AUTH-001 --blocks API-001` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Dependency added successfully'
    And spec/work-units.json on disk shows AUTH-001.blocks=['API-001']
    And spec/work-units.json on disk shows API-001.status='blocked'

  Scenario: CLI rejects invocation with no relationship args
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I run `fspec add-dependency AUTH-001` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to add dependency:'
    And stderr contains the substring 'Must specify at least one relationship'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: CLI rejects conflict between shorthand and --depends-on
    Given a project root tempdir with spec/work-units.json containing AUTH-001, AUTH-002, AUTH-003 all status=specifying
    When I run `fspec add-dependency AUTH-001 AUTH-002 --depends-on AUTH-003` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to add dependency:'
    And stderr contains the substring 'Cannot specify dependency both as argument and --depends-on option'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: CLI rejects circular blocks with exit 1
    Given a project root tempdir with spec/work-units.json containing AUTH-001 with blocks=['AUTH-002'] and AUTH-002 blockedBy=['AUTH-001']
    When I run `fspec add-dependency AUTH-002 --blocks AUTH-001` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to add dependency:'
    And stderr contains the substring 'Circular dependency detected'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/work-units.json containing AUTH-001 and AUTH-002 both status=specifying
    When I dispatch add-dependency via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-002' dependsOn='AUTH-001'
    Then the dispatcher returns success=true
    And running `fspec add-dependency AUTH-002 AUTH-001` afterwards exits 1 with 'Dependency already exists'
    And the CLI bridge module codelet/fspec/src/add_dependency.rs contains NO inline edge-add, status guard, cycle, or file-write logic — its only computation is shorthand resolution + conflict pre-check + JSON arg marshalling
