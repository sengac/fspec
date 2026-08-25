@done
@state-management
@tui
@bug-fix
@TUI-079
Feature: BoardView does not fully update when work-units.json changes — globalStreamListener uses lossy updateWorkUnitsFromWatcher path
  """
  Fix is in globalStreamListener.ts handleStreamChunk(): replace updateWorkUnitsFromWatcher(chunk.workUnits) with loadData(), then sync session context from store data instead of chunk data
  After loadData() completes, session sync must read from useFspecStore.getState().workUnits (reloaded data) — NOT from chunk.workUnits (partial Rust data)
  updateWorkUnitsFromWatcher() can be left in fspecStore for backward compat but is no longer called — the watcher event is now purely a signal
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When globalStreamListener receives a WorkUnitsUpdate event, it must call loadData() (full re-read from disk) instead of updateWorkUnitsFromWatcher() (partial in-memory patch)
  #   2. After loadData() completes from a watcher event, work units must be ordered according to the states arrays in work-units.json (column priority order), not by flat array insertion order
  #   3. Full WorkUnit fields (stateHistory, attachments, updated, etc.) must be available after a watcher-triggered reload — not just the 7 fields from Rust WorkUnitInfo
  #   4. Work units deleted from work-units.json must be removed from the store after a watcher-triggered reload — no ghost entries
  #   5. If the currently-attached work unit (sessionStore.currentWorkUnitId) no longer exists in the store after a watcher-triggered reload, session context must be cleared (setCurrentWorkUnit(null, null))
  #   6. The session header status sync (updating currentWorkUnitStatus when status changes) must still work after switching to loadData() — it should read from the store's reloaded data, not from chunk.workUnits
  #   7. The Rust watcher event serves purely as a file-changed signal — the watcher data (chunk.workUnits) should not be used for store updates or session sync after the fix
  #
  # EXAMPLES:
  #   1. AI agent moves AUTH-001 from backlog to specifying via fspec tool → BoardView updates AUTH-001 to the specifying column in the correct priority position (from states array), not appended to end
  #   2. AI agent changes status of AUTH-001 externally → the ⏩ last-changed indicator moves to AUTH-001 on the board (because fresh stateHistory is loaded)
  #   3. User adds attachment to TOOL-014 via fspec CLI while board is open → attachment indicator appears on TOOL-014 in the details panel without TUI restart
  #   4. User deletes work unit AUTH-003 via fspec CLI while board is open → AUTH-003 disappears from the board (no ghost entry remains)
  #   5. User reprioritizes work units externally (reorders states arrays via fspec) → board reflects the new priority order within each column
  #   6. Session is attached to TOOL-014, TOOL-014 is deleted externally → session header clears work unit context (shows no work unit), board no longer shows TOOL-014
  #   7. Session is attached to AUTH-001, AI agent changes AUTH-001 status from backlog to implementing → session header updates to show 'implementing' status
  #   8. New work unit INFRA-001 created externally while board is open → INFRA-001 appears on the board in the correct column and position
  #
  # ========================================
  Background: User Story
    As a developer using the TUI board
    I want to see the board fully update when work-units.json changes externally
    So that I see accurate ordering, indicators, attachments, and deletions without restarting the TUI

  # ========================================
  # GAP 1 + GAP 5: Ordering from states arrays
  # ========================================
  @bug-fix
  @regression
  Scenario: Work unit status change preserves correct column priority order
    Given the TUI board is open with work units loaded
    And the backlog column shows "AUTH-001" at position 1 and "AUTH-002" at position 2
    When an external process moves "AUTH-001" from backlog to specifying
    And the file watcher triggers a WorkUnitsUpdate event
    Then the globalStreamListener should call loadData instead of updateWorkUnitsFromWatcher
    And "AUTH-001" should appear in the specifying column
    And "AUTH-001" should be in the position defined by the states.specifying array

  @bug-fix
  @regression
  Scenario: External priority reordering is reflected on the board
    Given the TUI board is open with work units loaded
    And the backlog column shows "AUTH-001" at position 1 and "AUTH-002" at position 2
    When an external process reorders the states.backlog array to ["AUTH-002", "AUTH-001"]
    And the file watcher triggers a WorkUnitsUpdate event
    Then the backlog column should show "AUTH-002" at position 1 and "AUTH-001" at position 2

  @bug-fix
  @regression
  Scenario: Last-changed indicator updates when status changes externally
  # ========================================
  # GAP 2: stateHistory for last-changed indicator
  # ========================================
    Given the TUI board is open with work units loaded
    And the last-changed indicator is showing on "AUTH-002"
    When an external process changes the status of "AUTH-001" to specifying
    And "AUTH-001" now has the most recent stateHistory timestamp
    And the file watcher triggers a WorkUnitsUpdate event
    Then the last-changed indicator should move to "AUTH-001"

  @bug-fix
  @regression
  Scenario: Attachment added externally appears in details panel
  # ========================================
  # GAP 3: Attachments visible after external change
  # ========================================
    Given the TUI board is open with work units loaded
    And work unit "TOOL-014" has no attachments
    When an external process adds an attachment to "TOOL-014"
    And the file watcher triggers a WorkUnitsUpdate event
    Then the details panel for "TOOL-014" should show the attachment

  @bug-fix
  @regression
  Scenario: Deleted work unit disappears from the board
  # ========================================
  # GAP 4: Deleted work units removed
  # ========================================
    Given the TUI board is open with work units loaded
    And "AUTH-003" is visible in the backlog column
    When an external process deletes "AUTH-003" from work-units.json
    And the file watcher triggers a WorkUnitsUpdate event
    Then "AUTH-003" should no longer appear on the board

  @bug-fix
  @regression
  @unit
  Scenario: globalStreamListener calls loadData on WorkUnitsUpdate event
  # ========================================
  # GAP 6: loadData called instead of updateWorkUnitsFromWatcher
  # ========================================
    Given the globalStreamListener is initialized
    When a WorkUnitsUpdate stream chunk is received
    Then loadData should be called on fspecStore
    And updateWorkUnitsFromWatcher should NOT be called

  @bug-fix
  @regression
  @unit
  Scenario: Watcher event chunk data is not used for store updates
  # ========================================
  # GAP 7: Watcher event used only as signal
  # ========================================
    Given the globalStreamListener is initialized
    When a WorkUnitsUpdate stream chunk is received with partial work unit data
    Then the store should be updated from the full file re-read via loadData
    And the chunk.workUnits data should not be passed to any store update function

  @bug-fix
  @regression
  Scenario: Session context cleared when attached work unit is deleted externally
  # ========================================
  # GAP 8: Session context cleared for deleted work unit
  # ========================================
    Given the TUI board is open with work units loaded
    And a session is attached to work unit "TOOL-014"
    And sessionStore.currentWorkUnitId is "TOOL-014"
    When an external process deletes "TOOL-014" from work-units.json
    And the file watcher triggers a WorkUnitsUpdate event
    Then "TOOL-014" should no longer appear on the board
    And sessionStore.currentWorkUnitId should be null
    And sessionStore.currentWorkUnitStatus should be null

  @bug-fix
  @regression
  Scenario: Session header status syncs from store data after watcher reload
  # ========================================
  # Session header status sync still works
  # ========================================
    Given the TUI board is open with work units loaded
    And a session is attached to work unit "AUTH-001"
    And sessionStore.currentWorkUnitStatus is "backlog"
    When an external process changes "AUTH-001" status to "implementing"
    And the file watcher triggers a WorkUnitsUpdate event
    Then sessionStore.currentWorkUnitStatus should be "implementing"
    And the status should be read from the store's reloaded data not from chunk.workUnits

  @bug-fix
  @regression
  Scenario: New work unit created externally appears on the board
  # ========================================
  # New work unit appears correctly
  # ========================================
    Given the TUI board is open with work units loaded
    And "INFRA-001" does not exist on the board
    When an external process creates "INFRA-001" with status "backlog"
    And the file watcher triggers a WorkUnitsUpdate event
    Then "INFRA-001" should appear in the backlog column
    And "INFRA-001" should be in the position defined by the states.backlog array
