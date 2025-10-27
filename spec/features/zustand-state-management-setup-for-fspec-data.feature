@typescript
@zustand
@interactive-cli
@tui
@high
@ITF-002
Feature: Zustand state management setup for fspec data
  """
  Critical: Store must provide selectors (useWorkUnitsByStatus, useWorkUnitsByEpic) for efficient filtering. State shape: { workUnits: [], epics: [], isLoaded: boolean, error: string | null }
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Store must load data from spec/work-units.json, spec/epics.json, spec/features/ directory
  #   2. Store must use Zustand with Immer for immutable state updates
  #   3. Store actions must integrate with existing loadWorkUnits, loadEpics utility functions
  #   4. Store must provide selectors for filtering work units by status, epic, prefix
  #   5. Store initialization must be async to load JSON files from disk
  #
  # EXAMPLES:
  #   1. Store initializes and loads 206 work units from work-units.json - workUnits array has 206 items
  #   2. Store loads 13 epics from epics.json - epics array has correct data structure with id, title, workUnits
  #   3. Component calls useWorkUnitsByStatus('backlog') - returns filtered array of work units in backlog status
  #   4. Component updates work unit status via store action - state updates immutably and component re-renders
  #   5. Store provides getWorkUnitsByEpic selector - returns work units filtered by epic ID
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the store poll for file changes or use file watchers for real-time updates?
  #   A: true
  #
  #   Q: Should store initialization happen automatically on import or require explicit init() call?
  #   A: true
  #
  #   Q: Should the store cache data in memory or reload from disk on every access?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer building interactive TUI
    I want to have Zustand store automatically load and sync all fspec data from JSON files
    So that the TUI can display real work units, epics, and features without manual data loading

  Scenario: Store loads work units from work-units.json
    Given the spec/work-units.json file exists with 206 work units
    When I call store.loadData()
    Then the workUnits array should have 206 items
    And isLoaded should be true

  Scenario: Store loads epics from epics.json
    Given the spec/epics.json file exists with 13 epics
    When I call store.loadData()
    Then the epics array should have 13 items
    And each epic should have id, title, and workUnits fields

  Scenario: Selector filters work units by status
    Given the store has loaded work units with various statuses
    When I call useWorkUnitsByStatus('backlog')
    Then I should receive only work units with status='backlog'

  Scenario: Selector filters work units by epic
    Given the store has loaded work units belonging to different epics
    When I call useWorkUnitsByEpic('interactive-tui-foundation')
    Then I should receive only work units with epic='interactive-tui-foundation'

  Scenario: Store updates are immutable
    Given the store has loaded work units
    When I call updateWorkUnitStatus('ITF-001', 'done')
    Then the work unit status should be updated
    And the original state object should not be mutated (Immer ensures immutability)
