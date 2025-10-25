@testing
@high
@done
@acdd
@cli
@traceability
@test-coverage
@TEST-006
Feature: Docstring-based test-to-scenario linking system
  """
  Coverage System: Uses .feature.coverage JSON files (auto-created by create-feature) to store scenario-to-test-to-implementation mappings. Complements feature files (acceptance criteria) with traceability data.

  Commands: link-coverage (add mappings), show-coverage (view gaps), audit-coverage (verify paths), unlink-coverage (remove mappings).

  File Format: JSON with scenarios array containing testMappings (file, lines, implMappings). Stats track totalScenarios, coveredScenarios, coveragePercent.

  Integration: Essential for reverse ACDD to track mapping progress. Critical for refactoring safety and gap detection.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every scenario must track which test file validates it and which implementation file implements it
  #   2. Coverage files (.feature.coverage) are auto-created by create-feature command and store scenario-to-test-to-implementation mappings
  #   3. Link coverage IMMEDIATELY after writing tests or code (do not batch)
  #   4. Coverage tracking uses line ranges for tests (e.g., 45-62) and specific line numbers for implementation (e.g., 10,11,12,23,24)
  #   5. Audit-coverage verifies file paths exist and can auto-fix broken links with --fix flag
  #   6. Coverage files (.feature.coverage) are created automatically when create-feature command runs. Detection is implicit through show-coverage command which shows uncovered scenarios.
  #   7. Audit-coverage command validates file paths exist. Out-of-sync links can be detected (broken paths) and auto-fixed with --fix flag. Line number validation is NOT performed.
  #   8. This IS the .feature.coverage system. Coverage files store scenario-to-test-to-implementation mappings. This complements feature files (acceptance criteria) with traceability data.
  #
  # EXAMPLES:
  #   1. Developer writes test for 'Login with valid credentials' scenario at lines 45-62 in auth.test.ts, runs link-coverage command, coverage file updated with test mapping
  #   2. Developer implements login function at lines 10-24 in login.ts, runs link-coverage to add implementation mapping to existing test, coverage file shows full traceability
  #   3. Developer runs show-coverage for user-authentication feature, sees 50% coverage (1 of 2 scenarios covered), identifies uncovered scenario to work on next
  #   4. Developer refactors codebase and moves test files, runs audit-coverage, discovers broken file paths, runs audit-coverage --fix to remove stale mappings
  #   5. Developer runs show-coverage with no arguments, sees project-wide report showing 65% overall coverage across all features, identifies features needing attention
  #
  # QUESTIONS (ANSWERED):
  #   Q: What should the docstring format look like? Should it reference scenario names, Given/When/Then steps, or both? Should it include feature file paths?
  #   A: true
  #
  #   Q: How should the tool detect existing tests without docstring links? Should it scan all test files in the project, or specific directories?
  #   A: true
  #
  #   Q: What should the migration tool do? (A) Interactive prompt for each unlinked test, (B) Batch analysis and suggested mappings, (C) Automatic linking based on test names, or (D) Something else?
  #   A: true
  #
  #   Q: Should the sync tool validate that docstring references match actual scenarios in feature files? What happens if they're out of sync?
  #   A: true
  #
  #   Q: Should this integrate with existing .feature.coverage files, or be a separate system? Should it replace or complement coverage tracking?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. Coverage tracking uses scenario-level traceability in .feature.coverage files, NOT step-level @step docstrings. This system complements scenario-level coverage, not replaces it.
  #   2. No migration tool exists. Coverage tracking is done manually via link-coverage command after writing tests and implementation. This is intentional - explicit linking ensures accuracy.
  #
  # ========================================
  Background: User Story
    As a developer using fspec for ACDD workflow
    I want to link Gherkin scenarios to test files and implementation code
    So that I maintain traceability between acceptance criteria, tests, and code for refactoring safety and gap detection

  Scenario: Link test file to scenario after writing tests
    Given I have written a test for 'Login with valid credentials' at lines 45-62 in src/__tests__/auth.test.ts
    And the feature file user-authentication.feature contains the scenario 'Login with valid credentials'
    When I run 'fspec link-coverage user-authentication --scenario "Login with valid credentials" --test-file src/__tests__/auth.test.ts --test-lines 45-62'
    Then the coverage file should be updated with the test mapping
    And running 'fspec show-coverage user-authentication' should show the scenario with test file path

  Scenario: Link implementation to existing test mapping
    Given I have linked a test for 'Login with valid credentials' scenario
    And I have implemented the login function at lines 10-24 in src/auth/login.ts
    When I run 'fspec link-coverage user-authentication --scenario "Login with valid credentials" --test-file src/__tests__/auth.test.ts --impl-file src/auth/login.ts --impl-lines 10-24'
    Then the coverage file should show full traceability from scenario to test to implementation
    And running 'fspec show-coverage user-authentication' should display test file and implementation file paths with line numbers

  Scenario: Identify coverage gaps with show-coverage
    Given the user-authentication feature has 2 scenarios: 'Login with valid credentials' and 'Login with invalid credentials'
    And only 'Login with valid credentials' has test/implementation coverage
    When I run 'fspec show-coverage user-authentication'
    Then I should see coverage at 50% (1 of 2 scenarios covered)
    And the uncovered scenario should be clearly marked as 'NOT COVERED'

  Scenario: Audit and fix broken coverage links after refactoring
    Given I have refactored the codebase and moved test files to a new location
    And some coverage file paths now reference files that no longer exist
    When I run 'fspec audit-coverage user-authentication'
    Then I should see a report showing broken file paths
    And when I run 'fspec audit-coverage user-authentication --fix', the broken mappings should be removed from the coverage file

  Scenario: View project-wide coverage report
    Given I have multiple features with varying coverage levels
    When I run 'fspec show-coverage' with no arguments
    Then I should see an overall coverage percentage across all features
    And I should see a list of features with their individual coverage percentages
    And features with low coverage should be clearly identified for attention
