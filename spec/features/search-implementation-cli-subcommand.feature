@querying
@cli
@wip
@RPC-296
Feature: Port search-implementation command to Rust

  """
  Core impl: codelet/fspec-core/src/commands/search_implementation.rs rewrites stub; signature run(args_json, project_root). Reads spec/features/*.feature.coverage via types/coverage.rs CoverageFile (inline dir walk, parity with show_test_patterns); extracts implMappings file paths. Reads each impl file via project_root.join(file). workUnitId = featureName.to_uppercase(). Submits optional shared-file request: add impl-extraction helper to io/coverage_glob.rs.
  Two-front-doors: dispatcher and clap CLI both call search_implementation::run. CLI bridge codelet/fspec/src/search_implementation.rs marshals --function/--show-work-units/--json into JSON only. Help config codelet/fspec-core/src/help/configs/search_implementation.rs (search-implementation-help.ts exists as rich help) + intercept arm + Mode::SearchImplementation variant wired by supervisor.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Reads spec/features/*.feature.coverage sidecars (flat dir), parses each JSON, and extracts implementation file paths from scenarios.testMappings.implMappings
  #   2. The function option is REQUIRED; show-work-units and json are optional boolean flags
  #   3. Each candidate implementation file is read from disk and matched by simple case-sensitive substring (content.includes(function)); unreadable files are skipped
  #   4. The dispatcher returns a JSON envelope with searchedFiles (count of impl mappings examined) and files; each file entry carries content, filePath and workUnits (workUnitId = featureName uppercased)
  #   5. Missing spec/features directory or coverage parse errors yield searchedFiles=0 and empty files (not an error)
  #   6. The CLI bridge prints the JSON envelope when --json is set, otherwise a green checkmark summary line 'Found "function" in N file(s)'; errors go to stderr and exit 1
  #
  # EXAMPLES:
  #   1. Searching for function 'loadConfig' returns the impl files whose content contains 'loadConfig' along with their work-unit ids
  #   2. Searching for a function that appears in no linked impl file returns an empty files array
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to search implementation files linked via coverage data for a specific function name, sharing one Rust source of truth between the LLM dispatcher and the CLI
    So that I can perform impact analysis on function usage across work units without launching Node

  Scenario: Clap exposes search-implementation as a subcommand and prints flag help
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec search-implementation --help` from a shell
    Then the command exits 0
    And stdout contains the substring 'search-implementation'
    And stdout contains the substring '--function'

  Scenario: CLI without --function flag exits non-zero
    Given an empty directory is the current working directory
    When I run `./codelet/target/release/fspec search-implementation` from that directory
    Then the command exits with a non-zero status

  Scenario: CLI default output prints the green summary line
    Given a temp workspace has a coverage sidecar whose implMappings reference an on-disk file containing "loadConfig"
    When I run `./codelet/target/release/fspec search-implementation --function loadConfig` from that workspace
    Then the command exits 0
    And stdout contains the substring 'Found "loadConfig" in 1 file(s)'

  Scenario: CLI --json prints the 2-space JSON envelope to stdout
    Given a temp workspace has a coverage sidecar whose implMappings reference an on-disk file containing "loadConfig"
    When I run `./codelet/target/release/fspec search-implementation --function loadConfig --json` from that workspace
    Then the command exits 0
    And stdout parses as JSON with searchedFiles and files fields

  Scenario: search-implementation --help is byte-for-byte identical to the TS formatCommandHelp reference
    Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    When I run `./codelet/target/release/fspec search-implementation --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/search-implementation.txt
    And stdout starts with a blank line followed by 'SEARCH-IMPLEMENTATION'

  Scenario: Default combined TUI mode is preserved when no subcommand is provided
    Given the fspec Rust binary has search-implementation registered as a clap subcommand alongside other ported subcommands
    When I run `./codelet/target/release/fspec --help`
    Then the help output lists search-implementation as an available subcommand
    And the long-about description still documents that running fspec with no subcommand enters combined TUI mode

  Scenario: CLI delegates to the same fspec_core function used by the dispatcher
    Given a temp workspace has a coverage sidecar whose implMappings reference an on-disk file containing "loadConfig"
    When I dispatch search-implementation through fspec_core::dispatch::dispatch_command with function='loadConfig' against that workspace
    And I run `./codelet/target/release/fspec search-implementation --function loadConfig --json` against the same workspace
    Then both invocations produce a JSON envelope with the same files array length
    And the CLI bridge module codelet/fspec/src/search_implementation.rs contains NO inline filtering or rendering logic — its only computation is JSON arg marshalling
