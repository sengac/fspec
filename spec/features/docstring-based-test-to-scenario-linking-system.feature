@done
@validation
@high
@cli
@traceability
@test-coverage
@TEST-006
Feature: Docstring-based test-to-scenario linking system
  """
  Step-level validation emits <system-reminder> showing missing/mismatched steps with exact text to copy from feature file. show-coverage displays step-level status (✓ matched / ✗ MISSING STEP COMMENT)
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
  #   9. Test files use comments (// Given, // When, // Then) that must match actual steps in feature file scenarios
  #   10. System validates that test comments match feature file steps exactly (Cucumber-style step matching)
  #   11. Step matching supports parameterized steps using hybrid similarity algorithm for fuzzy matching (e.g., '// Given I have 5 items' matches 'Given I have {int} items')
  #   12. Existing fspec coverage commands (link-coverage, show-coverage, audit-coverage) are enhanced to validate step comments match feature file steps
  #   13. Step comment validation is a HARD ERROR by default - command fails if test comments don't match feature steps
  #   14. Override flag --skip-step-validation allows linking even when step comments don't match (escape hatch for edge cases)
  #   15. When step validation fails, emit <system-reminder> showing: (1) which steps are missing/mismatched, (2) exact step text from feature file to copy, (3) how to override with --skip-step-validation flag
  #   16. Step comments use '@step' prefix for fast searching: '// @step Given I am on the login page'
  #   17. System is backward compatible - also recognizes plain step comments without @step prefix (e.g., '// Given I am on the login page')
  #   18. show-coverage displays step-level validation status showing which steps have matching comments (✓ matched) vs missing (✗ MISSING STEP COMMENT)
  #   19. Step matching uses existing hybrid similarity algorithm with adaptive thresholds: <10 chars=0.85, 10-20=0.80, 20-40=0.75, 40+=0.70
  #
  # EXAMPLES:
  #   1. Developer writes test for 'Login with valid credentials' scenario at lines 45-62 in auth.test.ts, runs link-coverage command, coverage file updated with test mapping
  #   2. Developer implements login function at lines 10-24 in login.ts, runs link-coverage to add implementation mapping to existing test, coverage file shows full traceability
  #   3. Developer runs show-coverage for user-authentication feature, sees 50% coverage (1 of 2 scenarios covered), identifies uncovered scenario to work on next
  #   4. Developer refactors codebase and moves test files, runs audit-coverage, discovers broken file paths, runs audit-coverage --fix to remove stale mappings
  #   5. Developer runs show-coverage with no arguments, sees project-wide report showing 65% overall coverage across all features, identifies features needing attention
  #   6. Test has '// Given I have 5 items', feature has 'Given I have {int} items', hybrid similarity matches them with high confidence
  #   7. Running 'fspec link-coverage user-auth --scenario Login...' validates that test file contains matching step comments for all Given/When/Then steps in the scenario
  #   8. Test missing '// When I click login', link-coverage fails with system-reminder showing exact text to add: '// When I click the login button' and override option '--skip-step-validation'
  #   9. Test file with '// @step Given I am on the login page' matches feature step 'Given I am on the login page'
  #   10. Legacy test with '// Given I am logged in' (no @step prefix) still matches feature step 'Given I am logged in' for backward compatibility
  #   11. show-coverage output shows: 'Scenario: Login (FULLY COVERED)' with step breakdown '✓ Given I am on login page (matched)' and '✗ Then I should be logged in (MISSING STEP COMMENT)'
  #   12. Step '// @step Given I have 5 items' matches 'Given I have {int} items' using hybrid similarity with threshold 0.75 (medium length step)
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
    I want to link test step comments to Gherkin feature file steps using @step prefix
    So that I maintain Cucumber-style step-level traceability and get validation when steps don't match

  Scenario: Validate test with @step prefix comments matches feature steps
    Given I have a feature file with scenario 'Login' containing steps 'Given I am on the login page', 'When I click the login button', 'Then I should be logged in'
    And I have a test file with comments '// @step Given I am on the login page', '// @step When I click the login button', '// @step Then I should be logged in'
    When I run 'fspec link-coverage user-login --scenario Login --test-file src/__tests__/auth.test.ts --test-lines 10-25'
    Then the command should succeed with step validation passing
    And the coverage file should be updated with the test mapping

  Scenario: Fail validation when test missing step comments with helpful system-reminder
    Given I have a feature file with scenario containing step 'When I click the login button'
    And I have a test file that is missing the '// @step When I click the login button' comment
    When I run 'fspec link-coverage user-login --scenario Login --test-file src/__tests__/auth.test.ts --test-lines 10-25'
    Then the command should fail with exit code 1
    And a <system-reminder> should show the exact step text to add: '// @step When I click the login button'
    And the reminder should include override option: '--skip-step-validation'

  Scenario: Match parameterized steps using hybrid similarity algorithm
    Given I have a feature file with parameterized step 'Given I have {int} items in my cart'
    And I have a test file with comment '// @step Given I have 5 items in my cart'
    When I run 'fspec link-coverage shopping-cart --scenario Add-items --test-file src/__tests__/cart.test.ts --test-lines 20-35'
    Then the step should match using hybrid similarity with threshold 0.75
    And the command should succeed with step validation passing

  Scenario: Support backward compatibility with plain step comments without @step prefix
    Given I have a legacy test file with plain comments '// Given I am logged in' (no @step prefix)
    And I have a feature file with step 'Given I am logged in'
    When I run 'fspec link-coverage user-session --scenario Session-management --test-file src/__tests__/session.test.ts --test-lines 15-30'
    Then the step should match even without @step prefix
    And the command should succeed with step validation passing

  Scenario: Display step-level validation status in show-coverage output
    Given I have a feature with scenario 'Login' containing 3 steps
    And the test file has matching comments for 2 steps but is missing comment for 'Then I should be logged in'
    When I run 'fspec show-coverage user-login'
    Then I should see '✓ Given I am on the login page (matched)'
    And I should see '✓ When I click the login button (matched)'
    And I should see '✗ Then I should be logged in (MISSING STEP COMMENT)'
