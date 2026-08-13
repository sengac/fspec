@done
@RPC-189
Feature: fspec add-rule CLI subcommand
  """
  CLI bridge: rust/fspec/src/add_rule.rs — clap-derived struct mirroring TS Commander.js registration
  (src/commands/add-rule.ts:76-93). Surface: `fspec add-rule <workUnitId> <rule>`.
  Stdout (success): '✓ Rule added successfully' (TS uses chalk.green; ANSI tolerated via substring match).
  Stderr (failure): '✗ Failed to add rule: <message>'; exit code 1. Mirrors TS `output.error('✗ Failed to add rule:', ...)`.
  Two-front-doors invariant: bridge marshals positional args into JSON {workUnitId, rule} and forwards to
  commands::add_rule::run — NO domain logic in the bridge.
  Help fixture captured from `node dist/index.js add-rule --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's add-rule subcommand to parse the same positional arguments as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven Example Mapping script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec add-rule --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/add-rule.txt
    And stdout starts with a blank line followed by 'ADD-RULE'

  Scenario: CLI successfully appends a rule and prints the success line
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I run `fspec add-rule AUTH-001 "Email must be valid format"` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Rule added successfully'
    And spec/work-units.json on disk shows AUTH-001.rules has length 1
    And spec/work-units.json on disk shows AUTH-001.rules[0].text='Email must be valid format'

  Scenario: CLI rejects a non-specifying status with exit 1 and TS-parity error prefix
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog
    When I run `fspec add-rule AUTH-001 "Anything"` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to add rule:'
    And stderr contains the substring "Can only add rules during discovery/specification phase. AUTH-001 is in 'backlog' state."
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    When I dispatch add-rule via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' rule='R1'
    Then the dispatcher returns success=true
    And running `fspec add-rule AUTH-001 "R2"` afterwards exits 0
    And spec/work-units.json on disk shows AUTH-001.rules has length 2
    And the CLI bridge module rust/fspec/src/add_rule.rs contains NO inline rule construction, status guard, or file-write logic — its only computation is JSON arg marshalling
