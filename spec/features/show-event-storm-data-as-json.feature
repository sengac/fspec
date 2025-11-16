@EXMAP-011
Feature: Show Event Storm data as JSON

  """
  Implementation:
  - Read work-units.json, extract eventStorm.items array, filter deleted=false, output as JSON. Use existing WorkUnitsData and EventStorm types. Command: fspec show-event-storm <work-unit-id> [--format=json|pretty]
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Command must output Event Storm data as valid JSON to stdout
  #   2. Must include all Event Storm item types: bounded_context, aggregate, event, command, policy, hotspot, external_system
  #   3. Must filter out deleted items (deleted: true)
  #   4. Output structure must match EventStorm type from types/index.ts
  #   5. Must fail gracefully if work unit has no Event Storm data
  #   6. No semantic interpretation - just return raw structural data
  #
  # EXAMPLES:
  #   1. Work unit AUTH-001 with 3 events (UserLoggedIn, UserRegistered, PasswordChanged) outputs JSON array with 3 event objects
  #   2. Work unit with bounded context 'User Management' outputs JSON with bounded_context object
  #   3. Work unit with no Event Storm data returns error message
  #   4. Work unit with deleted items (deleted: true) excludes them from output
  #
  # ========================================

  Background: User Story
    As a AI agent analyzing Event Storm sessions
    I want to query Event Storm artifacts as structured JSON data
    So that I can use my language understanding to interpret domain semantics and suggest appropriate tags

  Scenario: Output Event Storm events as JSON array
    Given work unit "TEST-001" has 3 events: "UserLoggedIn", "UserRegistered", "PasswordChanged"
    And the events are not deleted
    When I run "fspec show-event-storm TEST-001"
    Then the output should be valid JSON
    And the JSON should contain an array with 3 event objects
    And each event object should have "type" as "event"
    And each event object should have a "text" field matching the event name

  Scenario: Output Event Storm bounded context as JSON object
    Given work unit "DOMAIN-001" has a bounded context named "User Management"
    And the bounded context is not deleted
    When I run "fspec show-event-storm DOMAIN-001"
    Then the output should be valid JSON
    And the JSON should contain a bounded_context object
    And the object should have "type" as "bounded_context"
    And the object should have "text" as "User Management"

  Scenario: Handle work unit with no Event Storm data
    Given work unit "EMPTY-001" has no Event Storm items
    When I run "fspec show-event-storm EMPTY-001"
    Then the command should exit with error
    And the error message should indicate no Event Storm data exists

  Scenario: Filter out deleted Event Storm items
    Given work unit "MIX-001" has 5 Event Storm items
    And 2 items have "deleted" set to true
    And 3 items have "deleted" set to false
    When I run "fspec show-event-storm MIX-001"
    Then the output should be valid JSON
    And the JSON should contain exactly 3 items
    And no deleted items should be included in the output
