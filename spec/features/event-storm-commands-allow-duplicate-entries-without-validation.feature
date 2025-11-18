@done
@cli
@critical
@event-storm
@validation
@BUG-087
Feature: Event Storm commands allow duplicate entries without validation

  """
  Check for duplicates in add-domain-event command before adding. Search existing non-deleted events with case-insensitive text match. If found, throw error with event ID. Apply same logic to add-command, add-policy, add-hotspot.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Prevent duplicate Event Storm entries by default (case-insensitive check)
  #   2. Throw error with message: Event 'X' already exists (ID: N)
  #   3. Only check non-deleted entries for duplicates
  #
  # EXAMPLES:
  #   1. add-domain-event UI-001 'EventA' succeeds, add-domain-event UI-001 'EventA' throws error
  #   2. add-domain-event UI-001 'EventA' succeeds, add-domain-event UI-001 'eventa' (lowercase) throws error (case-insensitive)
  #   3. add-domain-event UI-001 'EventA', delete event 0, add-domain-event UI-001 'EventA' succeeds (deleted entries ignored)
  #
  # ========================================

  Background: User Story
    As a developer using Event Storm commands
    I want to prevent duplicate entries when adding the same text
    So that Event Storm data stays clean and scenarios generate correctly

  Scenario: Prevent duplicate domain event
    Given a work unit "TEST-001" exists with Event Storm
    And I add domain event "EventA"
    When I try to add domain event "EventA" again
    Then an error should be thrown
    And the error should say "Event 'EventA' already exists"

  Scenario: Prevent duplicate with case-insensitive check
    Given a work unit "TEST-002" exists with Event Storm
    And I add domain event "EventA"
    When I try to add domain event "eventa" (lowercase)
    Then an error should be thrown
    And the error should say "Event 'eventa' already exists"

  Scenario: Allow same text after deletion
    Given a work unit "TEST-003" exists with Event Storm
    And I add domain event "EventA"
    And I delete the event
    When I try to add domain event "EventA" again
    Then the command should succeed
    And a new event should be created
