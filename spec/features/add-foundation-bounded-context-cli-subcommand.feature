@done
@RPC-183
Feature: fspec add-foundation-bounded-context CLI subcommand

  """
  CLI bridge: codelet/fspec/src/add_foundation_bounded_context.rs — clap-derived struct mirroring
  the TS Commander.js registration (src/commands/add-foundation-bounded-context.ts:122-132).
  Surface: `fspec add-foundation-bounded-context <text>`.
  Stdout (success): '✓ Added bounded context "<text>" to foundation Event Storm' ONLY (the TS
  command calls generateFoundationMdCommand whose result is discarded — it never prints a
  regeneration line; the Rust core itself does NOT touch FOUNDATION.md either).
  Stderr (failure): 'Error: <message>'; exit code 1.
  Two-front-doors invariant: the bridge marshals args into JSON {text} and forwards to
  commands::add_foundation_bounded_context::run — NO domain logic in the bridge (no item
  construction, seeding, or file-write logic). Help fixture captured from
  `node dist/index.js add-foundation-bounded-context --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-foundation-bounded-context subcommand to parse the same positional argument as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven foundation Event Storming script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec add-foundation-bounded-context --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-foundation-bounded-context.txt
    And stdout starts with a blank line followed by 'ADD-FOUNDATION-BOUNDED-CONTEXT'

  Scenario: CLI successfully appends a bounded context and prints the success line
    Given a project root tempdir with spec/foundation.json containing the generic schema and no eventStorm field
    When I run `fspec add-foundation-bounded-context "Order Management"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Added bounded context "Order Management" to foundation Event Storm'
    And spec/foundation.json on disk shows eventStorm.items has length 1
    And spec/foundation.json on disk shows eventStorm.items[0].text='Order Management'
    And spec/foundation.json on disk shows eventStorm.items[0].type='bounded_context'

  Scenario: CLI prints ONLY the success line (no FOUNDATION.md regeneration line)
    Given a project root tempdir with spec/foundation.json containing the generic schema and no eventStorm field
    When I run `fspec add-foundation-bounded-context "Identity"` in that tempdir
    Then the exit code is 0
    And stdout does NOT contain the substring 'Regenerated'
    And stdout contains the substring '✓ Added bounded context "Identity" to foundation Event Storm'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/foundation.json containing the generic schema and no eventStorm field
    When I dispatch add-foundation-bounded-context via fspec_core::dispatch::dispatch_command with text='C1'
    Then the dispatcher returns success=true
    And running `fspec add-foundation-bounded-context "C2"` afterwards exits 0
    And spec/foundation.json on disk shows eventStorm.items has length 2
    And the CLI bridge module codelet/fspec/src/add_foundation_bounded_context.rs contains NO inline item construction, seeding, or file-write logic — its only computation is JSON arg marshalling
