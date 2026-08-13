@done
@RPC-274
Feature: fspec remove-foundation-bounded-context CLI subcommand
  """
  CLI bridge: rust/fspec/src/remove_foundation_bounded_context.rs — clap-derived struct mirroring
  the TS Commander.js registration (src/commands/remove-foundation-bounded-context.ts:140-160).
  Surface: `fspec remove-foundation-bounded-context <context-name> [--cascade]`.
  Stdout (success): '✓ Removed bounded context "<name>"<cascadeMsg> from foundation Event Storm'
  (cascadeMsg = ' and all its children' when --cascade) ONLY (the TS command calls
  generateFoundationMdCommand whose result is discarded — it never prints a regeneration line;
  the Rust core itself does NOT touch FOUNDATION.md either).
  Stderr (failure): 'Error: <message>'; exit code 1.
  Two-front-doors invariant: the bridge marshals args into JSON {contextName, cascade?} and forwards
  to commands::remove_foundation_bounded_context::run — NO domain logic in the bridge (no
  find/soft-delete/file-write logic). Help fixture captured from
  `node dist/index.js remove-foundation-bounded-context --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's remove-foundation-bounded-context subcommand to parse the same positional argument and --cascade flag as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven foundation Event Storming script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec remove-foundation-bounded-context --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/remove-foundation-bounded-context.txt
    And stdout starts with a blank line followed by 'REMOVE-FOUNDATION-BOUNDED-CONTEXT'

  Scenario: CLI soft-deletes a childless bounded context and prints the success line
    Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Identity' deleted=false and no children
    When I run `fspec remove-foundation-bounded-context "Identity"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Removed bounded context "Identity" from foundation Event Storm'
    And spec/foundation.json on disk shows the 'Identity' bounded_context item has deleted=true

  Scenario: CLI --cascade removes the context and prints the cascade success line
    Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Sales' with 2 non-deleted child items carrying its boundedContextId
    When I run `fspec remove-foundation-bounded-context "Sales" --cascade` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Removed bounded context "Sales" and all its children from foundation Event Storm'
    And spec/foundation.json on disk shows both child items have deleted=true

  Scenario: CLI rejects a non-empty context without --cascade with exit 1
    Given a project root tempdir with spec/foundation.json containing an eventStorm bounded_context text='Sales' with 2 non-deleted child items carrying its boundedContextId
    When I run `fspec remove-foundation-bounded-context "Sales"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring "Bounded context 'Sales' has 2 child items. Use --cascade to remove the context and all its children."
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/foundation.json containing eventStorm bounded_contexts text='C1' and text='C2' both deleted=false and childless
    When I dispatch remove-foundation-bounded-context via fspec_core::dispatch::dispatch_command with contextName='C1'
    Then the dispatcher returns success=true
    And running `fspec remove-foundation-bounded-context "C2"` afterwards exits 0
    And spec/foundation.json on disk shows both 'C1' and 'C2' items have deleted=true
    And the CLI bridge module rust/fspec/src/remove_foundation_bounded_context.rs contains NO inline find, soft-delete, or file-write logic — its only computation is JSON arg marshalling
