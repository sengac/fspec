@done
@status-display
@tui
@TUI-060
Feature: Session Header Work Unit Status Display
  """
  Display work unit ID and status in session header with realtime updates.
  Format: '#N (WORK-ID: status): model'.

  Source of Truth: Rust session state for session info (model, work unit ID, status)
  Data Flow: Rust session state -> AgentView sync -> sessionStore -> SessionHeader

  Key Constraints:
  - ONE singleton Rust file watcher (notify crate) for work-units.json at BoardView level
  - AgentView does NOT create its own watcher
  - SessionHeader uses Zustand hooks, not props, for work unit info
  """

  # ========================================
  # BUSINESS RULES (from Example Mapping):
  # ========================================
  #
  #   1. Rust state is the source of truth for session information
  #   2. SessionHeader MUST use Zustand store directly - no props for dynamic state
  #   3. ONE singleton file watcher at BoardView level only
  #   4. File watcher single responsibility: update workUnits array via loadData()
  #   5. Rust session state syncs to sessionStore (workUnitId, workUnitStatus)
  #   6. SessionHeader subscribes to sessionStore for currentWorkUnitId/currentWorkUnitStatus
  #   7. Header format: '#N (WORK-ID: status): model' - data from Zustand subscriptions
  #
  # ========================================
  Background: User Story
    As a developer using the TUI
    I want to see the attached work unit ID and status update in realtime in the session header
    So that I always know which work unit I'm working on and its current status

  # ----------------------------------------
  # Zustand Store Architecture
  # ----------------------------------------
  @unit
  Scenario: SessionHeader subscribes to sessionStore for work unit info
    Given SessionHeader component is rendered
    Then it should use useCurrentWorkUnitId hook to get work unit ID
    And it should use useCurrentWorkUnitStatus hook to get work unit status
    And it should NOT receive workUnitId or workUnitStatus as props

  @unit
  Scenario: sessionStore provides currentWorkUnitId and currentWorkUnitStatus
    Given sessionStore is initialized
    Then it should have a currentWorkUnitId field
    And it should have a currentWorkUnitStatus field
    And it should have a setCurrentWorkUnit action

  @unit
  Scenario: AgentView syncs Rust snapshot to sessionStore
    Given AgentView is processing a Rust state update
    When the rustSnapshot contains workUnitId and workUnitStatus
    Then AgentView should call sessionStore setCurrentWorkUnit with those values
    And SessionHeader should re-render with the new values

  # ----------------------------------------
  # Singleton File Watcher
  # ----------------------------------------
  @unit
  Scenario: BoardView has singleton file watcher for work-units.json
    Given BoardView is rendered
    Then it should start the Rust file watcher for spec/work-units.json
    And the watcher should call fspecStore loadData on file changes

  @unit
  Scenario: AgentView does NOT create its own file watcher
    Given AgentView is rendered as a child of BoardView
    Then AgentView should NOT call useWorkUnitsWatcher
    And AgentView should NOT create any file watchers
    And there should be exactly ONE watcher for work-units.json total

  # ----------------------------------------
  # Integration Scenarios
  # ----------------------------------------
  @integration
  Scenario: Status change via fspec command updates header in realtime
    Given I am in AgentView with session #1
    And work unit "TUI-060" with status "specifying" is attached
    And the header displays "#1 (TUI-060: specifying): claude-sonnet-4"
    When the AI runs "fspec update-work-unit-status TUI-060 testing"
    And Rust detects the status change and updates sessionStore
    Then the header should update to "#1 (TUI-060: testing): claude-sonnet-4"

  @integration
  Scenario: Opening AgentView shows work unit info from sessionStore
    Given I am on the BoardView
    And work unit "TUI-060" has status "implementing"
    When I open AgentView for work unit "TUI-060"
    And Rust initializes session with workUnitId TUI-060 and workUnitStatus implementing
    Then AgentView should sync this to sessionStore
    And the header should display "#1 (TUI-060: implementing): claude-sonnet-4"

  @integration
  Scenario: Header displays work unit ID without status when status is missing
    Given I am in AgentView with session #1
    And Rust provides workUnitId LEGACY-001 but workUnitStatus is undefined
    When sessionStore is updated with these values
    Then the header should display "#1 (LEGACY-001): model"
