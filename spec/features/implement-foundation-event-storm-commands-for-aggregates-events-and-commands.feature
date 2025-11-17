@foundation-management
@critical
@setup
@foundation
@event-storm
@cli
@FOUND-033
Feature: Implement Foundation Event Storm Commands for Aggregates, Events, and Commands
  """
  Three new commands to implement: add-aggregate-to-foundation, add-domain-event-to-foundation, add-command-to-foundation. Each command creates an item in foundation.json eventStorm.items array with boundedContextId linking to parent context. Commands follow same pattern as work unit Event Storm commands. Implementation requires finding bounded context by name, validating it exists, creating new item with auto-incremented ID, and saving atomically. Support optional --description flag for all three commands. Update show-foundation-event-storm to support --context and --type filtering. Update generate-foundation-md to render Event Storm section with bounded contexts and their aggregates/events/commands in FOUNDATION.md.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each aggregate must be linked to a specific bounded context
  #   2. Domain events must be linked to a specific bounded context
  #   3. Commands must be linked to a specific bounded context
  #   4. Bounded context must exist before adding aggregates, events, or commands to it
  #   5. Foundation Event Storm uses same data structure as work unit Event Storm (foundation.json eventStorm field)
  #   6. Commands must follow pattern: add-<type>-to-foundation <context> <name>
  #   7. Yes, support optional --description flag to match work unit Event Storm pattern
  #   8. Yes, generate-foundation-md should include Event Storm section showing bounded contexts with their aggregates, events, and commands
  #
  # EXAMPLES:
  #   1. Developer runs 'fspec add-aggregate-to-foundation "Work Management" "WorkUnit"' and it creates item with type='aggregate' linked to Work Management context
  #   2. Developer runs 'fspec add-domain-event-to-foundation "Work Management" "WorkUnitCreated"' and it creates item with type='domain_event' linked to Work Management context
  #   3. Developer runs 'fspec add-command-to-foundation "Work Management" "CreateWorkUnit"' and it creates item with type='command' linked to Work Management context
  #   4. Developer tries to add aggregate to non-existent context 'Foo' and gets error: Bounded context 'Foo' not found
  #   5. Developer runs 'fspec show-foundation-event-storm --context="Work Management"' and sees only items for that context (bounded context, aggregates, events, commands)
  #   6. Developer runs 'fspec show-foundation-event-storm --type=aggregate' and sees only aggregate items across all contexts
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we support optional --description flag for aggregates, events, and commands?
  #   A: true
  #
  #   Q: Should generate-foundation-md include Event Storm visualization (aggregates, events, commands) in the output?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer conducting Big Picture Event Storming
    I want to add aggregates, domain events, and commands to foundation bounded contexts
    So that I can complete the domain architecture model and generate comprehensive FOUNDATION.md documentation

  Scenario: Add aggregate to existing bounded context
    Given a foundation with bounded context "Work Management"
    When I run 'fspec add-aggregate-to-foundation "Work Management" "WorkUnit"'
    Then a new item with type 'aggregate' should be created in foundation.json
    And the item should have text "WorkUnit"
    And the item should be linked to "Work Management" bounded context via boundedContextId

  Scenario: Add domain event to existing bounded context
    Given a foundation with bounded context "Work Management"
    When I run 'fspec add-domain-event-to-foundation "Work Management" "WorkUnitCreated"'
    Then a new item with type 'domain_event' should be created in foundation.json
    And the item should have text "WorkUnitCreated"
    And the item should be linked to "Work Management" bounded context

  Scenario: Add command to existing bounded context
    Given a foundation with bounded context "Work Management"
    When I run 'fspec add-command-to-foundation "Work Management" "CreateWorkUnit"'
    Then a new item with type 'command' should be created in foundation.json
    And the item should have text "CreateWorkUnit"
    And the item should be linked to "Work Management" bounded context

  Scenario: Add aggregate to non-existent bounded context
    Given a foundation without bounded context "Foo"
    When I run 'fspec add-aggregate-to-foundation "Foo" "Bar"'
    Then the command should fail with error
    And the error message should contain "Bounded context 'Foo' not found"

  Scenario: Filter Event Storm by bounded context
    Given a foundation with multiple bounded contexts and items
    When I run 'fspec show-foundation-event-storm --context="Work Management"'
    Then only items for "Work Management" context should be displayed
    And this includes the bounded context itself, aggregates, events, and commands

  Scenario: Filter Event Storm by item type
    Given a foundation with multiple item types (bounded contexts, aggregates, events, commands)
    When I run 'fspec show-foundation-event-storm --type=aggregate'
    Then only aggregate items should be displayed
    And items from all bounded contexts should be included
