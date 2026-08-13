@done
@RPC-173
@rust
@cli
@mutation
Feature: Port add-capability command to Rust
  """
  Rust port of the TypeScript `add-capability` command
  (src/commands/add-capability.ts via src/commands/register-add-capability.ts).
  Appends a {name, description} capability to solutionSpace.capabilities in the
  foundation document.

  Core impl at rust/fspec-core/src/commands/add_capability.rs uses the
  2-arg signature run(args_json, project_root). Args (camelCase JSON):
  { name: String, description: String }.

  Draft precedence: if spec/foundation.json.draft exists it is read AND written
  back; otherwise spec/foundation.json is used. The command deliberately does
  NOT route through ensure_foundation_file — when NEITHER file exists it fails
  with 'foundation.json not found' and creates no file (parity with the TS
  ENOENT branch which throws instead of auto-creating).

  Placeholder pruning mirrors isPlaceholderCapability: a capability whose name
  OR description matches the regex /\[QUESTION:|\[DETECTED:/ is a placeholder.
  Placeholders are removed ONLY when the array contains EXCLUSIVELY
  placeholders; mixed arrays keep every entry. When N placeholders are removed a
  'Removed N placeholder capability(ies)' line precedes the success lines.

  Framing-A divergence — trailing newline: the TS command writes
  JSON.stringify(data, null, 2) + '\n'. write_json_atomic does NOT emit a
  trailing newline, so the Rust port uses a module-local atomic write that
  appends '\n' for byte-exact parity (avoids touching shared io/).

  Two-front-doors: clap CLI and LLM dispatcher both call
  commands::add_capability::run.
  """

  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want a Rust implementation of the add-capability command that matches the TypeScript behaviour
    So that the standalone fspec Rust binary can add capabilities to foundation.json without depending on Node.js

  Scenario: Dispatcher appends a capability to an existing foundation.json
    Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'Reporting'}]
    And no spec/foundation.json.draft file exists
    When I dispatch add-capability with name='User Authentication' description='Login and sessions'
    Then the dispatcher returns success=true
    And spec/foundation.json solutionSpace.capabilities has length 2
    And the last capability has name='User Authentication' and description='Login and sessions'
    And the result fileName is 'foundation.json'

  Scenario: Draft takes precedence over the final foundation file
    Given spec/foundation.json.draft exists with solutionSpace.capabilities=[{name:'Reporting'}]
    And spec/foundation.json also exists
    When I dispatch add-capability with name='Data Export' description='Export to CSV'
    Then the dispatcher returns success=true
    And spec/foundation.json.draft solutionSpace.capabilities has length 2
    And spec/foundation.json is left unchanged
    And the result fileName is 'foundation.json.draft'

  Scenario: Capabilities array is created when solutionSpace has no capabilities key
    Given spec/foundation.json exists with a solutionSpace object that has no capabilities key
    When I dispatch add-capability with name='Search' description='Full text search'
    Then the dispatcher returns success=true
    And spec/foundation.json solutionSpace.capabilities has length 1
    And the only capability has name='Search'

  Scenario: All-placeholder capabilities are pruned before the new entry is added
    Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'[QUESTION: What can users do?]', description:'[DETECTED: ...]'}]
    When I dispatch add-capability with name='Login' description='Authenticate users'
    Then the dispatcher returns success=true
    And spec/foundation.json solutionSpace.capabilities has length 1
    And the only capability has name='Login'
    And the result removedCount is 1

  Scenario: Mixed real-and-placeholder array keeps every entry
    Given spec/foundation.json exists with solutionSpace.capabilities=[{name:'Reporting'},{name:'[QUESTION: anything else?]'}]
    When I dispatch add-capability with name='Login' description='Authenticate users'
    Then the dispatcher returns success=true
    And spec/foundation.json solutionSpace.capabilities has length 3
    And the result removedCount is 0

  Scenario: Dispatcher fails when neither foundation.json nor its draft exists
    Given a project root directory with no spec/foundation.json and no spec/foundation.json.draft
    When I dispatch add-capability with name='X' description='Y'
    Then the dispatcher returns success=false
    And the error message contains the substring 'foundation.json not found'
    And no spec/foundation.json file is created
    And no spec/foundation.json.draft file is created

  Scenario: Unknown top-level foundation fields are preserved on write
    Given spec/foundation.json exists with solutionSpace.capabilities=[] and a custom top-level 'experiments' key
    When I dispatch add-capability with name='Search' description='Full text search'
    Then the dispatcher returns success=true
    And spec/foundation.json still contains the 'experiments' key with its original value

  Scenario: Successful result reports the written file, name and description
    Given spec/foundation.json exists with solutionSpace.capabilities=[]
    When I dispatch add-capability with name='Search' description='Full text search'
    Then the dispatcher returns success=true
    And the result name is 'Search'
    And the result description is 'Full text search'
    And the result fileName is 'foundation.json'
