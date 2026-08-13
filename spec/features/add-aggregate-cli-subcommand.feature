@wip
@RPC-165
Feature: fspec add-aggregate CLI subcommand
  """
  CLI bridge: rust/fspec/src/add_aggregate.rs — clap-derived struct mirroring the TS
  Commander.js registration (src/commands/add-aggregate.ts:152-189). Surface:
  `fspec add-aggregate <workUnitId> <text> [--responsibilities <list>] [--timestamp <ms>]
  [--bounded-context <context>]`.

  PARITY NOTE — TS latent bug (source-of-truth behaviour):
  The TS `.action()` callback renders its success line via `logger.success(...)`. The fspec
  Winston `logger` (src/utils/logger.ts) has ONLY a file transport (~/.fspec/fspec.log) and
  NO `success` level, so `logger.success(...)` throws a TypeError that is swallowed by the
  surrounding try/catch — which then calls `logger.error(...)` (also file-only) and
  `process.exit(1)`. Net observable behaviour of `node dist/index.js add-aggregate ...` for
  EVERY invocation: stdout EMPTY, stderr EMPTY, exit code 1 — even on success. The aggregate
  IS still persisted to spec/work-units.json (the mutation happens before the throw). The
  Rust port matches this byte-for-byte: it runs the core mutation, emits no console output,
  and returns exit 1. (Contrast add-command/add-bounded-context, which use the console-backed
  `output.log`/`output.error` abstraction and behave normally.)
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

  Scenario: CLI persists the aggregate but produces no output and exits 1 (TS logger.success bug parity)
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I run `fspec add-aggregate AUTH-001 "Order" --responsibilities "Place,Cancel"` in that tempdir
    Then the exit code is 1
    And stdout is empty
    And stderr is empty
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 1
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].text='Order'
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items[0].responsibilities equals the array ["Place","Cancel"]

  Scenario: CLI rejects a done work unit with exit 1 and no console output (TS logger file-only parity)
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=done
    When I run `fspec add-aggregate AUTH-001 "Anything"` in that tempdir
    Then the exit code is 1
    And stdout is empty
    And stderr is empty
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I dispatch add-aggregate via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' text='A1'
    Then the dispatcher returns success=true
    And running `fspec add-aggregate AUTH-001 "A2"` afterwards exits 1 with no console output (TS logger.success bug parity)
    And spec/work-units.json on disk shows AUTH-001.eventStorm.items has length 2
    And the CLI bridge module rust/fspec/src/add_aggregate.rs contains NO inline item construction, status guard, or file-write logic — its only computation is JSON arg marshalling
