@event-storming
@event-storm
@cli
@RPC-179
Feature: add-domain-event CLI subcommand
  """
  CLI bridge: rust/fspec/src/add_domain_event.rs — clap-derived struct mirroring TS Commander.js registration (src/commands/add-domain-event.ts). Surface: `fspec add-domain-event <workUnitId> <text> [--timestamp <ms>] [--bounded-context <ctx>]`.
  Stdout (success): '✓ Added domain event "<text>" to <workUnitId> (ID: <eventId>)' (chalk.green; ANSI tolerated via substring match).
  Stderr (failure): '✗ Failed to add domain event: <message>'; exit code 1.
  Two-front-doors invariant: bridge marshals positional/option args into JSON and forwards to commands::add_domain_event::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js add-domain-event --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-domain-event subcommand to parse the same positional arguments and options as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven Event Storm script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    given the fspec Rust binary is built and on PATH
    when I run `fspec add-domain-event --help`
    then the exit code is 0
    And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/add-domain-event.txt

  Scenario: CLI appends a domain event and prints the success line
    given a project root tempdir with spec/work-units.json containing RPC-179 status=specifying
    when I run `fspec add-domain-event RPC-179 "UserRegistered"` in that tempdir
    then the exit code is 0
    And stdout contains the substring '✓ Added domain event "UserRegistered" to RPC-179 (ID: 0)'
    And spec/work-units.json on disk shows RPC-179 eventStorm items has length 1

  Scenario: CLI rejects a duplicate event with exit 1 and TS-parity error prefix
    given a project root tempdir with spec/work-units.json containing RPC-179 status=specifying with a non-deleted event "UserRegistered" at id 0
    when I run `fspec add-domain-event RPC-179 "UserRegistered"` in that tempdir
    then the exit code is 1
    And stderr contains the substring '✗ Failed to add domain event:'
    And stderr contains the substring "Event 'UserRegistered' already exists (ID: 0)"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    given a project root tempdir with spec/work-units.json containing RPC-179 status=specifying
    when I dispatch add-domain-event via fspec_core::dispatch::dispatch_command with workUnitId='RPC-179' text='E1'
    then the dispatcher returns success=true
    And running `fspec add-domain-event RPC-179 "E2"` afterwards exits 0
    And spec/work-units.json on disk shows RPC-179 eventStorm items has length 2
    And the CLI bridge module rust/fspec/src/add_domain_event.rs contains NO inline event construction, dedup check, status guard, or file-write logic — its only computation is JSON arg marshalling
