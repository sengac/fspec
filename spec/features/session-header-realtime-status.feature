@status-display
@tui
@TUI-060
Feature: Session Header Work Unit Status Display

  """
  Create useWorkUnitsWatcher hook that uses chokidar to watch spec/work-units.json and calls loadData() on the Zustand store
  Refactor BoardView to use useWorkUnitsWatcher instead of inline chokidar setup
  Add useWorkUnitsWatcher to AgentView to enable realtime status updates in SessionHeader
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Extract file watching logic into a reusable hook (e.g., useWorkUnitsWatcher) following DRY/SOLID/COMPOSABLE principles
  #   2. The hook should live in its own file (src/tui/hooks/useWorkUnitsWatcher.ts) for separation of concerns
  #   3. Both BoardView and AgentView should use the same hook to watch work-units.json
  #   4. SessionHeader displays format: '#N (WORK-ID: status): model' when work unit is attached
  #   5. When work unit status is undefined/missing, show '#N (WORK-ID): model' without status
  #
  # EXAMPLES:
  #   1. User is in AgentView working on TUI-060 (specifying status). AI runs 'fspec update-work-unit-status TUI-060 testing'. Header updates from '#1 (TUI-060: specifying): claude-sonnet-4' to '#1 (TUI-060: testing): claude-sonnet-4'
  #   2. User attaches a different work unit to the session. Header updates from '#1 (TUI-060: specifying): model' to '#1 (AUTH-001: backlog): model'
  #   3. User detaches work unit from session. Header updates from '#1 (TUI-060: specifying): model' to '#1: model'
  #
  # ========================================

  Background: User Story
    As a developer using the TUI
    I want to see the attached work unit ID and status update in realtime in the session header
    So that I always know which work unit I'm working on and its current status without manually checking

  @integration
  Scenario: Status change via fspec command updates header in realtime
    Given I am in AgentView with session #1
    And work unit "TUI-060" with status "specifying" is attached to the session
    And the header displays "#1 (TUI-060: specifying): claude-sonnet-4"
    When the AI runs "fspec update-work-unit-status TUI-060 testing"
    And the work-units.json file is updated
    Then the header should update to "#1 (TUI-060: testing): claude-sonnet-4"

  @integration
  Scenario: Attaching a different work unit updates header in realtime
    Given I am in AgentView with session #1
    And work unit "TUI-060" with status "specifying" is attached to the session
    And the header displays "#1 (TUI-060: specifying): model"
    When I attach work unit "AUTH-001" with status "backlog" to the session
    Then the header should update to "#1 (AUTH-001: backlog): model"

  @integration
  Scenario: Detaching work unit removes it from header display
    Given I am in AgentView with session #1
    And work unit "TUI-060" with status "specifying" is attached to the session
    And the header displays "#1 (TUI-060: specifying): model"
    When I detach the work unit from the session
    Then the header should update to "#1: model"

  Scenario: Header displays work unit ID without status when status is missing
    Given I am in AgentView with session #1
    And work unit "LEGACY-001" without status is attached to the session
    Then the header should display "#1 (LEGACY-001): model"

  @unit
  Scenario: useWorkUnitsWatcher hook watches work-units.json
    Given the useWorkUnitsWatcher hook is initialized
    And the spec/work-units.json file exists
    When the work-units.json file changes
    Then the hook should call loadData on the Zustand store

  @unit
  Scenario: BoardView uses the shared useWorkUnitsWatcher hook
    Given BoardView is rendered
    Then it should use the useWorkUnitsWatcher hook
    And not have inline chokidar file watching code

  @unit
  Scenario: AgentView uses the shared useWorkUnitsWatcher hook
    Given AgentView is rendered
    Then it should use the useWorkUnitsWatcher hook
    And receive work unit updates from the Zustand store
