@done
@RPC-173
@rust
@cli
@mutation
Feature: fspec add-capability CLI subcommand (Rust port)
  """
  Clap derive subcommand `add-capability` mirrors the TS Commander.js
  registration at src/commands/register-add-capability.ts — two required
  positional arguments `<name>` and `<description>`. The CLI bridge at
  codelet/fspec/src/add_capability.rs marshals the clap args into a JSON object
  and delegates to codelet_fspec_core::commands::add_capability::run; no draft
  probing, placeholder detection, or file IO logic is duplicated in the bridge.

  Stdout success block (parity with the TS output.log lines):
  [Removed N placeholder capability(ies)]   (only when N > 0)
  ✓ Added capability to <fileName>
  Name: <name>
  Description: <description>
  where <fileName> is 'foundation.json.draft' or 'foundation.json' depending on
  which file was actually written.

  Exit codes: 0 on success, 1 on any FspecCoreError. The missing-foundation
  failure is written to stderr (parity with the chalk-red TS error path:
  '✗ foundation.json not found' plus the discover-foundation hint line).

  The `fspec add-capability --help` output is byte-for-byte identical to the
  captured fixture at codelet/fspec/tests/fixtures/help/add-capability.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want an `add-capability` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes add-capability with two positional args in --help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec add-capability --help`
    Then the command exits 0
    And stdout describes the add-capability subcommand
    And stdout mentions the `<name>` argument
    And stdout mentions the `<description>` argument

  Scenario: CLI adds a capability and prints the success block on stdout
    Given spec/foundation.json exists with solutionSpace.capabilities=[]
    When I run `./codelet/target/release/fspec add-capability "Search" "Full text search"`
    Then the command exits 0
    And stdout contains the line '✓ Added capability to foundation.json'
    And stdout contains the substring '  Name: Search'
    And stdout contains the substring '  Description: Full text search'
    And spec/foundation.json solutionSpace.capabilities contains exactly one entry named 'Search'

  Scenario: CLI writes to the draft and reports the draft file name
    Given spec/foundation.json.draft exists with solutionSpace.capabilities=[{name:'Reporting'}]
    When I run `./codelet/target/release/fspec add-capability "Data Export" "Export to CSV"`
    Then the command exits 0
    And stdout contains the line '✓ Added capability to foundation.json.draft'
    And spec/foundation.json.draft solutionSpace.capabilities has length 2

  Scenario: CLI prints the placeholder-removal line when only placeholders existed
    Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'[QUESTION: What can users do?]', description:'[DETECTED: ...]'}]
    When I run `./codelet/target/release/fspec add-capability "Login" "Authenticate users"`
    Then the command exits 0
    And stdout contains the substring 'Removed 1 placeholder capability(ies)'
    And stdout contains the line '✓ Added capability to foundation.json'

  Scenario: CLI fails with exit 1 when foundation.json is missing
    Given a project root directory with no spec/foundation.json and no spec/foundation.json.draft
    When I run `./codelet/target/release/fspec add-capability "X" "Y"`
    Then the command exits with code 1
    And stderr contains the substring 'foundation.json not found'
    And no spec/foundation.json file is created

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/foundation.json exists with solutionSpace.capabilities=[]
    When I dispatch add-capability via fspec_core::dispatch::dispatch_command with name='Via dispatcher' description='d'
    Then the dispatcher writes spec/foundation.json
    And running `./codelet/target/release/fspec add-capability "Via CLI" "d"` afterwards exits 0
    And spec/foundation.json solutionSpace.capabilities contains two entries
    And the CLI bridge module codelet/fspec/src/add_capability.rs contains NO inline draft probing, placeholder detection, or JSON-mutation logic — its only computation is JSON arg marshalling

  Scenario: add-capability --help is byte-for-byte identical to the captured fixture
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec add-capability --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/add-capability.txt
