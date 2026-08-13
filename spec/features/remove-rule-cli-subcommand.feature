@done
@RPC-279
Feature: fspec remove-rule CLI subcommand
  """
  CLI bridge: rust/fspec/src/remove_rule.rs — clap-derived struct mirroring TS Commander.js registration
  (src/commands/remove-rule.ts:88-105). Surface: `fspec remove-rule <workUnitId> <index>` (index parsed via parseInt base 10).
  Stdout (success): '✓ Removed rule: "<text>"' (TS uses chalk.green; ANSI tolerated via substring match).
  Stderr (failure): '✗ Failed to remove rule: <message>'; exit code 1. Mirrors TS `output.error('✗ Failed to remove rule:', ...)`.
  Non-numeric index (e.g. 'abc') parity: TS `parseInt('abc',10)` → NaN → 'Rule with ID NaN not found'; Rust accepts INDEX as String, parses to i64 on success or surfaces 'NaN' on failure.
  Help fixture captured from `node dist/index.js remove-rule --help`.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want the standalone Rust fspec binary's remove-rule subcommand to parse the same positional arguments as the TypeScript Commander.js registration
    So that any existing TS-CLI-driven Example Mapping script keeps working after the cutover

  Scenario: Help output matches the captured TS fixture
    Given the fspec Rust binary is built and on PATH
    When I run `fspec remove-rule --help`
    Then the exit code is 0
    And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/remove-rule.txt
    And stdout starts with a blank line followed by 'REMOVE-RULE'

  Scenario: CLI soft-deletes a rule and prints the canonical success line
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,text:'Email must be valid',deleted:false,createdAt:'x'}]
    When I run `fspec remove-rule AUTH-001 0` in that tempdir
    Then the exit code is 0
    And stdout contains the substring '✓ Removed rule: "Email must be valid"'
    And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=true

  Scenario: CLI rejects an unknown rule id with exit 1 and TS-parity error prefix
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,text:'r0',deleted:false,createdAt:'x'}]
    When I run `fspec remove-rule AUTH-001 99` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to remove rule:'
    And stderr contains the substring 'Rule with ID 99 not found'

  Scenario: CLI matches TS NaN behaviour when index is non-numeric
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,text:'r0',deleted:false,createdAt:'x'}]
    When I run `fspec remove-rule AUTH-001 abc` in that tempdir
    Then the exit code is 1
    And stderr contains the substring '✗ Failed to remove rule:'
    And stderr contains the substring 'Rule with ID NaN not found'

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and rules=[{id:0,text:'r0',deleted:false,createdAt:'x'},{id:1,text:'r1',deleted:false,createdAt:'x'}]
    When I dispatch remove-rule via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' index=0
    Then the dispatcher returns success=true
    And running `fspec remove-rule AUTH-001 1` afterwards exits 0
    And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=true and AUTH-001.rules[1].deleted=true
    And the CLI bridge module rust/fspec/src/remove_rule.rs contains NO inline soft-delete or file-write logic — its only computation is JSON arg marshalling
