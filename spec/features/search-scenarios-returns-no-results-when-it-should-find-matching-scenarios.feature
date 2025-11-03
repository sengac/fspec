@cli
@querying
@done
@BUG-059
Feature: search-scenarios returns no results when it should find matching scenarios

  """
  Architecture notes:
  - Search implementation is in src/utils/feature-parser.ts (searchScenarios function)
  - Must search across: scenario names, feature names, feature file paths, feature descriptions, and work unit titles
  - Work unit titles retrieved from spec/work-units.json by matching @WORK-UNIT-ID tags in feature files
  - Feature descriptions are the triple-quoted doc strings at the top of feature files
  - Search is case-insensitive substring matching
  - Returns all scenarios from features that match any search field
  - Step text should NOT be searched (keeps search focused on WHAT, not HOW)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Search must check scenario names for matches
  #   2. Business Rule 6 from QRY-002 states: 'Search must support text search across scenario names, feature descriptions, and work unit titles'
  #   3. Yes - feature descriptions should be searched. Already specified in QRY-002 BR6. Useful for finding features by technical details (technology names, architecture patterns, implementation approaches). Even if match is in description, returning scenarios from that feature is valuable.
  #   4. Yes - feature names should be searched. Feature name is the primary identifier for a feature, parallel to scenario names for scenarios. User's bug report expected this behavior. Low noise, high relevance results. Searching 'authentication' should find all scenarios from 'Feature: User Authentication'.
  #   5. Yes - feature file names should be searched. File names use kebab-case while feature names use spaces - substring matching won't catch both. Users see filenames in filesystem (ls spec/features/), natural to search by what they see. Low cost, already have filePath. User's bug report explicitly mentioned filename matching.
  #   6. Yes - work unit titles should be searched. Already in QRY-002 BR6 spec. Work unit titles often differ from feature names (bug descriptions vs feature titles). Users think in work units (BUG-059, AUTH-001). Infrastructure already exists (extract IDs from tags). One JSON read, cacheable. Strengthens traceability.
  #
  # EXAMPLES:
  #   1. User searches for 'report-bug', expects to find scenarios from 'report-bug-to-github-with-ai-assistance.feature', but gets 0 results because search only checks scenario names
  #   2. User searches for 'authentication', should find scenarios from features with 'authentication' in feature name, file name, or scenario names
  #   3. Current implementation only searches scenario names (line 104-107 in feature-parser.ts), but Business Rule 6 says it should also search feature descriptions and work unit titles
  #
  # QUESTIONS (ANSWERED):
  #   Q: The existing spec (QRY-002 Business Rule 6) says search should cover 'feature descriptions' (the triple-quoted doc strings at the top of features). Should we implement this?
  #   A: true
  #
  #   Q: The spec doesn't mention searching feature names (e.g., 'Report bug to GitHub with AI assistance'), but your bug example expects this. Should we search feature names in addition to scenario names?
  #   A: true
  #
  #   Q: Should we also search feature file names (e.g., 'report-bug-to-github-with-ai-assistance.feature')? This is NOT in the spec, but could be useful for finding features by filename patterns.
  #   A: true
  #
  #   Q: Should we search within scenario step text (Given/When/Then)? This is NOT in the spec but could help find scenarios by their implementation details (e.g., search 'browser' to find scenarios that open browsers).
  #   A: true
  #
  #   Q: The existing spec says search should cover 'work unit titles' (from spec/work-units.json). Should we search the work unit title field when a feature has a work unit tag like @BUG-059?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. No - step text should NOT be searched. Steps describe HOW (implementation), not WHAT (capability). High noise from verbose text with common words. search-implementation exists for finding by HOW it works. Not in QRY-002 spec (likely intentional). Keep search-scenarios focused on capabilities.
  #
  # ========================================

  Background: User Story
    As a developer using fspec
    I want to search for scenarios across feature files
    So that I can quickly find relevant scenarios without reading all feature files

  Scenario: Search by feature file name (kebab-case)
    Given a feature file named "spec/features/report-bug-to-github-with-ai-assistance.feature"
    And the feature file contains scenarios
    When I run "fspec search-scenarios --query=report-bug"
    Then I should see scenarios from "report-bug-to-github-with-ai-assistance.feature"
    And the results should not be empty

  Scenario: Search by feature name
    Given a feature file with "Feature: User Authentication"
    And the feature file contains scenarios
    When I run "fspec search-scenarios --query=authentication"
    Then I should see all scenarios from the "User Authentication" feature
    And the results should not be empty

  Scenario: Search by feature description
    Given a feature file with description containing "mermaid diagram validation"
    And the feature file contains scenarios
    When I run "fspec search-scenarios --query=mermaid"
    Then I should see all scenarios from features mentioning "mermaid" in description
    And the results should not be empty

  Scenario: Search by work unit title
    Given a work unit "BUG-059" with title "search-scenarios returns no results"
    And a feature file tagged with "@BUG-059"
    When I run "fspec search-scenarios --query=returns no results"
    Then I should see scenarios from the feature tagged with "@BUG-059"
    And the results should not be empty

  Scenario: Search by scenario name (existing functionality)
    Given a feature file with scenario "Login with valid credentials"
    When I run "fspec search-scenarios --query=login"
    Then I should see the scenario "Login with valid credentials"
    And the results should not be empty
