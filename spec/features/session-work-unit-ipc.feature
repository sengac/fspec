@done
@status-display
@session-attachment
@tui
@TUI-060
Feature: Session Work Unit Attachment via IPC
  """
  When AI runs update-work-unit-status via the Fspec tool (TUI context) on a DIFFERENT
  work unit, the session should automatically attach to the NEW work unit via IPC.

  This feature handles:
  - IPC message sending from Fspec tool subprocess
  - Session attachment updates in fspecStore
  - Board badge movement between work unit cards

  Note: Header display updates are handled by session-header-realtime-status.feature
  using Zustand sessionStore subscriptions.
  """

  Background: User Story
    As a developer using the TUI
    I want the session to automatically attach to the work unit I'm updating
    So that the board and header always reflect my current focus

  # ----------------------------------------
  # Core IPC Session Attachment Scenarios
  # ----------------------------------------
  @integration
  @critical
  Scenario: Fspec tool updates status on DIFFERENT work unit and session attaches to it
    Given I am in AgentView with session #1
    And work unit "TUI-060" with status "specifying" is attached to session #1
    And work unit "AUTH-001" exists with status "backlog"
    When the AI runs "fspec update-work-unit-status AUTH-001 testing" via Fspec tool
    Then an IPC message "work-unit-changed" should be sent with workUnitId "AUTH-001"
    And the TUI receives the IPC message and calls attachSession for AUTH-001
    And the board should show session badge on AUTH-001 card instead of TUI-060 card

  @integration
  Scenario: Fspec tool updates status on SAME work unit without changing attachment
    Given I am in AgentView with session #1
    And work unit "TUI-060" with status "specifying" is attached to session #1
    When the AI runs "fspec update-work-unit-status TUI-060 testing" via Fspec tool
    Then no IPC message "work-unit-changed" should be sent
    And the session should remain attached to TUI-060

  @integration
  Scenario: CLI command does NOT trigger session attachment change
    Given I am in AgentView with session #1
    And work unit "TUI-060" with status "specifying" is attached to session #1
    When the user runs "fspec update-work-unit-status AUTH-001 testing" from CLI directly
    Then no IPC message "work-unit-changed" should be sent
    And the session should remain attached to TUI-060

  # ----------------------------------------
  # IPC Implementation Scenarios
  # ----------------------------------------
  @unit
  Scenario: workUnitStatusHook sends IPC message on context change
    Given the workUnitStatusHook is called with workUnitId "AUTH-001"
    And the active session has workUnitId "TUI-060" attached
    When the hook detects a work unit context change
    Then it should call sendIPCMessage with type "work-unit-changed"
    And the payload should include workUnitId "AUTH-001" and the sessionId

  @unit
  Scenario: TUI IPC listener handles work-unit-changed message
    Given the TUI has an IPC server listening
    And session "#1" is attached to work unit "TUI-060"
    When an IPC message with type "work-unit-changed" arrives
    And the payload contains workUnitId "AUTH-001" and sessionId "#1"
    Then the fspecStore attachSession should be called with AUTH-001 and session #1
    And the old attachment for TUI-060 should be removed
