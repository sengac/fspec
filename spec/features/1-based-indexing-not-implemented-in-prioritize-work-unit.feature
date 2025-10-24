@high
@cli
@work-management
@validation
@BUG-042
Feature: 1-based indexing not implemented in prioritize-work-unit

  """
  Architecture notes:
  - Convert 1-based user input to 0-based array index: newIndex = options.position - 1
  - Validate position >= 1 before conversion (reject 0 or negative)
  - Help text documents 1-based indexing (position 1 = first item, position 3 = third item)
  - Error message must clearly state "1-based index" to guide users
  - Allow positions beyond array length (splice will insert at end as expected)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Help text says numeric positions are 1-based (position 1 = first item, position 3 = third item)
  #   2. Code must convert 1-based user input to 0-based array indices (position - 1)
  #   3. Position must be >= 1 (reject position 0 or negative)
  #
  # EXAMPLES:
  #   1. User runs 'fspec prioritize-work-unit AUTH-001 --position 1' expecting AUTH-001 to be first. AUTH-001 becomes first item (index 0).
  #   2. User runs 'fspec prioritize-work-unit AUTH-001 --position 3' expecting AUTH-001 to be third. AUTH-001 becomes third item (index 2).
  #   3. User runs 'fspec prioritize-work-unit AUTH-001 --position 0'. Command throws: 'Invalid position: 0. Position must be >= 1 (1-based index)'
  #
  # ========================================

  Background: User Story
    As a user running prioritize-work-unit with numeric positions
    I want to use intuitive 1-based positions matching help text
    So that position 3 means third item as documented, not fourth item

  Scenario: Position 1 means first item
    Given work units AUTH-001, AUTH-002, AUTH-003 are in backlog
    And they are ordered: AUTH-002, AUTH-003, AUTH-001
    When I run "fspec prioritize-work-unit AUTH-001 --position 1"
    Then the command should succeed
    And the backlog order should be: AUTH-001, AUTH-002, AUTH-003
    And AUTH-001 should be first in backlog

  Scenario: Position 3 means third item
    Given work units AUTH-001, AUTH-002, AUTH-003, AUTH-004 are in backlog
    And they are ordered: AUTH-002, AUTH-003, AUTH-004, AUTH-001
    When I run "fspec prioritize-work-unit AUTH-001 --position 3"
    Then the command should succeed
    And the backlog order should be: AUTH-002, AUTH-003, AUTH-001, AUTH-004
    And AUTH-001 should be third in backlog

  Scenario: Reject position 0 as invalid
    Given work unit AUTH-001 is in backlog
    When I run "fspec prioritize-work-unit AUTH-001 --position 0"
    Then the command should fail
    And the error message should contain "Invalid position: 0"
    And the error message should contain "Position must be >= 1 (1-based index)"

  Scenario: Reject negative position as invalid
    Given work unit AUTH-001 is in backlog
    When I run "fspec prioritize-work-unit AUTH-001 --position -1"
    Then the command should fail
    And the error message should contain "Invalid position: -1"
    And the error message should contain "Position must be >= 1 (1-based index)"
