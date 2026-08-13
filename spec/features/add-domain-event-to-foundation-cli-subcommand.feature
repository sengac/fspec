@done
@event-storm
@cli
@RPC-180
Feature: fspec add-domain-event-to-foundation CLI subcommand
  """
  CLI bridge: rust/fspec/src/add_domain_event_to_foundation.rs — clap-derived struct mirroring the
  TS Commander.js registration (src/commands/add-domain-event-to-foundation.ts:138-161). Surface:
  `fspec add-domain-event-to-foundation <context-name> <event-name> [--description <text>]`.
  Stdout (success): '✓ Added domain event "<event-name>" to "<context-name>" bounded context' (TS uses
  output.log('✓', message); ANSI tolerated via substring match). Stderr (failure): 'Error: <message>';
  exit code 1. Mirrors TS output.error(chalk.red('Error:'), message). Two-front-doors invariant: the
  bridge marshals args into JSON {contextName, eventName, description?} and forwards to fspec_core
  commands::add_domain_event_to_foundation::run — NO domain logic in the bridge. Help fixture captured
  from `node dist/index.js add-domain-event-to-foundation --help`. FOUNDATION.md is regenerated after
  the write (matches the event-storm twins; supervisor ruling 2026-06-13).
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-domain-event-to-foundation subcommand to parse the same positional arguments and flags as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven Event Storming script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec add-domain-event-to-foundation --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/add-domain-event-to-foundation.txt

  Scenario: CLI successfully appends a domain event and prints the success line
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I run `fspec add-domain-event-to-foundation "Work Management" "WorkUnitCreated"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Added domain event "WorkUnitCreated" to "Work Management" bounded context'
    And spec/foundation.json on disk shows eventStorm.items gained an event item with text='WorkUnitCreated' and boundedContextId=0

  Scenario: CLI forwards the --description flag into the persisted item
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I run `fspec add-domain-event-to-foundation "Work Management" "WorkUnitCreated" --description "Signals work unit reached done status"` in that tempdir
    Then the exit code is 0
    And spec/foundation.json on disk shows the appended event item description='Signals work unit reached done status'

  Scenario: CLI rejects a missing bounded context with exit 1 and the TS-parity error prefix
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I run `fspec add-domain-event-to-foundation "Nope" "WorkUnitCreated"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring "Bounded context 'Nope' not found"
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I dispatch add-domain-event-to-foundation via fspec_core::dispatch::dispatch_command with contextName='Work Management' eventName='E1'
    Then the dispatcher returns success=true
    And running `fspec add-domain-event-to-foundation "Work Management" "E2"` afterwards exits 0
    And spec/foundation.json on disk shows eventStorm.items contains both event items 'E1' and 'E2'
    And the CLI bridge module rust/fspec/src/add_domain_event_to_foundation.rs contains NO inline item construction, context lookup, or file-write logic — its only computation is JSON arg marshalling
