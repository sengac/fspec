@querying
@cli
@wip
@RPC-297
Feature: Port search-scenarios command to Rust

  """
  Core impl: codelet/fspec-core/src/commands/search_scenarios.rs rewrites the stub; signature run(args_json, project_root). Reuses io/feature_glob.rs (filtered to flat spec/features/*.feature) + io/gherkin.rs::parse_feature_lenient. Reads spec/work-units.json best-effort for work-unit-title matching.
  Two-front-doors: dispatcher and clap CLI both call search_scenarios::run. CLI bridge codelet/fspec/src/search_scenarios.rs marshals --query/--regex/--json into JSON only. Help config + intercept arm in main.rs (search-scenarios-help.ts exists as rich help). Mode::SearchScenarios variant + forward! arm wired by supervisor.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Reads spec/features/*.feature (flat dir, NON-recursive parity with TS glob) and parses each with the lenient gherkin front-end
  #   2. Literal search is case-insensitive substring; regex search uses case-insensitive RegExp; invalid regex surfaces an error
  #   3. The query option is REQUIRED; regex and json are optional boolean flags
  #   4. When a feature name, description, file path, or its work-unit title matches, ALL scenarios of that feature are returned; otherwise only scenarios whose name matches
  #   5. The dispatcher returns a JSON envelope with searchedFiles, scenarios, format and searchMode fields; each scenario has name, scenarioName, featureFilePath and workUnitId
  #   6. workUnitId falls back to 'unknown' when the feature has no @PREFIX-NNN tag; missing spec/features directory yields searchedFiles=0 and empty scenarios (not an error)
  #   7. The CLI bridge prints the JSON envelope when --json is set, otherwise a green checkmark summary line 'Found N scenarios matching "query"'; errors go to stderr and exit 1
  #
  # EXAMPLES:
  #   1. Searching 'Login' matches a scenario named 'Login with valid credentials' and returns its featureFilePath and workUnitId
  #   2. Searching '--query valid.* --regex' matches multiple scenarios via regex
  #   3. Searching '--query nonexistent' with no matches returns searchedFiles>0 and an empty scenarios array
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to search scenarios across all feature files by literal text or regex, sharing one Rust source of truth between the LLM dispatcher and the CLI
    So that I can locate scenarios by keyword without launching Node

  Scenario: Literal query matches a scenario by name
    Given a temp project root contains spec/features with a feature whose scenario is named "Login with valid credentials"
    When I dispatch search-scenarios with query="Login"
    Then the dispatcher returns success=true
    And the scenarios array contains an entry with scenarioName "Login with valid credentials"
    And that entry carries its featureFilePath and workUnitId

  Scenario: Regex query matches multiple scenarios case-insensitively
    Given a temp project root contains spec/features with scenarios named "Validate user" and "valid email"
    When I dispatch search-scenarios with query="valid.*" and regex=true
    Then the dispatcher returns success=true
    And the searchMode field equals 'regex'
    And the scenarios array has 2 elements

  Scenario: No match returns an empty scenarios array
    Given a temp project root contains spec/features with at least one feature file
    When I dispatch search-scenarios with query="zzz-nonexistent-zzz"
    Then the dispatcher returns success=true
    And the searchedFiles field is greater than 0
    And the scenarios array is empty

  Scenario: Feature-name match returns all of that feature's scenarios
    Given a temp project root contains a feature named "User Authentication" with two scenarios
    When I dispatch search-scenarios with query="Authentication"
    Then the dispatcher returns success=true
    And the scenarios array has 2 elements

  Scenario: workUnitId falls back to unknown when feature has no work-unit tag
    Given a temp project root contains an untagged feature with one scenario
    When I dispatch search-scenarios with query matching that scenario
    Then the dispatcher returns success=true
    And the matching scenario's workUnitId equals 'unknown'

  Scenario: Invalid regex pattern surfaces a structured error
    Given a temp project root contains spec/features with at least one feature file
    When I dispatch search-scenarios with query="[" and regex=true
    Then the dispatcher returns success=false
    And the error field contains the substring 'regex'

  Scenario: Missing spec/features directory yields zero searched files
    Given a temp project root with no spec/features directory
    When I dispatch search-scenarios with query="anything"
    Then the dispatcher returns success=true
    And the searchedFiles field equals 0
    And the scenarios array is empty

  Scenario: json format flag sets the format field to json
    Given a temp project root contains spec/features with at least one feature file
    When I dispatch search-scenarios with query="Login" and json=true
    Then the dispatcher returns success=true
    And the format field equals 'json'

  Scenario: Shared infrastructure module is registered for search-scenarios
    Given the codelet/fspec-core crate is built
    When I inspect codelet/fspec-core/src/commands/search_scenarios.rs
    Then the module no longer returns FspecCoreError::NotYetPorted
    And the dispatcher routes search-scenarios to the new run function
