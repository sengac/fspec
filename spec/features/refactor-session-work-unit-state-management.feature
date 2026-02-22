@done
@zustand
@session-management
@state-management
@tui
@TUI-068
Feature: Refactor session-work unit state management

  """
  Remove currentWorkUnitId, setCurrentWorkUnitId(), getCurrentWorkUnitId() from fspecStore
  Keep fspecStore.sessionAttachments for multi-session tracking and IPC
  Remove workUnitId prop from AgentViewProps, read from useSessionStore(s => s.currentWorkUnitId)
  BoardView calls sessionStore.setCurrentWorkUnit() BEFORE entering agent mode
  AgentView handleExitChoice ALWAYS calls sessionStore.setCurrentWorkUnit(null, null)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. sessionStore.currentWorkUnitId is the single source of truth for current work unit context
  #   2. fspecStore.sessionAttachments tracks work unit ↔ session mappings for multi-session/IPC support
  #   3. BoardView.selectedWorkUnit is for UI navigation highlight only, not session context
  #   4. Work unit context MUST be cleared (set to null) when exiting AgentView, for BOTH Detach and Close
  #   5. AgentView must read work unit context from sessionStore, not from props
  #   6. Pressing / without a selected work unit creates session without auto-attaching to any work unit
  #
  # EXAMPLES:
  #   1. User selects TOOL-014 on board, presses Enter → session attaches to TOOL-014, sessionStore.currentWorkUnitId = TOOL-014
  #   2. User closes session attached to TOOL-014, returns to board, presses / → new session created with NO work unit attachment
  #   3. User detaches from session on TOOL-014, returns to board, presses / → new session created with NO work unit attachment
  #   4. User on board with no work unit selected, presses / → new session created without any work unit context
  #   5. AI changes work unit to AUTH-001 via IPC → both fspecStore.sessionAttachments and sessionStore.currentWorkUnitId are updated
  #   6. fspecStore no longer has currentWorkUnitId or setCurrentWorkUnitId or getCurrentWorkUnitId (removed duplicates)
  #
  # ========================================

  Background: User Story
    As a developer using the TUI
    I want to have work unit context properly managed across session lifecycle
    So that new sessions don't incorrectly auto-attach to stale work units

  @happy-path
  Scenario: Session attaches to selected work unit when entering agent mode
    Given I am viewing the board with work units
    And I have selected work unit "TOOL-014"
    When I press Enter to start a session
    Then a new session should be created
    And the session should be attached to work unit "TOOL-014"
    And sessionStore.currentWorkUnitId should be "TOOL-014"

  @regression @bug-fix
  Scenario: New session does not auto-attach after closing previous session
    Given I am in an agent session attached to work unit "TOOL-014"
    When I close the session
    And I return to the board
    And I press "/" to start a new session
    Then a new session should be created
    And the session should NOT be attached to any work unit
    And sessionStore.currentWorkUnitId should be null

  @regression @bug-fix
  Scenario: New session does not auto-attach after detaching from previous session
    Given I am in an agent session attached to work unit "TOOL-014"
    When I detach from the session
    And I return to the board
    And I press "/" to start a new session
    Then a new session should be created
    And the session should NOT be attached to any work unit
    And sessionStore.currentWorkUnitId should be null

  @happy-path
  Scenario: Session created without work unit when no selection on board
    Given I am viewing the board with work units
    And no work unit is selected
    When I press "/" to start a new session
    Then a new session should be created
    And the session should NOT be attached to any work unit
    And sessionStore.currentWorkUnitId should be null

  @integration @ipc
  Scenario: Work unit context updates via IPC
    Given I am in an agent session attached to work unit "TOOL-014"
    When the AI changes work unit to "AUTH-001" via IPC
    Then sessionStore.currentWorkUnitId should be "AUTH-001"
    And fspecStore.sessionAttachments should map "AUTH-001" to the current session

  @code-quality @refactoring
  Scenario: Duplicate state removed from fspecStore
    Given I inspect the fspecStore implementation
    Then fspecStore should NOT have a currentWorkUnitId property
    And fspecStore should NOT have a setCurrentWorkUnitId method
    And fspecStore should NOT have a getCurrentWorkUnitId method
    And fspecStore should still have sessionAttachments for multi-session tracking

  # ========================================
  # SESSION SERVICE FACADE SCENARIOS
  # ========================================

  @unit @service
  Scenario: destroySession orchestrates all cleanup atomically
    Given I have an active session "session-123" attached to work unit "TOOL-014"
    When I call destroySession("session-123")
    Then sessionManagerDestroy should be called with "session-123"
    And fspecStore.sessionAttachments should NOT contain "TOOL-014"
    And sessionStore.currentWorkUnitId should be null
    And GlobalSessionStreamManager should unsubscribe from "session-123"

  @unit @service
  Scenario: attachToWorkUnit orchestrates all stores atomically
    Given I have an active session "session-123"
    When I call attachToWorkUnit("session-123", "TOOL-014")
    Then fspecStore.sessionAttachments should map "TOOL-014" to "session-123"
    And sessionStore.currentWorkUnitId should be "TOOL-014"
    And workUnitContextService should set context for "session-123" with work unit "TOOL-014"

  @unit @service
  Scenario: detachFromWorkUnit clears all state atomically
    Given I have an active session "session-123" attached to work unit "TOOL-014"
    When I call detachFromWorkUnit("session-123")
    Then fspecStore.sessionAttachments should NOT contain "TOOL-014"
    And sessionStore.currentWorkUnitId should be null
    And workUnitContextService should clear context for "session-123"

  @isolated @git-worktree
  Scenario: Isolated session close prompts user then calls merge or discard
    Given I have an isolated session "session-123" with changes in worktree
    When I choose to close the session
    Then the UI should prompt "Merge changes to main?" with options Merge and Discard
    When the user chooses "Merge"
    Then mergeSessionChanges should be called with "session-123"
    And destroySession should be called with "session-123"

  @isolated @git-worktree
  Scenario: Isolated session discard removes worktree without applying changes
    Given I have an isolated session "session-123" with changes in worktree
    When I choose to close the session
    And the user chooses "Discard"
    Then discardSessionChanges should be called with "session-123"
    And destroySession should be called with "session-123"
    And the worktree changes should NOT be applied to main

  # ========================================
  # COMPONENT INTEGRATION SCENARIOS
  # ========================================

  @integration @agentview
  Scenario: AgentView uses sessionService facade for all session-work unit lifecycle operations
    Given I inspect AgentView.tsx imports
    Then AgentView should import from sessionService
    And AgentView should NOT directly import sessionManagerDestroy from codelet-napi
    And AgentView should NOT use useFspecStore.attachSession directly
    And AgentView should NOT use useFspecStore.detachSession directly
    And AgentView should use attachToWorkUnit from sessionService for all attachment operations
    And AgentView should use detachFromWorkUnit from sessionService for all detachment operations
    And AgentView should NOT directly call useSessionStore for session lifecycle

  @integration @boardview
  Scenario: BoardView IPC handler uses sessionService for work unit attachment
    Given I receive an IPC message with type "work-unit-changed"
    And the payload contains workUnitId "AUTH-001" and sessionId "session-123"
    When BoardView processes the IPC message
    Then attachToWorkUnit should be called with "session-123" and "AUTH-001"
    And BoardView should NOT directly call fspecStore.attachSession

  @integration @globalstreamlistener
  Scenario: globalStreamListener uses sessionService for work unit context sync
    Given I receive a FspecCommandCompleted stream chunk
    And the chunk indicates work unit changed to "AUTH-001"
    When globalStreamListener processes the chunk
    Then sessionService should be used to sync work unit context
    And globalStreamListener should NOT directly call sessionStore.setCurrentWorkUnit
    And globalStreamListener should NOT directly call workUnitContextService
