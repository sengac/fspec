@done
@RPC-269
@rust
@cli
@mutation
Feature: Port remove-capability command to Rust

  """
  Rust port of the TypeScript `remove-capability` command
  (src/commands/remove-capability.ts via
  src/commands/register-remove-capability.ts). Removes a capability matched by
  EXACT case-sensitive name from solutionSpace.capabilities.

  Core impl at codelet/fspec-core/src/commands/remove_capability.rs uses the
  2-arg signature run(args_json, project_root). Args (camelCase JSON):
  { name: String }.

  Draft precedence: if spec/foundation.json.draft exists it is read AND written
  back; otherwise spec/foundation.json is used. The command does NOT route
  through ensure_foundation_file — when NEITHER file exists it fails with
  'foundation.json not found' and creates no file.

  Matching removes the FIRST entry whose name == the requested name (splice
  index, 1). Failure modes:
    * capabilities missing or empty → 'Capability "<name>" not found' with the
      detail line 'No capabilities exist in foundation'.
    * name not present → 'Capability "<name>" not found' with the detail line
      'Available capabilities: <comma-joined names>'.

  Framing-A divergence — trailing newline: the TS command writes
  JSON.stringify(data, null, 2) + '\n'. The Rust port uses a module-local atomic
  write that appends '\n' for byte-exact parity (avoids touching shared io/).
  The error reason carries the TS detail line so the CLI can render byte-parity
  stderr.

  Two-front-doors: clap CLI and LLM dispatcher both call
  commands::remove_capability::run.
  """

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want a Rust implementation of the remove-capability command that matches the TypeScript behaviour
    So that the standalone fspec Rust binary can remove capabilities from foundation.json without depending on Node.js

  Scenario: Dispatcher removes a capability from an existing foundation.json
    Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'User Authentication'},{name:'Search'}]
    And no spec/foundation.json.draft file exists
    When I dispatch remove-capability with name='Search'
    Then the dispatcher returns success=true
    And spec/foundation.json solutionSpace.capabilities has length 1
    And the remaining capability has name='User Authentication'
    And the result fileName is 'foundation.json'

  Scenario: Draft takes precedence over the final foundation file
    Given spec/foundation.json.draft exists with solutionSpace.capabilities=[{name:'Reporting'},{name:'Data Export'}]
    And spec/foundation.json also exists
    When I dispatch remove-capability with name='Reporting'
    Then the dispatcher returns success=true
    And spec/foundation.json.draft solutionSpace.capabilities has length 1
    And spec/foundation.json is left unchanged
    And the result fileName is 'foundation.json.draft'

  Scenario: Only the first exact case-sensitive match is removed
    Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'login'},{name:'Login'}]
    When I dispatch remove-capability with name='Login'
    Then the dispatcher returns success=true
    And spec/foundation.json solutionSpace.capabilities has length 1
    And the remaining capability has name='login'

  Scenario: Dispatcher fails when no capabilities exist
    Given spec/foundation.json exists with an empty solutionSpace.capabilities array
    When I dispatch remove-capability with name='X'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Capability "X" not found'
    And the error message contains the substring 'No capabilities exist in foundation'

  Scenario: Dispatcher fails and lists available capabilities when the name is not found
    Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'Reporting'},{name:'Search'}]
    When I dispatch remove-capability with name='Login'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Capability "Login" not found'
    And the error message contains the substring 'Available capabilities: Reporting, Search'

  Scenario: Dispatcher fails when neither foundation.json nor its draft exists
    Given a project root directory with no spec/foundation.json and no spec/foundation.json.draft
    When I dispatch remove-capability with name='X'
    Then the dispatcher returns success=false
    And the error message contains the substring 'foundation.json not found'
    And no spec/foundation.json file is created

  Scenario: Unknown top-level fields and untouched capabilities are preserved on write
    Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'Reporting'},{name:'Search'}] and a custom top-level 'experiments' key
    When I dispatch remove-capability with name='Reporting'
    Then the dispatcher returns success=true
    And spec/foundation.json still contains the 'experiments' key with its original value
    And the remaining capability has name='Search'
