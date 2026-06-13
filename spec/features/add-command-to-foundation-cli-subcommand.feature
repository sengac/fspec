@done
@RPC-175
Feature: fspec add-command-to-foundation CLI subcommand

  """
  CLI bridge: codelet/fspec/src/add_command_to_foundation.rs — clap-derived struct mirroring the TS
  Commander.js registration (src/commands/add-command-to-foundation.ts:138-156). Surface:
  `fspec add-command-to-foundation <context-name> <command-name> [--description <text>]`.
  Stdout (success): '✓ Added command "<command-name>" to "<context-name>" bounded context' (TS uses
  output.log('✓', message); ANSI tolerated via substring match). Stderr (failure):
  'Error: <message>'; exit code 1. Mirrors TS output.error(chalk.red('Error:'), message).
  Two-front-doors invariant: the bridge marshals args into JSON {contextName, commandName, description?}
  and forwards to fspec_core commands::add_command_to_foundation::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js add-command-to-foundation --help`.
  DIVERGENCE: FOUNDATION.md regeneration skipped per the add_diagram (RPC-178) precedent.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-command-to-foundation subcommand to parse the same positional arguments and flags as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven Event Storming script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec add-command-to-foundation --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-command-to-foundation.txt

  Scenario: CLI successfully appends a command and prints the success line
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I run `fspec add-command-to-foundation "Work Management" "CreateWorkUnit"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Added command "CreateWorkUnit" to "Work Management" bounded context'
    And spec/foundation.json on disk shows eventStorm.items gained a command item with text='CreateWorkUnit' and boundedContextId=0

  Scenario: CLI forwards the --description flag into the persisted item
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I run `fspec add-command-to-foundation "Work Management" "CreateWorkUnit" --description "Creates a work unit"` in that tempdir
    Then the exit code is 0
    And spec/foundation.json on disk shows the appended command item description='Creates a work unit'

  Scenario: CLI rejects a missing bounded context with exit 1 and the TS-parity error prefix
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I run `fspec add-command-to-foundation "Nope" "CreateWorkUnit"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring "Bounded context 'Nope' not found"
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and nextItemId=1
    When I dispatch add-command-to-foundation via fspec_core::dispatch::dispatch_command with contextName='Work Management' commandName='C1'
    Then the dispatcher returns success=true
    And running `fspec add-command-to-foundation "Work Management" "C2"` afterwards exits 0
    And spec/foundation.json on disk shows eventStorm.items contains both command items 'C1' and 'C2'
    And the CLI bridge module codelet/fspec/src/add_command_to_foundation.rs contains NO inline item construction, context lookup, or file-write logic — its only computation is JSON arg marshalling
