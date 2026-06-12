@wip
@RPC-174
Feature: fspec add-command CLI subcommand

  """
  CLI bridge: codelet/fspec/src/add_command.rs — clap-derived struct mirroring the TS
  Commander.js registration (src/commands/add-command.ts:149-185). Surface:
  `fspec add-command <workUnitId> <text> [--actor <actor>] [--timestamp <ms>]
  [--bounded-context <context>]`.
  Stdout (success): '✓ Added command "<text>" to <workUnitId> (ID: <id>)' (TS uses chalk.green;
  ANSI tolerated via substring match). Stderr (failure): '✗ Failed to add command: <message>';
  exit code 1. Mirrors TS `output.error('✗ Failed to add command:', ...)`.
  Two-front-doors invariant: the bridge marshals positional args + options into JSON
  {workUnitId, text, actor?, timestamp?, boundedContext?} and forwards to
  commands::add_command::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js add-command --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-command subcommand to parse the same positional arguments and options as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven Event Storm script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec add-command --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-command.txt
    And stdout starts with a blank line followed by 'ADD-COMMAND'

  Scenario: CLI successfully appends a command and prints the success line
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I run `fspec add-command AUTH-001 "PlaceOrder" --actor "Customer"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Added command "PlaceOrder" to AUTH-001 (ID: 0)'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 1
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].text='PlaceOrder'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].actor='Customer'

  Scenario: CLI rejects a blocked work unit with exit 1 and the TS-parity error prefix
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=blocked
    When I run `fspec add-command AUTH-001 "Anything"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to add command:'
    And stderr contains the substring "Cannot add Event Storm items to work unit in blocked state"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I dispatch add-command via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' text='C1'
    Then the dispatcher returns success=true
    And running `fspec add-command AUTH-001 "C2"` afterwards exits 0
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 2
    And the CLI bridge module codelet/fspec/src/add_command.rs contains NO inline item construction, status guard, or file-write logic — its only computation is JSON arg marshalling
