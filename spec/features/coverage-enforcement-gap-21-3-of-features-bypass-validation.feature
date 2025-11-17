@cli
@acdd
@validation
@coverage
@critical
@COV-054
Feature: Coverage enforcement gap: 21.3% of features bypass validation
  """
  Uses extractWorkUnitTags() from src/utils/work-unit-tags.ts to parse feature files and detect @WORK-UNIT-ID tags. Auto-discovery fallback when workUnit.linkedFeatures is empty or null. Maintains backward compatibility with explicit linkedFeatures array (explicit wins over auto-discovery).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Coverage validation must auto-discover feature files tagged with @WORK-UNIT-ID when linkedFeatures array is empty
  #   2. Auto-discovery must scan all feature files in spec/features directory
  #   3. Auto-discovery must use extractWorkUnitTags() utility from src/utils/work-unit-tags.ts
  #   4. If both linkedFeatures array AND @TAG exist, linkedFeatures takes precedence (explicit override)
  #   5. Coverage validation must run for ALL scenarios in auto-discovered features
  #
  # EXAMPLES:
  #   1. Work unit AUTH-001 has linkedFeatures: null but feature file has @AUTH-001 tag → Auto-discovery finds user-authentication.feature → Coverage validation runs
  #   2. Work unit with both linkedFeatures: ['custom-name'] AND @TAG in feature file → Use linkedFeatures (explicit wins over auto-discovery)
  #   3. Work unit TASK-001 (type=task) has no @TAG and no linkedFeatures → Allow progression with warning (tasks exempt)
  #   4. Work unit BUG-042 has @BUG-042 tag in feature file but 0% coverage → Block progression with coverage error
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to have coverage validation automatically detect features tagged with work unit IDs
    So that I don't need to manually run link-feature commands and coverage validation never gets skipped

  Scenario: Auto-discover features from @TAG when linkedFeatures is empty
    Given a work unit AUTH-001 with linkedFeatures: null
    When moving AUTH-001 to done status
    Then auto-discovery should find user-authentication.feature
    And a feature file spec/features/user-authentication.feature with @AUTH-001 tag
    And the feature has 3 scenarios with 0% coverage
    And coverage validation should run for all 3 scenarios
    And the status update should fail with coverage error

  Scenario: Explicit linkedFeatures takes precedence over @TAG auto-discovery
    Given a work unit AUTH-002 with linkedFeatures: ['custom-feature']
    When moving AUTH-002 to done status
    Then coverage validation should use custom-feature.feature
    And a feature file spec/features/user-login.feature with @AUTH-002 tag
    And a feature file spec/features/custom-feature.feature without @AUTH-002 tag
    And coverage validation should NOT use user-login.feature
    And explicit linkedFeatures should override auto-discovery

  Scenario: Task work units are exempt from coverage validation
    Given a work unit TASK-001 with type: 'task'
    When moving TASK-001 to done status
    Then auto-discovery should find no features
    And linkedFeatures: null
    And no feature files with @TASK-001 tag
    And the status update should succeed with warning
    And tasks should be exempt from coverage requirements

  Scenario: Block progression when auto-discovered feature has incomplete coverage
    Given a work unit BUG-042 with linkedFeatures: null
    When moving BUG-042 to done status
    Then auto-discovery should find auth-bug-fix.feature
    And a feature file spec/features/auth-bug-fix.feature with @BUG-042 tag
    And the feature has 2 scenarios
    And coverage shows 0% (0/2 scenarios covered)
    And coverage validation should detect 0% coverage
    And the status update should fail with coverage error
