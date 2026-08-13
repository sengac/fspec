@done
@event-storm
@cli
@RPC-272
Feature: fspec remove-domain-event-from-foundation CLI subcommand
  """
  CLI bridge: rust/fspec/src/remove_domain_event_from_foundation.rs — clap-derived struct mirroring
  the TS Commander.js registration (src/commands/remove-domain-event-from-foundation.ts:131-153).
  Surface: `fspec remove-domain-event-from-foundation <context-name> <event-name>` (NO options).
  Stdout (success): '✓ Removed domain event "<event-name>" from "<context-name>" bounded context' (TS
  uses output.log('✓', message); ANSI tolerated via substring match). Stderr (failure):
  'Error: <message>'; exit code 1. Mirrors TS output.error(chalk.red('Error:'), message).
  Two-front-doors invariant: the bridge marshals args into JSON {contextName, eventName} and forwards
  to fspec_core commands::remove_domain_event_from_foundation::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js remove-domain-event-from-foundation --help`.
  FOUNDATION.md is regenerated after the write (matches the event-storm twins; supervisor ruling
  2026-06-13).
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's remove-domain-event-from-foundation subcommand to parse the same positional arguments as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven Event Storming script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec remove-domain-event-from-foundation --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/remove-domain-event-from-foundation.txt

  Scenario: CLI successfully soft-deletes a domain event and prints the success line
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and an event text='WorkUnitCreated' boundedContextId=0 deleted=false
    When I run `fspec remove-domain-event-from-foundation "Work Management" "WorkUnitCreated"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Removed domain event "WorkUnitCreated" from "Work Management" bounded context'
    And spec/foundation.json on disk shows the WorkUnitCreated event item deleted=true

  Scenario: CLI rejects a missing event with exit 1 and the TS-parity error prefix
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and an event text='WorkUnitCreated' boundedContextId=0
    When I run `fspec remove-domain-event-from-foundation "Work Management" "Ghost"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring "Domain event 'Ghost' not found in bounded context 'Work Management'"
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and events text='E1' and text='E2' both boundedContextId=0
    When I dispatch remove-domain-event-from-foundation via fspec_core::dispatch::dispatch_command with contextName='Work Management' eventName='E1'
    Then the dispatcher returns success=true
    And running `fspec remove-domain-event-from-foundation "Work Management" "E2"` afterwards exits 0
    And spec/foundation.json on disk shows both event items E1 and E2 with deleted=true
    And the CLI bridge module rust/fspec/src/remove_domain_event_from_foundation.rs contains NO inline context lookup, event match, or file-write logic — its only computation is JSON arg marshalling
