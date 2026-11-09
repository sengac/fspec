@done
@RPC-165
@BUG-532
@cli
@foundation-management
Feature: fspec add-aggregate CLI subcommand
  """
  CLI bridge: rust/fspec/src/add_aggregate.rs — clap-derived struct mirroring the TS
  Commander.js registration (src/commands/add-aggregate.ts:152-189). Surface:
  `fspec add-aggregate <workUnitId> <text> [--responsibilities <list>] [--timestamp <ms>]
  [--bounded-context <context>]`.

  Console output and exit code (BUG-532): on success the bridge prints
  '✓ Added aggregate "<text>" to <workUnitId> (ID: <aggregateId>)' on stdout and exits 0, like
  add-domain-event; on failure it prints '✗ Failed to add aggregate: <error>' on stderr and exits 1.
  The TypeScript CLI's logger.success bug (empty output and exit 1 even on success) is not kept.
  Two-front-doors invariant: the bridge marshals positional args + options into JSON
  {workUnitId, text, responsibilities?, timestamp?, boundedContext?} and forwards to
  commands::add_aggregate::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js add-aggregate --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-aggregate subcommand to parse the same positional arguments and options as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven Event Storm script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec add-aggregate --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/add-aggregate.txt
    And stdout starts with a blank line followed by 'ADD-AGGREGATE'

  Scenario: CLI persists the aggregate, prints the success line and exits 0
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I run `fspec add-aggregate AUTH-001 "Order" --responsibilities "Place,Cancel"` in that tempdir
    Then the exit code is 0
    And stdout is '✓ Added aggregate "Order" to AUTH-001 (ID: 0)' followed by a newline
    And stderr is empty
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 1
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].text='Order'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].responsibilities equals the array ["Place","Cancel"]

  Scenario: CLI rejects a done work unit with exit 1 and an error on stderr
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=done
    When I run `fspec add-aggregate AUTH-001 "Anything"` in that tempdir
    Then the exit code is 1
    And stdout is empty
    And stderr starts with '✗ Failed to add aggregate: '
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I dispatch add-aggregate via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' text='A1'
    Then the dispatcher returns success=true
    And running `fspec add-aggregate AUTH-001 "A2"` afterwards exits 0 and prints '✓ Added aggregate "A2" to AUTH-001 (ID: 1)'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 2
    And the CLI bridge module rust/fspec/src/add_aggregate.rs contains NO inline item construction, status guard, or file-write logic — its only computation is JSON arg marshalling
