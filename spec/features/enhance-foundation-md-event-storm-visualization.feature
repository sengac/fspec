@markdown
@event-storm
@foundation-management
@generator
@high
@FOUND-042
Feature: Enhance FOUNDATION.md Event Storm visualization
  """
  File location: src/generators/foundation-md.ts (lines 180-223). Enhancement requires filtering Event Storm items by boundedContextId and generating markdown sections for each bounded context showing aggregates, events, and commands with their descriptions. Uses existing foundation.eventStorm.items array as data source.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each bounded context must have a dedicated section in FOUNDATION.md
  #   2. Aggregates must be listed under each bounded context with their descriptions
  #   3. Domain events must be listed under each bounded context with their descriptions
  #   4. Commands must be listed under each bounded context with their descriptions
  #   5. Items must only appear under their parent bounded context (filter by boundedContextId)
  #   6. Deleted items must be excluded from the visualization
  #
  # EXAMPLES:
  #   1. Work Management context shows 4 aggregates: WorkUnit, Epic, Dependency, Prefix with descriptions
  #   2. Work Management context shows 4 events: WorkUnitCreated, WorkUnitStatusChanged, WorkUnitBlocked, DependencyAdded with descriptions
  #   3. Work Management context shows 4 commands: CreateWorkUnit, UpdateWorkUnitStatus, BlockWorkUnit, AddDependency with descriptions
  #   4. All 6 bounded contexts have their own sections after the Bounded Context Map diagram
  #   5. Deleted items (deleted: true) do not appear in the output
  #
  # ========================================
  Background: User Story
    As a developer viewing foundation architecture
    I want to see complete Event Storm visualization in FOUNDATION.md
    So that I understand the domain model with aggregates, events, and commands for each bounded context

  Scenario: Render bounded context with aggregates, events, and commands
    Given foundation.json has Work Management context with 4 aggregates, 4 events, 4 commands
    When I run generate-foundation-md
    Then FOUNDATION.md should have a 'Work Management Context' section
    And the section should list 4 aggregates with descriptions
    And the section should list 4 domain events with descriptions
    And the section should list 4 commands with descriptions

  Scenario: Render all bounded contexts with complete sections
    Given foundation.json has 6 bounded contexts with Event Storm items
    When I run generate-foundation-md
    Then FOUNDATION.md should have 6 context sections after the Bounded Context Map
    And each section should show aggregates, events, and commands for that context only

  Scenario: Exclude deleted items from visualization
    Given foundation.json has Work Management context with 1 deleted aggregate
    When I run generate-foundation-md
    Then the deleted aggregate should not appear in FOUNDATION.md
    And only non-deleted items should be listed
