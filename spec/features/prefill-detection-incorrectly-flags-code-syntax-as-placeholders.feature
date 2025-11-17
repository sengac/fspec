@validation
@validator
@BUG-079
Feature: Prefill detection incorrectly flags code syntax as placeholders
  """
  Architecture notes:
  - Uses prefill detection utilities in src/utils/prefill-detection.ts
  - Currently uses overly broad regex that matches ANY bracket syntax
  - Must be replaced with specific placeholder pattern matching
  - Affects workflow progression blocking in update-work-unit-status command
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Prefill detection MUST only match actual placeholder text patterns we generate: role, action, benefit, precondition, expected outcome, TODO markers
  #   2. Prefill detection MUST NOT match arbitrary bracket syntax like array access, TypeScript generics, or code examples
  #
  # EXAMPLES:
  #   1. Step 'Then only work-unit.ts should contain workUnitsData.workUnits with id = object logic' incorrectly flagged as containing placeholder
  #   2. Step 'Given I have role placeholder' should be flagged as containing placeholder
  #   3. Step 'When I access array with index in code' should NOT be flagged as placeholder
  #
  # ========================================
  Background: User Story
    As a AI agent writing Gherkin specifications
    I want to describe code syntax in steps without triggering false positive prefill detection
    So that I can write clear, accurate specifications without workarounds

  Scenario: Code syntax with brackets should not trigger prefill detection
    Given I have a feature file with step "Then only work-unit.ts should contain workUnitsData.workUnits with id = object logic"
    When I run prefill detection on the feature file
    Then prefill detection should NOT flag the step as containing placeholders
    And the step should be considered complete

  Scenario: Actual placeholder patterns should trigger prefill detection
    Given I have a feature file with step "Given I have role placeholder"
    When I run prefill detection on the feature file
    Then prefill detection SHOULD flag the step as containing placeholder "role"
    And the step should be considered incomplete

  Scenario: Array access syntax should not trigger prefill detection
    Given I have a feature file with step "When I access array with index in code"
    When I run prefill detection on the feature file
    Then prefill detection should NOT flag the step as containing placeholders
    And the step should be considered complete
