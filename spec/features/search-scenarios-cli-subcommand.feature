@querying
@cli
@wip
@RPC-297
Feature: Port search-scenarios command to Rust
  """
  Core impl: rust/fspec-core/src/commands/search_scenarios.rs rewrites the stub; signature run(args_json, project_root). Reuses io/feature_glob.rs (filtered to flat spec/features/*.feature) + io/gherkin.rs::parse_feature_lenient. Reads spec/work-units.json best-effort for work-unit-title matching.
  Two-front-doors: dispatcher and clap CLI both call search_scenarios::run. CLI bridge rust/fspec/src/search_scenarios.rs marshals --query/--regex/--json into JSON only. Help config + intercept arm in main.rs (search-scenarios-help.ts exists as rich help). Mode::SearchScenarios variant + forward! arm wired by supervisor.
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

  Scenario: Clap exposes search-scenarios as a subcommand and prints flag help
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec search-scenarios --help` from a shell
    Then the command exits 0
    And stdout contains the substring 'search-scenarios'
    And stdout contains the substring '--query'

  Scenario: CLI without --query flag exits non-zero
    Given an empty directory is the current working directory
    When I run `./rust/target/release/fspec search-scenarios` from that directory
    Then the command exits with a non-zero status

  Scenario: CLI default output prints the green summary line
    Given a temp workspace contains spec/features with a feature whose scenario is named "Login with valid credentials"
    When I run `./rust/target/release/fspec search-scenarios --query Login` from that workspace
    Then the command exits 0
    And stdout contains the substring 'Found 1 scenarios matching "Login"'

  Scenario: CLI --json prints the 2-space JSON envelope to stdout
    Given a temp workspace contains spec/features with a feature whose scenario is named "Login with valid credentials"
    When I run `./rust/target/release/fspec search-scenarios --query Login --json` from that workspace
    Then the command exits 0
    And stdout parses as JSON with searchedFiles, scenarios, format, and searchMode fields
    And the JSON.searchMode field equals 'literal'

  Scenario: CLI --regex sets searchMode to regex
    Given a temp workspace contains spec/features with scenarios named "Validate user" and "valid email"
    When I run `./rust/target/release/fspec search-scenarios --query valid.* --regex --json` from that workspace
    Then the command exits 0
    And the JSON.searchMode field equals 'regex'
    And the JSON.scenarios array has 2 elements

  Scenario: CLI exits non-zero on an invalid regex pattern
    Given a temp workspace contains spec/features with at least one feature file
    When I run `./rust/target/release/fspec search-scenarios --query [ --regex` from that workspace
    Then the command exits with a non-zero status
    And stderr contains the substring '✗ Search failed:'

  Scenario: search-scenarios --help is byte-for-byte identical to the TS formatCommandHelp reference
    Given the fspec Rust binary at rust/target/release/fspec has been compiled
    When I run `./rust/target/release/fspec search-scenarios --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/search-scenarios.txt
    And stdout starts with a blank line followed by 'SEARCH-SCENARIOS'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has search-scenarios registered as a clap subcommand alongside other ported subcommands
    When I run `./rust/target/release/fspec --help`
    Then the help output lists search-scenarios as an available subcommand
    And the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a temp workspace contains spec/features with a feature whose scenario is named "Login with valid credentials"
    When I dispatch search-scenarios through fspec_core::dispatch::dispatch_command with query='Login' against that workspace
    And I run `./rust/target/release/fspec search-scenarios --query Login --json` against the same workspace
    Then both invocations produce a JSON envelope with the same scenarios array length
    And the CLI bridge module rust/fspec/src/search_scenarios.rs contains NO inline filtering or rendering logic — its only computation is JSON arg marshalling
