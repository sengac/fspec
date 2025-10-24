@high
@cli
@work-management
@validation
@BUG-041
Feature: Data integrity validation missing in prioritize-work-unit

  """
  Architecture notes:
  - Add validation check before prioritizing work unit to ensure workUnit.status matches the states array it's in
  - Use .includes() to check if work unit ID exists in states[workUnit.status] array
  - Throw error with repair-work-units suggestion if data is corrupted
  - Validate --before and --after targets are in the correct states array (same as source work unit)
  - Use .filter() instead of splice to safely remove work unit from array (handles duplicates)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Work unit status field MUST match which states array it's in (e.g., status='specifying' must be in states.specifying)
  #   2. If work unit status doesn't match its array location, throw error with repair suggestion
  #   3. Validate work unit's own status matches its array before prioritizing
  #   4. When using --before or --after, validate target work unit is in the same states array
  #
  # EXAMPLES:
  #   1. AUTH-001 has status='specifying' but is in states.testing array. User runs 'fspec prioritize-work-unit AUTH-001 --position top'. Command throws: 'Data integrity error: Work unit AUTH-001 has status specifying but is not in states.specifying array. Run fspec repair-work-units'
  #   2. FEAT-017 is in specifying (correct). User runs 'fspec prioritize-work-unit FEAT-017 --before AUTH-001' where AUTH-001 status='specifying' but AUTH-001 is NOT in states.specifying. Command throws: 'Data integrity error: Work unit AUTH-001 has status specifying but is not in states.specifying array'
  #   3. Work unit AUTH-001 has status='implementing' and IS in states.implementing array. User runs 'fspec prioritize-work-unit AUTH-001 --position top'. Command succeeds (data is valid)
  #
  # ========================================

  Background: User Story
    As a developer using prioritize-work-unit
    I want to detect data corruption early with clear error messages
    So that I can fix corrupted data before it causes silent failures or duplicates

  Scenario: Detect work unit in wrong states array
    Given work unit AUTH-001 has status "specifying"
    But AUTH-001 is in the states.testing array instead of states.specifying
    When I run "fspec prioritize-work-unit AUTH-001 --position top"
    Then the command should fail
    And the error message should contain "Data integrity error"
    And the error message should contain "AUTH-001 has status 'specifying' but is not in states.specifying array"
    And the error message should contain "Run 'fspec repair-work-units' to fix data corruption"

  Scenario: Detect --before target in wrong states array
    Given work unit FEAT-017 is in specifying status
    And FEAT-017 is correctly in the states.specifying array
    And work unit AUTH-001 has status "specifying"
    But AUTH-001 is NOT in the states.specifying array
    When I run "fspec prioritize-work-unit FEAT-017 --before AUTH-001"
    Then the command should fail
    And the error message should contain "Data integrity error"
    And the error message should contain "AUTH-001 has status 'specifying' but is not in states.specifying array"

  Scenario: Valid data passes integrity check
    Given work unit AUTH-001 has status "implementing"
    And AUTH-001 is correctly in the states.implementing array
    When I run "fspec prioritize-work-unit AUTH-001 --position top"
    Then the command should succeed
    And AUTH-001 should be first in the states.implementing array
