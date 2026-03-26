@BUG-096
Feature: Missing remove commands for foundation event storm artifacts

  """
  Follow existing add command file structure: implementation + help in src/commands/, registration import in src/cli/program.ts
  Use fileManager.transaction() for atomic JSON mutations — same pattern as add-foundation-bounded-context.ts
  Child items link to parent via boundedContextId field — use this for cascade detection and deletion
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All 4 remove commands use soft-delete (set deleted: true) — consistent with ItemWithId pattern
  #   2. Removing a non-empty bounded context without --cascade must fail with an error listing child counts
  #   3. Removing a bounded context with --cascade soft-deletes the context AND all child items (aggregates, events, commands with matching boundedContextId)
  #   4. After every removal, FOUNDATION.md must be regenerated via generateFoundationMdCommand
  #   5. Removing an item that does not exist must fail with a clear error message
  #   6. Each remove command must use fileManager.transaction for atomic updates — same pattern as add commands
  #   7. Each remove command must be registered in src/cli/program.ts and have a help file
  #   8. Remove commands for aggregates, events, and commands require both <context-name> and <item-name> arguments — matching the add command signature
  #
  # EXAMPLES:
  #   1. User removes an empty bounded context → it gets soft-deleted, show-foundation-event-storm no longer lists it
  #   2. User tries to remove bounded context with 3 aggregates and 2 events without --cascade → error shows '5 child items' and suggests --cascade
  #   3. User removes bounded context with --cascade → context + all its aggregates, events, commands are soft-deleted
  #   4. User removes aggregate 'WorkUnit' from 'Work Management' → only that aggregate is soft-deleted, other items unaffected
  #   5. User removes domain event 'WorkUnitCreated' from 'Work Management' → event soft-deleted
  #   6. User removes command 'CreateWorkUnit' from 'Work Management' → command soft-deleted
  #   7. User tries to remove aggregate from non-existent bounded context → error 'Bounded context X not found'
  #   8. User tries to remove non-existent aggregate → error 'Aggregate X not found in context Y'
  #   9. After removing items, FOUNDATION.md is regenerated and no longer shows the removed items
  #
  # ========================================

  Background: User Story
    As a developer using foundation event storm
    I want to remove bounded contexts, aggregates, domain events, and commands from the foundation event storm
    So that correct mistakes and refactor domain architecture without manually editing JSON

  Scenario: Remove an empty bounded context
    Given a foundation.json with a bounded context "Payments" and no child items
    When I run "fspec remove-foundation-bounded-context Payments"
    Then the command should succeed with a confirmation message
    And the bounded context "Payments" should have deleted set to true in foundation.json
    And "fspec show-foundation-event-storm" should not list "Payments"

  Scenario: Refuse to remove non-empty bounded context without cascade flag
    Given a foundation.json with a bounded context "Work Management" containing 3 aggregates and 2 events
    When I run "fspec remove-foundation-bounded-context 'Work Management'"
    Then the command should fail with an error mentioning "5 child items"
    And the error should suggest using the --cascade flag
    And the bounded context "Work Management" should still have deleted set to false

  Scenario: Remove non-empty bounded context with cascade flag
    Given a foundation.json with a bounded context "Work Management" containing aggregates, events, and commands
    When I run "fspec remove-foundation-bounded-context 'Work Management' --cascade"
    Then the command should succeed
    And the bounded context "Work Management" should have deleted set to true
    And all aggregates with boundedContextId matching "Work Management" should have deleted set to true
    And all events with boundedContextId matching "Work Management" should have deleted set to true
    And all commands with boundedContextId matching "Work Management" should have deleted set to true

  Scenario: Remove aggregate from bounded context
    Given a foundation.json with bounded context "Work Management" containing aggregate "WorkUnit" and aggregate "Epic"
    When I run "fspec remove-aggregate-from-foundation 'Work Management' WorkUnit"
    Then the command should succeed with a confirmation message
    And the aggregate "WorkUnit" should have deleted set to true
    And the aggregate "Epic" should still have deleted set to false

  Scenario: Remove domain event from bounded context
    Given a foundation.json with bounded context "Work Management" containing domain event "WorkUnitCreated"
    When I run "fspec remove-domain-event-from-foundation 'Work Management' WorkUnitCreated"
    Then the command should succeed with a confirmation message
    And the domain event "WorkUnitCreated" should have deleted set to true

  Scenario: Remove command from bounded context
    Given a foundation.json with bounded context "Work Management" containing command "CreateWorkUnit"
    When I run "fspec remove-command-from-foundation 'Work Management' CreateWorkUnit"
    Then the command should succeed with a confirmation message
    And the command "CreateWorkUnit" should have deleted set to true

  Scenario: Error when removing aggregate from non-existent bounded context
    Given a foundation.json with no bounded context named "Payments"
    When I run "fspec remove-aggregate-from-foundation Payments WorkUnit"
    Then the command should fail with an error containing "Bounded context 'Payments' not found"

  Scenario: Error when removing non-existent aggregate
    Given a foundation.json with bounded context "Work Management" containing no aggregate named "Foo"
    When I run "fspec remove-aggregate-from-foundation 'Work Management' Foo"
    Then the command should fail with an error containing "Aggregate 'Foo' not found"

  Scenario: FOUNDATION.md regenerated after removal
    Given a foundation.json with a bounded context "Payments" and no child items
    And FOUNDATION.md contains a reference to "Payments"
    When I run "fspec remove-foundation-bounded-context Payments"
    Then the command should succeed
    And FOUNDATION.md should be regenerated
    And FOUNDATION.md should not contain "Payments"
