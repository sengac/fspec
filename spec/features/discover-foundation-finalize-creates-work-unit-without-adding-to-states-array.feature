@work-unit-management
@cli
@BUG-078
Feature: discover-foundation --finalize creates work unit without adding to states array
  """
  DRY/SOLID Violation Pattern:
  - work-unit.ts:212 has centralized createWorkUnit() function with proper states.backlog.push()
  - create-story.ts:110, create-bug.ts:110, create-task.ts:110 duplicate this logic
  - discover-foundation.ts:401 duplicates AND forgets states.backlog.push()

  Impact Chain:
  - Missing states array entry → TUI fspecStore filters it out → Work unit invisible in Kanban

  Fix Strategy:
  - Extract createWorkUnit() as reusable function
  - All commands (create-story, create-bug, create-task, discover-foundation) must import and call it
  - Remove duplicated work unit creation logic from all violating files
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Work unit creation logic must be centralized - all commands must call createWorkUnit() from work-unit.ts
  #   2. All work unit creation commands (create-story, create-bug, create-task, discover-foundation) must call createWorkUnit() instead of duplicating logic
  #   3. states.backlog array must be updated atomically with workUnits object to prevent TUI display bugs
  #
  # EXAMPLES:
  #   1. discover-foundation.ts:401 manually creates work unit object and FORGETS to add to states.backlog array - causing FOUND-022 to be invisible in TUI
  #   2. create-story.ts:110, create-bug.ts:110, create-task.ts:110 all DUPLICATE the logic from work-unit.ts:212 - violating DRY principle
  #   3. work-unit.ts:212 has the CORRECT logic that adds to states.backlog array - this should be the ONLY place this logic exists (Single Responsibility Principle)
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to create work units through any command
    So that all work units appear in the TUI Kanban board without missing entries

  Scenario: discover-foundation --finalize creates work unit without states array entry
    Given I have a project with foundation.json.draft file
    When I run `fspec discover-foundation --finalize`
    Then a work unit with prefix "FOUND" should be created
    And the work unit should exist in workUnits object
    And the work unit ID should be added to states.backlog array
    And the work unit should be visible in the TUI Kanban board

  Scenario: create-story, create-bug, create-task must call createWorkUnit()
    Given I have registered prefixes for story, bug, and task work units
    When I run `fspec create-story TEST "Test Story"`
    Then the createWorkUnit() function should be called
    And the work unit should exist in workUnits object
    And the work unit ID should be added to states.backlog array
    When I run `fspec create-bug BUG "Test Bug"`
    Then the createWorkUnit() function should be called
    And the work unit should exist in workUnits object
    And the work unit ID should be added to states.backlog array
    When I run `fspec create-task TASK "Test Task"`
    Then the createWorkUnit() function should be called
    And the work unit should exist in workUnits object
    And the work unit ID should be added to states.backlog array

  Scenario: work-unit.ts createWorkUnit() is the single source of truth
    Given all work unit creation commands exist (create-story, create-bug, create-task, discover-foundation)
    When I analyze the codebase for work unit creation logic
    Then only work-unit.ts should contain work unit object assignment logic
    And only work-unit.ts should contain states backlog array push logic
    And all other commands should import and call createWorkUnit()
    And no code duplication should exist for work unit creation
