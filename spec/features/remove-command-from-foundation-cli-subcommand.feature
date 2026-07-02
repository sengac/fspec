@done
@RPC-270
Feature: fspec remove-command-from-foundation CLI subcommand
  """
  CLI bridge: codelet/fspec/src/remove_command_from_foundation.rs — clap-derived struct mirroring the
  TS Commander.js registration (src/commands/remove-command-from-foundation.ts:131-154). Surface:
  `fspec remove-command-from-foundation <context-name> <command-name>`.
  Stdout (success): '✓ Removed command "<command-name>" from "<context-name>" bounded context' (TS uses
  output.log('✓', message); ANSI tolerated via substring match). Stderr (failure): 'Error: <message>';
  exit code 1. Mirrors TS output.error(chalk.red('Error:'), message). Two-front-doors invariant: the
  bridge marshals args into JSON {contextName, commandName} and forwards to fspec_core
  commands::remove_command_from_foundation::run — NO domain logic in the bridge. Help fixture captured
  from `node dist/index.js remove-command-from-foundation --help`. DIVERGENCE: FOUNDATION.md
  regeneration skipped per the add_diagram (RPC-178) precedent.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's remove-command-from-foundation subcommand to parse the same positional arguments as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven Event Storming script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec remove-command-from-foundation --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/remove-command-from-foundation.txt

  Scenario: CLI successfully soft-deletes a command and prints the success line
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and a command text='CreateWorkUnit' boundedContextId=0 deleted=false
    When I run `fspec remove-command-from-foundation "Work Management" "CreateWorkUnit"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Removed command "CreateWorkUnit" from "Work Management" bounded context'
    And spec/foundation.json on disk shows the CreateWorkUnit command item deleted=true

  Scenario: CLI rejects a missing command with exit 1 and the TS-parity error prefix
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and a command text='CreateWorkUnit' boundedContextId=0
    When I run `fspec remove-command-from-foundation "Work Management" "Ghost"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring 'Error:'
    And stderr contains the substring "Command 'Ghost' not found in bounded context 'Work Management'"
    And spec/foundation.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/foundation.json whose eventStorm has a bounded_context text='Work Management' id=0 and commands text='C1' and text='C2' both boundedContextId=0
    When I dispatch remove-command-from-foundation via fspec_core::dispatch::dispatch_command with contextName='Work Management' commandName='C1'
    Then the dispatcher returns success=true
    And running `fspec remove-command-from-foundation "Work Management" "C2"` afterwards exits 0
    And spec/foundation.json on disk shows both command items C1 and C2 with deleted=true
    And the CLI bridge module codelet/fspec/src/remove_command_from_foundation.rs contains NO inline context lookup, command match, or file-write logic — its only computation is JSON arg marshalling
