@TUI-069
Feature: Fix TUI-068 code smells: error handling, console statements, facade completion

  """
  Architecture notes:
  - sessionService provides facade functions (attachToWorkUnit, detachFromWorkUnit, destroySession, getAttachedWorkUnit) that orchestrate updates to fspecStore and sessionStore atomically
  - Error handling with rollback: if any step fails during attach/detach, previous state is restored to maintain consistency
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. attachToWorkUnit and detachFromWorkUnit must have error handling with rollback on failure
  #   2. fspecStore.ts must use logger instead of console.* for error output
  #   3. sessionService must export getAttachedWorkUnit() to complete the facade pattern
  #   4. AgentView must use getAttachedWorkUnit from sessionService, NOT direct useFspecStore selector
  #   5. attachToWorkUnit accepts optional title parameter - callers should pass actual work unit title
  #
  # EXAMPLES:
  #   1. attachToWorkUnit rollback: If setWorkUnitContext throws, both fspecStore.attachSession and sessionStore.setCurrentWorkUnit are rolled back to previous state
  #   2. console.error replaced: fspecStore.ts loadData error handler uses logger.error instead of console.error
  #   3. facade completion: AgentView imports getAttachedWorkUnit from sessionService and removes direct useFspecStore(state => state.getWorkUnitBySession) selector
  #   4. title parameter: All attachToWorkUnit calls in AgentView now pass workUnit?.title as the fourth argument
  #
  # ========================================

  Background: User Story
    As a developer
    I want to have robust session-work unit state management with proper error handling
    So that state remains consistent even when operations fail

  @facade-pattern
  Scenario: sessionService exports getAttachedWorkUnit
    Given I inspect sessionService.ts exports
    Then getAttachedWorkUnit should be exported
    And getAttachedWorkUnit should accept a sessionId parameter
    And getAttachedWorkUnit should return the attached work unit ID or undefined

  @api-enhancement
  Scenario: attachToWorkUnit accepts optional title parameter
    Given I have an unattached session "session-123"
    When I call attachToWorkUnit with title "My Work Unit Title"
    Then setWorkUnitContext should receive the title "My Work Unit Title"

  @api-enhancement
  Scenario: attachToWorkUnit defaults title to workUnitId when not provided
    Given I have an unattached session "session-123"
    When I call attachToWorkUnit without a title parameter
    Then setWorkUnitContext should receive the workUnitId as the title
