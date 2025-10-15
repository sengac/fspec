@done
@workflow-automation
@hooks
@phase1
@HOOK-005
Feature: Hook condition evaluation

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # USER STORY:
  #   As a hook execution system
  #   I want to evaluate hook conditions to determine if a hook should run
  #   So that hooks only execute when their conditions match the current context
  #
  # BUSINESS RULES:
  #   1. Hooks with no condition always match (condition is optional)
  #   2. Condition with tags: hook runs if work unit has ANY of the specified tags (OR logic)
  #   3. Condition with prefix: hook runs if work unit ID starts with ANY of the specified prefixes (OR logic)
  #   4. Condition with epic: hook runs if work unit belongs to specified epic
  #   5. Condition with estimateMin/estimateMax: hook runs if work unit estimate is within range
  #   6. Multiple condition fields use AND logic (all must match)
  #   7. If context has no workUnitId, only hooks without conditions run
  #
  # EXAMPLES:
  #   1. Hook with no condition matches any context - always runs
  #   2. Hook with tags:[@security] matches work unit with @security tag
  #   3. Hook with prefix:[AUTH,SEC] matches AUTH-001 but not DASH-001
  #   4. Hook with epic:'user-management' matches work unit in that epic
  #   5. Hook with estimateMin:5, estimateMax:13 matches work unit with estimate 8
  #   6. Hook with tags:[@security] AND prefix:[AUTH] - both must match (AND logic)
  #   7. Context with no workUnitId - only unconditional hooks match
  #
  # ========================================
  Background: User Story
    As a hook execution system
    I want to evaluate hook conditions to determine if a hook should run
    So that hooks only execute when their conditions match the current context

  Scenario: Hook with no condition always matches
    Given I have a hook with no condition
    And I have a work unit context "AUTH-001"
    When I evaluate if the hook should run
    Then the hook should match
    And the hook should be included in execution

  Scenario: Hook with tag condition matches work unit with matching tag
    Given I have a hook with condition tags ["@security"]
    And I have a work unit "AUTH-001" with tags ["@security", "@critical"]
    When I evaluate if the hook should run
    Then the hook should match
    And the hook should be included in execution

  Scenario: Hook with tag condition does not match work unit without tag
    Given I have a hook with condition tags ["@security"]
    And I have a work unit "DASH-001" with tags ["@ui", "@phase1"]
    When I evaluate if the hook should run
    Then the hook should not match
    And the hook should be excluded from execution

  Scenario: Hook with prefix condition matches work unit with matching prefix
    Given I have a hook with condition prefix ["AUTH", "SEC"]
    And I have a work unit "AUTH-001"
    When I evaluate if the hook should run
    Then the hook should match
    And the hook should be included in execution

  Scenario: Hook with prefix condition does not match work unit with different prefix
    Given I have a hook with condition prefix ["AUTH", "SEC"]
    And I have a work unit "DASH-001"
    When I evaluate if the hook should run
    Then the hook should not match
    And the hook should be excluded from execution

  Scenario: Hook with epic condition matches work unit in that epic
    Given I have a hook with condition epic "user-management"
    And I have a work unit "AUTH-001" in epic "user-management"
    When I evaluate if the hook should run
    Then the hook should match
    And the hook should be included in execution

  Scenario: Hook with estimate range matches work unit within range
    Given I have a hook with condition estimateMin 5 and estimateMax 13
    And I have a work unit "AUTH-001" with estimate 8
    When I evaluate if the hook should run
    Then the hook should match
    And the hook should be included in execution

  Scenario: Hook with multiple conditions uses AND logic
    Given I have a hook with condition tags ["@security"] and prefix ["AUTH"]
    And I have a work unit "AUTH-001" with tags ["@security"]
    When I evaluate if the hook should run
    Then the hook should match because both conditions are met
    And the hook should be included in execution

  Scenario: Hook with multiple conditions fails if any condition is not met
    Given I have a hook with condition tags ["@security"] and prefix ["AUTH"]
    And I have a work unit "DASH-001" with tags ["@security"]
    When I evaluate if the hook should run
    Then the hook should not match because prefix does not match
    And the hook should be excluded from execution

  Scenario: Context without work unit ID only matches unconditional hooks
    Given I have a hook with condition tags ["@security"]
    And I have a context with no work unit ID
    When I evaluate if the hook should run
    Then the hook should not match
    And the hook should be excluded from execution
