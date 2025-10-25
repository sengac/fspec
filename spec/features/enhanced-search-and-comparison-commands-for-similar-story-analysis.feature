@done
@comparison
@search
@querying
@cli
@high
@QRY-002
Feature: Enhanced search and comparison commands for similar story analysis
  """
  Architecture notes:
  - New CLI commands for advanced querying and comparison
  - Commands: search-scenarios, compare-implementations, search-implementation, show-test-patterns
  - Uses existing work-units.json, feature files, and coverage files as data sources
  - Output formatting: table by default with --json flag for programmatic use
  - Search modes: literal string matching by default, --regex flag for patterns
  - Comparison features: side-by-side diffs, naming convention detection
  - Integration with /review command for cross-story analysis
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Search must support filtering by work unit type (story, bug, task)
  #   2. Search must support filtering by status (done, backlog, implementing, etc.)
  #   3. Search must support filtering by tags (single or multiple tags)
  #   4. Search must support filtering by feature group (e.g., all CLI features, all authentication features)
  #   5. Search results must show coverage information (test files and implementation files)
  #   6. Search must support text search across scenario names, feature descriptions, and work unit titles
  #   7. Search results must be sortable by date, story points, or work unit ID
  #   8. both - return work unit IDs for traceability and feature file paths for direct access to specs
  #   9. yes - side-by-side comparison helps identify pattern divergence and best practices
  #   10. table for human readability with --json flag option for programmatic use
  #   11. both - literal by default with --regex flag for advanced patterns
  #   12. yes - automatic highlighting of naming convention differences (camelCase vs snake_case, different prefixes, etc.)
  #
  # EXAMPLES:
  #   1. Search for all completed CLI stories: fspec query-work-units --type=story --status=done --tag=@cli
  #   2. Find scenarios with 'validation' in the name across all features: fspec search-scenarios --query=validation
  #   3. Compare implementation approaches for authentication stories: fspec compare-implementations --tag=@authentication --show-coverage
  #   4. Find all work units using a specific utility function: fspec search-implementation --function=loadConfig --show-work-units
  #   5. List test patterns for high-priority features: fspec show-test-patterns --tag=@high --include-coverage
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should search commands return work unit IDs, feature file paths, or both?
  #   A: true
  #
  #   Q: Should comparison commands show side-by-side diffs of implementation approaches?
  #   A: true
  #
  #   Q: What output format should search results use (table, JSON, plain text)?
  #   A: true
  #
  #   Q: Should search support regex patterns or just literal string matching?
  #   A: true
  #
  #   Q: Should comparison commands highlight naming convention differences automatically?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using /review command
    I want to search for similar completed stories to compare patterns and approaches
    So that I can identify inconsistencies and ensure architectural consistency across the codebase

  Scenario: Search for completed work units by type, status, and tag
    Given I have multiple completed work units tagged with @cli
    When I run 'fspec query-work-units --type=story --status=done --tag=@cli'
    Then the command should display a table of matching work units
    And each result should show work unit ID and feature file path
    And the results should include only story-type work units with done status
    And the results should include only work units tagged with @cli

  Scenario: Find scenarios by text search across features
    Given I have feature files with scenarios containing "validation" in their names
    When I run 'fspec search-scenarios --query=validation'
    Then the command should search all feature files
    And the results should show scenarios with "validation" in the scenario name
    And each result should show the scenario name, feature file path, and work unit ID
    And the results should be displayed in table format

  Scenario: Compare implementation approaches for tagged work units
    Given I have multiple completed work units tagged with @authentication
    And each work unit has coverage files linking to implementation code
    When I run 'fspec compare-implementations --tag=@authentication --show-coverage'
    Then the command should find all work units with @authentication tag
    And the results should show side-by-side comparison of implementation approaches
    And the results should highlight naming convention differences
    And the results should include test file and implementation file paths from coverage

  Scenario: Search implementation code for specific function usage
    Given I have implementation files using the "loadConfig" function
    And coverage files link these implementation files to work units
    When I run 'fspec search-implementation --function=loadConfig --show-work-units'
    Then the command should search all implementation files in coverage data
    And the results should show files containing "loadConfig" function
    And the results should show which work units use each file
    And the results should include file paths and work unit IDs

  Scenario: List test patterns for work units by tag
    Given I have multiple work units tagged with @high
    And each work unit has coverage files linking to test files
    When I run 'fspec show-test-patterns --tag=@high --include-coverage'
    Then the command should find all work units with @high tag
    And the results should show test file paths from coverage data
    And the results should identify common testing patterns across test files
    And the results should display patterns in table format

  Scenario: Search with regex pattern support
    Given I have scenarios with names containing "valid", "validate", or "validation"
    When I run 'fspec search-scenarios --query="valid.*" --regex'
    Then the command should use regex pattern matching
    And the results should include scenarios matching the regex pattern
    And the results should show all variations of "valid*" in scenario names

  Scenario: Output results in JSON format
    Given I have completed work units tagged with @cli
    When I run 'fspec query-work-units --type=story --status=done --tag=@cli --json'
    Then the command should output results in JSON format
    And the JSON should include work unit IDs and feature file paths
    And the JSON should be parsable for programmatic use
