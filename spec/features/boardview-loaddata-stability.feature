@BUG-119
Feature: BoardView loadData stability
  """
  Changes span 3 files: fspecStore.ts (in-flight guard + error handling), globalStreamListener.ts (JS debounce), BoardView.tsx (remove redundant loadData after moves)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Lock contention errors (ELOCKED / 'Lock file is already being held') must be treated as transient and must NOT set the error state or trigger an ErrorView render
  #   2. globalStreamListener must debounce loadData() calls on the JavaScript side (on top of the Rust 100ms debounce) so that rapid watcher events coalesce into a single load
  #   3. Concurrent loadData() calls must be prevented — if a loadData() is already in-flight, subsequent calls must be skipped (not queued)
  #   4. loadData() must NOT clear the error state before attempting to read — error clearing must only happen on successful read
  #   5. Explicit loadData() calls after moveWorkUnitUp/Down in BoardView must be removed — the file watcher already triggers a debounced loadData() after the write
  #   6. All existing TUI-079 correctness behavior must be preserved — loadData() path, session context sync, deleted work unit detection, ordering from states arrays
  #
  # EXAMPLES:
  #   1. AI agent writes to work-units.json (holding write lock) while watcher fires loadData() → loadData() gets ELOCKED error → board remains stable, no error screen flash, watcher retries after debounce succeeds
  #   2. User presses ] to move work unit down → work unit moves in the board, watcher fires once (debounced), no double-load or lock contention
  #   3. Multiple rapid external changes to work-units.json within 200ms → single loadData() call executes after debounce, board updates once smoothly
  #   4. loadData() is already running when watcher triggers another → second call is silently skipped, no lock contention
  #   5. Real errors like permission denied or disk full still show the ErrorView — only lock contention is silenced
  #
  # ========================================
  Background: User Story
    As a developer
    I want to see a stable BoardView without flickering when work-units.json changes externally
    So that I can work on the board without visual disruption

  @unit
  Scenario: Lock contention error does not trigger ErrorView
    Given the fspec store has loaded work units successfully
    When loadData() encounters a "Lock file is already being held" error
    Then the store error state must remain null
    And the existing work units must remain unchanged in the store
    And the error is logged at debug level as transient

  @unit
  Scenario: Lock contention does not clear prior successful state
    Given the fspec store has loaded work units successfully
    And the store has 5 work units displayed
    When loadData() is called and encounters a lock contention error
    Then the store must still have 5 work units
    And isLoaded must remain true

  @unit
  Scenario: Real errors still set error state
    Given the fspec store has loaded work units successfully
    When loadData() encounters a permission denied error
    Then the store error state must be set with the error details
    And the ErrorView should be displayed

  @unit
  Scenario: loadData clears error only on success not before reading
    Given the fspec store has a previous error state set
    When loadData() is called and succeeds
    Then the error state must be cleared after the read completes
    And the error state must not be cleared before attempting the read

  @unit
  Scenario: Concurrent loadData calls are prevented by in-flight guard
    Given loadData() is already in-flight
    When another loadData() call is triggered
    Then the second call must return immediately without acquiring any locks
    And only one loadData execution is running at any time

  @unit
  Scenario: globalStreamListener debounces WorkUnitsUpdate events
    Given the globalStreamListener is initialized
    When 3 WorkUnitsUpdate events arrive within 150ms
    Then loadData() must be called exactly once
    And the call must happen after the debounce period elapses

  @unit
  Scenario: Debounce timer resets on each new event
    Given the globalStreamListener is initialized
    And a WorkUnitsUpdate event arrived 50ms ago
    When another WorkUnitsUpdate event arrives
    Then the debounce timer must reset
    And loadData() must not be called until the debounce period elapses from the latest event

  @unit
  Scenario: Session context sync preserved after debounced loadData
    Given a session is attached to work unit AUTH-001
    And AUTH-001 status changes externally from backlog to implementing
    When the debounced loadData completes
    Then the session store must update currentWorkUnitStatus to implementing
    And Rust context must be updated with the new status
