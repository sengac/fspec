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

  Scenario: Function found in a linked implementation file
    Given a temp project root has a coverage sidecar whose implMappings reference an on-disk file containing "loadConfig"
    When I dispatch search-implementation with function="loadConfig"
    Then the dispatcher returns success=true
    And the files array contains an entry whose filePath is that implementation file
    And that entry's workUnits array carries the uppercased feature name

  Scenario: Function found in zero files returns an empty files array
    Given a temp project root has a coverage sidecar whose implMappings reference an on-disk file NOT containing "zzzNope"
    When I dispatch search-implementation with function="zzzNope"
    Then the dispatcher returns success=true
    And the files array is empty

  Scenario: Multiple impl mappings are counted in searchedFiles
    Given a temp project root has a coverage sidecar referencing two implMappings file paths
    When I dispatch search-implementation with function="anything"
    Then the dispatcher returns success=true
    And the searchedFiles field equals 2

  Scenario: Unreadable implementation files are skipped without error
    Given a temp project root has a coverage sidecar referencing an implMappings path that does not exist on disk
    When I dispatch search-implementation with function="loadConfig"
    Then the dispatcher returns success=true
    And the files array is empty

  Scenario: Missing spec/features directory yields zero searched files
    Given a temp project root with no spec/features directory
    When I dispatch search-implementation with function="anything"
    Then the dispatcher returns success=true
    And the searchedFiles field equals 0
    And the files array is empty

  Scenario: Shared infrastructure module is registered for search-implementation
    Given the codelet/fspec-core crate is built
    When I inspect codelet/fspec-core/src/commands/search_implementation.rs
    Then the module no longer returns FspecCoreError::NotYetPorted
    And the dispatcher routes search-implementation to the new run function
