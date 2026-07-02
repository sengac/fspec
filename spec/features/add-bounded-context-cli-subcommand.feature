@done
@RPC-172
Feature: fspec add-bounded-context CLI subcommand
  """
  CLI bridge: codelet/fspec/src/add_bounded_context.rs — clap-derived struct mirroring TS Commander.js
  registration (src/commands/add-bounded-context.ts:72-122). Surface:
  `fspec add-bounded-context <workUnitId> <text> [--description <scope>] [--timestamp <ms>] [--bounded-context <name>]`.
  Stdout (success): '✓ Bounded context added to <workUnitId> (id: <id>)' (TS uses chalk.green; ANSI tolerated via substring match).
  Stderr (failure): '✗ Failed to add bounded context: <message>'; exit code 1. Mirrors TS output.error.
  Two-front-doors invariant: bridge marshals args into JSON {workUnitId, text, description?, timestamp?, boundedContext?}
  and forwards to commands::add_bounded_context::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js add-bounded-context --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-bounded-context subcommand to parse the same positional arguments and flags as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven Event Storming script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec add-bounded-context --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-bounded-context.txt
    And stdout starts with a blank line followed by 'ADD-BOUNDED-CONTEXT'

  Scenario: CLI successfully appends a bounded context and prints the success line
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I run `fspec add-bounded-context AUTH-001 "Order Management"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Bounded context added to AUTH-001 (id: 0)'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 1
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].text='Order Management'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].type='bounded_context'

  Scenario: CLI forwards the description and bounded-context flags into the persisted item
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I run `fspec add-bounded-context AUTH-001 "Inventory" --description "Manages stock" --bounded-context "Logistics"` in that tempdir
    Then the exit code is 0
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].description='Manages stock'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].boundedContext='Logistics'

  Scenario: CLI rejects a done-state work unit with exit 1 and the TS-parity error prefix
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=done
    When I run `fspec add-bounded-context AUTH-001 "Anything"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to add bounded context:'
    And stderr contains the substring "Cannot add Event Storm items to work unit in done state"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I dispatch add-bounded-context via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' text='C1'
    Then the dispatcher returns success=true
    And running `fspec add-bounded-context AUTH-001 "C2"` afterwards exits 0
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 2
    And the CLI bridge module codelet/fspec/src/add_bounded_context.rs contains NO inline item construction, status guard, or file-write logic — its only computation is JSON arg marshalling
