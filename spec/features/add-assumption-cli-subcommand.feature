@done
@RPC-169
Feature: fspec add-assumption CLI subcommand
  """
  CLI bridge: codelet/fspec/src/add_assumption.rs — clap-derived struct mirroring TS Commander.js registration
  (src/commands/add-assumption.ts:65-80). Surface: `fspec add-assumption <work-unit-id> <assumption>`.
  Stdout (success): '✓ Assumption added successfully'. Stderr (failure): '✗ Failed to add assumption: <message>'; exit code 1. Mirrors TS `output.error('✗ Failed to add assumption:', ...)`.
  Help fixture captured from `node dist/index.js add-assumption --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-assumption subcommand to parse the same positional arguments as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven specification script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec add-assumption --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/add-assumption.txt
    And stdout starts with a blank line followed by 'ADD-ASSUMPTION'

  Scenario: CLI successfully appends an assumption and prints the success line
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I run `fspec add-assumption AUTH-001 "Users have valid email"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Assumption added successfully'
    And spec/work-units.json on disk shows AUTH-001.assumptions has length 1
    And spec/work-units.json on disk shows AUTH-001.assumptions[0]='Users have valid email'

  Scenario: CLI rejects a non-specifying status with exit 1 and TS-parity error prefix
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    When I run `fspec add-assumption AUTH-001 "Anything"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to add assumption:'
    And stderr contains the substring "Can only add assumptions during discovery/specification phase. AUTH-001 is in 'backlog' state."

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I dispatch add-assumption via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' assumption='A1'
    Then the dispatcher returns success=true
    And running `fspec add-assumption AUTH-001 "A2"` afterwards exits 0
    And spec/work-units.json on disk shows AUTH-001.assumptions has length 2
    And the CLI bridge module codelet/fspec/src/add_assumption.rs contains NO inline append, status guard, or file-write logic — its only computation is JSON arg marshalling
