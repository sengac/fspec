@foundation-management
@event-storming
@file-ops
@EXMAP-010
Feature: Big Picture Event Storm in foundation.json
  """
  Implementation:
  - Extend GenericFoundation type with optional eventStorm: FoundationEventStorm field. Use EventStormBase interface shared with work unit EventStorm. Commands: add-foundation-bounded-context, show-foundation-event-storm. Use fileManager.transaction() for atomic updates. Update generic-foundation.schema.json. NO semantic code - structural data only.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Foundation Event Storm must use level='big_picture' (not 'process_modeling' or 'software_design')
  #   2. All Event Storm items must use same EventStormItem union type as work unit-level for consistency
  #   3. Foundation Event Storm is OPTIONAL in foundation.json (not required for all projects)
  #   4. Commands must NOT include semantic logic (tag suggestion, classification, inference) - zero-semantics principle
  #   5. Deleted items (deleted=true) must be filtered out when showing Event Storm data
  #   6. nextItemId must auto-increment for each new item to ensure stable IDs
  #   7. Foundation Event Storm commands must use fileManager.transaction() for atomic updates
  #
  # EXAMPLES:
  #   1. Add bounded context 'User Management' to foundation Event Storm → creates item with type='bounded_context', text='User Management', level='big_picture', deleted=false
  #   2. Show foundation Event Storm filtered by type='bounded_context' → returns only bounded context items, excludes deleted items
  #   3. Foundation.json has no eventStorm section → add-foundation-bounded-context initializes eventStorm with level='big_picture', items=[], nextItemId=1
  #   4. Foundation Event Storm has 3 items, 1 deleted → show-foundation-event-storm returns 2 active items only
  #
  # ========================================
  Background: User Story
    As a AI agent performing Big Picture Event Storming
    I want to capture foundation-level bounded contexts, pivotal events, and major aggregates in foundation.json
    So that I can understand the strategic domain structure before diving into tactical work units

  Scenario: Add bounded context to foundation with no existing Event Storm
    Given foundation.json has no eventStorm section
    When I run "fspec add-foundation-bounded-context 'User Management'"
    Then the eventStorm section should be created with level='big_picture'
    And the eventStorm should have items array and nextItemId=1
    And a bounded context item should be added with type='bounded_context'
    And the bounded context text should be "User Management"
    And the bounded context deleted flag should be false

  Scenario: Show foundation Event Storm filtered by type
    Given foundation.json has Event Storm with 2 bounded contexts and 1 aggregate
    And all items have deleted=false
    When I run "fspec show-foundation-event-storm --type=bounded_context"
    Then the output should be valid JSON
    And the JSON should contain 2 items
    And all items should have type='bounded_context'
    And no deleted items should be included

  Scenario: Initialize Event Storm section when missing
    Given foundation.json exists without eventStorm section
    When I add a bounded context using add-foundation-bounded-context
    Then eventStorm section should be initialized
    And eventStorm.level should equal "big_picture"
    And eventStorm.items should be an empty array initially
    And eventStorm.nextItemId should equal 1
    And the new bounded context should be added to items array

  Scenario: Filter out deleted items when showing Event Storm
    Given foundation Event Storm has 3 items total
    And 1 item has deleted=true
    And 2 items have deleted=false
    When I run "fspec show-foundation-event-storm"
    Then the output should contain exactly 2 items
    And no items should have deleted=true
    And all returned items should have deleted=false
