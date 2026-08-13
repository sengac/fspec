@done
@RPC-269
@rust
@cli
@mutation
Feature: fspec remove-capability CLI subcommand (Rust port)
  """
  Clap derive subcommand `remove-capability` mirrors the TS Commander.js
  registration at src/commands/register-remove-capability.ts — a single
  required positional argument `<name>`. The CLI bridge at
  rust/fspec/src/remove_capability.rs marshals the clap arg into a JSON
  object and delegates to
  codelet_fspec_core::commands::remove_capability::run; no draft probing,
  matching, or file IO logic is duplicated in the bridge.

  Stdout success line (parity with the TS output.log line):
  ✓ Removed capability "<name>" from <fileName>
  where <fileName> is 'foundation.json.draft' or 'foundation.json' depending on
  which file was actually written.

  Exit codes: 0 on success, 1 on any FspecCoreError. The TS command prints two
  stderr lines on the not-found paths ('✗ Capability "<name>" not found' plus
  either '  No capabilities exist in foundation' or
  '  Available capabilities: <names>'); the CLI bridge renders the same two
  lines from the error reason carried out of fspec-core.

  The `fspec remove-capability --help` output is byte-for-byte identical to the
  captured fixture at rust/fspec/tests/fixtures/help/remove-capability.txt.
  """

  Background: User Story
    As a fspec user running the standalone Rust binary
    I want a `remove-capability` subcommand whose CLI shape mirrors the TypeScript reference
    So that scripts and muscle-memory keep working when the binary swap from Node.js to Rust lands

  Scenario: Clap exposes remove-capability with one positional arg in --help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec remove-capability --help`
    Then the command exits 0
    And stdout describes the remove-capability subcommand
    And stdout mentions the `<name>` argument

  Scenario: CLI removes a capability and prints the success line on stdout
    Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'User Authentication'},{name:'Search'}]
    When I run `./rust/target/release/fspec remove-capability "Search"`
    Then the command exits 0
    And stdout contains the line '✓ Removed capability "Search" from foundation.json'
    And spec/foundation.json solutionSpace.capabilities has length 1

  Scenario: CLI prints the no-capabilities detail line and exits 1 when none exist
    Given spec/foundation.json exists with an empty solutionSpace.capabilities array
    When I run `./rust/target/release/fspec remove-capability "X"`
    Then the command exits with code 1
    And stderr contains the substring 'Capability "X" not found'
    And stderr contains the substring 'No capabilities exist in foundation'

  Scenario: CLI lists available capabilities and exits 1 when the name is not found
    Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'Reporting'},{name:'Search'}]
    When I run `./rust/target/release/fspec remove-capability "Login"`
    Then the command exits with code 1
    And stderr contains the substring 'Capability "Login" not found'
    And stderr contains the substring 'Available capabilities: Reporting, Search'

  Scenario: CLI fails with exit 1 when foundation.json is missing
    Given a project root directory with no spec/foundation.json and no spec/foundation.json.draft
    When I run `./rust/target/release/fspec remove-capability "X"`
    Then the command exits with code 1
    And stderr contains the substring 'foundation.json not found'
    And no spec/foundation.json file is created

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'A'},{name:'B'}]
    When I dispatch remove-capability via fspec_core::dispatch::dispatch_command with name='A'
    Then the dispatcher writes spec/foundation.json
    And running `./rust/target/release/fspec remove-capability "B"` afterwards exits 0
    And spec/foundation.json solutionSpace.capabilities is empty
    And the CLI bridge module rust/fspec/src/remove_capability.rs contains NO inline draft probing, matching, or JSON-mutation logic — its only computation is JSON arg marshalling

  Scenario: remove-capability --help is byte-for-byte identical to the captured fixture
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec remove-capability --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/remove-capability.txt
