@done
@feature-management
@coverage-tracking
@scenario-generation
@cli
@phase2
@SPEC-002
Feature: Scenario deduplication and refactoring detection during generation
  """
  Extends src/commands/generate-scenarios.ts to search existing feature files before creating new ones. Uses semantic analysis to match scenarios. Integrates with existing coverage system (fspec link-coverage, unlink-coverage) to update test mappings when refactoring. Provides fspec audit-scenarios command for maintenance. Must preserve test files and implementation when updating scenarios.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When generating scenarios, check if any match existing scenarios in other feature files
  #   2. If a scenario is a refactor of an existing scenario, update the existing feature file instead of creating a duplicate
  #   3. When refactoring scenarios, update corresponding tests and regenerate coverage mappings
  #   4. Create new feature files only for truly new scenarios that don't match existing ones
  #   5. AI agent searches all feature files first, analyzes scenario titles and steps, takes hints from user's intent to determine if this is a refactor or genuinely new functionality
  #   6. Old scenario should be updated (changed into the new scenario) because it already has associated test, implementation, and coverage mappings that should be preserved
  #   7. System MUST automatically update tests and regenerate coverage when refactoring scenarios - this is enforced and non-negotiable
  #   8. Detection happens during 'fspec generate-scenarios' command as primary workflow. Before generating scenarios, search existing features for matches, prompt user to confirm if refactor, then update existing feature OR create new. Also provide 'fspec audit-scenarios' as standalone command for finding duplicates across all features for maintenance/cleanup.
  #   9. When generating scenarios, check if any match existing scenarios in other feature files
  #   10. If a scenario is a refactor of an existing scenario, update the existing feature file instead of creating a duplicate
  #   11. When refactoring scenarios, update corresponding tests and regenerate coverage mappings
  #   12. Create new feature files only for truly new scenarios that don't match existing ones
  #   13. AI agent searches all feature files first, analyzes scenario titles and steps, takes hints from user's intent to determine if this is a refactor or genuinely new functionality
  #   14. Old scenario should be updated (changed into the new scenario) because it already has associated test, implementation, and coverage mappings that should be preserved
  #   15. System MUST automatically update tests and regenerate coverage when refactoring scenarios - this is enforced and non-negotiable
  #   16. Detection happens during 'fspec generate-scenarios' command as primary workflow. Before generating scenarios, search existing features for matches, prompt user to confirm if refactor, then update existing feature OR create new. Also provide 'fspec audit-scenarios' as standalone command for finding duplicates across all features for maintenance/cleanup.
  #
  # EXAMPLES:
  #   1. User runs 'fspec generate-scenarios AUTH-005' where examples describe login validation. System finds existing scenario 'Validate user credentials' in user-authentication.feature. System prompts: 'Scenario appears to refactor existing scenario in user-authentication.feature. Update existing? (y/n)'. User confirms 'y'. System updates existing scenario, updates test file header comment, regenerates coverage.
  #   2. User runs 'fspec generate-scenarios AUTH-006' where examples describe OAuth integration. System searches all feature files, finds no OAuth-related scenarios. System creates new feature file 'oauth-integration.feature' with new scenarios.
  #   3. User runs 'fspec generate-scenarios BUG-009' with 3 examples. System detects: Example 1 matches existing scenario in feature-validation.feature (refactor), Example 2 matches scenario in tag-management.feature (refactor), Example 3 is new. System prompts for each match, updates 2 existing features with refactored scenarios, creates new feature file for Example 3 only.
  #   4. User runs 'fspec audit-scenarios' after several months of development. System finds 5 duplicate scenarios across different feature files. System reports: 'Found 5 potential duplicates' with file names, scenario titles, and similarity scores. User can choose to merge duplicates interactively.
  #   5. User runs 'fspec generate-scenarios AUTH-005' where examples describe login validation. System finds existing scenario 'Validate user credentials' in user-authentication.feature. System prompts: 'Scenario appears to refactor existing scenario in user-authentication.feature. Update existing? (y/n)'. User confirms 'y'. System updates existing scenario, updates test file header comment, regenerates coverage.
  #   6. User runs 'fspec generate-scenarios AUTH-006' where examples describe OAuth integration. System searches all feature files, finds no OAuth-related scenarios. System creates new feature file 'oauth-integration.feature' with new scenarios.
  #   7. User runs 'fspec generate-scenarios BUG-009' with 3 examples. System detects: Example 1 matches existing scenario in feature-validation.feature (refactor), Example 2 matches scenario in tag-management.feature (refactor), Example 3 is new. System prompts for each match, updates 2 existing features with refactored scenarios, creates new feature file for Example 3 only.
  #   8. User runs 'fspec audit-scenarios' after several months of development. System finds 5 duplicate scenarios across different feature files. System reports: 'Found 5 potential duplicates' with file names, scenario titles, and similarity scores. User can choose to merge duplicates interactively.
  #
  # QUESTIONS (ANSWERED):
  #   Q: How should we detect if a scenario is a 'refactor' vs a 'new' scenario? Should we use semantic similarity, exact title matching, or analyze the Given-When-Then steps?
  #   A: true
  #
  #   Q: When we detect a refactored scenario, what should happen to the old scenario? Should it be marked as deprecated, deleted, or should we prompt the user to decide?
  #   A: true
  #
  #   Q: Should the system automatically update tests when refactoring scenarios, or should it flag them for manual review? What about cases where tests might break?
  #   A: true
  #
  #   Q: Should this detection happen during 'fspec generate-scenarios' command, or should it be a separate validation step that can be run independently?
  #   A: true
  #
  #   Q: You mentioned 'similar to how the last bug story we fixed did' - which bug story are you referring to? Can you provide the work unit ID so I can review that approach?
  #   A: true
  #
  #   Q: How should we detect if a scenario is a 'refactor' vs a 'new' scenario? Should we use semantic similarity, exact title matching, or analyze the Given-When-Then steps?
  #   A: true
  #
  #   Q: When we detect a refactored scenario, what should happen to the old scenario? Should it be marked as deprecated, deleted, or should we prompt the user to decide?
  #   A: true
  #
  #   Q: Should the system automatically update tests when refactoring scenarios, or should it flag them for manual review? What about cases where tests might break?
  #   A: true
  #
  #   Q: Should this detection happen during 'fspec generate-scenarios' command, or should it be a separate validation step that can be run independently?
  #   A: true
  #
  #   Q: You mentioned 'similar to how the last bug story we fixed did' - which bug story are you referring to? Can you provide the work unit ID so I can review that approach?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. BUG-008: generate-scenarios produces malformed Gherkin steps from example titles. This bug fix improved how generate-scenarios parses example titles and creates proper Gherkin scenarios. The pattern we should follow is similar - analyzing example text and intelligently transforming it into proper scenarios.
  #   2. CORRECTION: Reference work unit is BUG-022 (not BUG-008). BUG-022 'Feature file naming for bug work units' - AI searches existing feature files, extracts capability from bug context, and either updates existing feature file OR creates capability-oriented new file. This demonstrates the exact pattern we need: search before create, intelligently match scenarios, update existing features instead of duplicating.
  #   3. BUG-022 demonstrates the pattern: AI searches existing feature files, determines if scenario matches existing capability, and updates existing feature files instead of creating duplicates.
  #
  # ========================================
  Background: User Story
    As a developer using fspec for ACDD workflow
    I want to detect and handle scenario refactoring during generation
    So that I avoid duplicate scenarios across feature files and maintain proper test coverage

  Scenario: Update existing feature file when scenario is a refactor
    Given I have a work unit AUTH-005 with examples describing login validation
    And an existing feature file 'user-authentication.feature' contains scenario 'Validate user credentials'
    When I run 'fspec generate-scenarios AUTH-005'
    Then the system should prompt 'Scenario appears to refactor existing scenario in user-authentication.feature. Update existing? (y/n)'
    And when I confirm 'y', the existing scenario should be updated with the new content
    And the test file header comment should be updated
    And the coverage mappings should be regenerated

  Scenario: Create new feature file when no match found
    Given I have a work unit AUTH-006 with examples describing OAuth integration
    And no existing feature files contain OAuth-related scenarios
    When I run 'fspec generate-scenarios AUTH-006'
    Then the system should create a new feature file 'oauth-integration.feature'
    And the new file should contain all generated scenarios

  Scenario: Handle mixed refactor and new scenarios
    Given I have a work unit BUG-009 with 3 examples
    And Example 1 matches an existing scenario in feature-validation.feature
    And Example 2 matches an existing scenario in tag-management.feature
    And Example 3 is completely new
    When I run 'fspec generate-scenarios BUG-009'
    Then the system should prompt me for each match (Examples 1 and 2)
    And when I confirm, it should update the 2 existing feature files
    And it should create a new feature file for Example 3 only

  Scenario: Audit command finds duplicate scenarios
    Given I have been developing for several months
    And duplicate scenarios exist across different feature files
    When I run 'fspec audit-scenarios'
    Then the system should report 'Found 5 potential duplicates'
    And it should display file names, scenario titles, and similarity scores
    And I should be able to merge duplicates interactively
