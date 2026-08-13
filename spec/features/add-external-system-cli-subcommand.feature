@done
@RPC-182
Feature: fspec add-external-system CLI subcommand
  """
  CLI bridge: rust/fspec/src/add_external_system.rs — clap-derived struct mirroring TS Commander.js
  registration (src/commands/add-external-system.ts:77-129). Surface:
  `fspec add-external-system <workUnitId> <text> [--type <type>] [--timestamp <ms>] [--bounded-context <name>]`.
  Stdout (success): '✓ External system added to <workUnitId> (id: <id>)' (TS uses chalk.green; ANSI tolerated via substring match).
  Stderr (failure): '✗ Failed to add external system: <message>'; exit code 1. Mirrors TS output.error.
  Two-front-doors invariant: bridge marshals args into JSON {workUnitId, text, type?, timestamp?, boundedContext?}
  and forwards to commands::add_external_system::run — NO domain logic in the bridge. Note: the --type CLI
  flag is forwarded as the JSON `type` key, which the core maps to the item field `integrationType`.
  Help fixture captured from `node dist/index.js add-external-system --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-external-system subcommand to parse the same positional arguments and flags as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven Event Storming script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec add-external-system --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/add-external-system.txt
    And stdout starts with a blank line followed by 'ADD-EXTERNAL-SYSTEM'

  Scenario: CLI successfully appends an external system and prints the success line
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I run `fspec add-external-system AUTH-001 "Payment Gateway"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ External system added to AUTH-001 (id: 0)'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 1
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].text='Payment Gateway'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].type='external_system'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].color='pink'

  Scenario: CLI forwards the type flag into the integrationType item field
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I run `fspec add-external-system AUTH-001 "Stripe API" --type REST_API --bounded-context "Payments"` in that tempdir
    Then the exit code is 0
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].integrationType='REST_API'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].boundedContext='Payments'

  Scenario: CLI rejects a done-state work unit with exit 1 and the TS-parity error prefix
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=done
    When I run `fspec add-external-system AUTH-001 "Anything"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to add external system:'
    And stderr contains the substring "Cannot add Event Storm items to work unit in done state"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I dispatch add-external-system via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' text='S1'
    Then the dispatcher returns success=true
    And running `fspec add-external-system AUTH-001 "S2"` afterwards exits 0
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 2
    And the CLI bridge module rust/fspec/src/add_external_system.rs contains NO inline item construction, status guard, or file-write logic — its only computation is JSON arg marshalling
