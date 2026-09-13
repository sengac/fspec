@done
@BUG-180
@bug-180
@header
@agent-view
@tui
@rpc
@multi-session
@rust
Feature: BUG-180 SessionHeader work-unit status sync
  """
  BUG-180: when a work unit's status changes (via the board, a slash
  command, the LLM's fspec tool, or any external edit of
  spec/work-units.json), the SessionHeader chip in AgentView keeps
  painting the OLD status until the session is detached and
  re-attached.

  Root cause: the WorkUnitsWatcher push already reaches the TUI
  (work_units_rx -> Action::WorkUnitsLoaded on both the embedded and
  websocket transports), but the App::dispatch handler only re-seeds
  BoardStore and never syncs AgentViewStore.work_unit_context_by_session
  (or the legacy current_work_unit_* fallback slots), leaving the
  per-session WorkUnitContext frozen at attach time.

  Fix: complete the projection at the single mutation point
  (App::dispatch, RPC-009 tenere pattern) — the WorkUnitsLoaded handler
  calls AgentViewStore::sync_work_unit_contexts after the board
  re-seed. No new push channel or delta action is introduced.
  """

  Background: User Story
    As a TUI user with an agent session attached to a work unit
    I want the SessionHeader work-unit chip to repaint with the current status whenever the work unit's status changes from any writer
    So that the AgentView chrome never paints a stale status that disagrees with the board

  Scenario: WorkUnitsLoaded syncs per-session WorkUnitContext status
    Given an App wired to a MockBackend with open session s-1 bound to work unit AUTH-001 whose current status is "backlog"
    When Action::WorkUnitsLoaded carrying a snapshot where AUTH-001 now has status "implementing" is dispatched
    Then AgentViewStore.work_unit_context_for(s-1) returns Some with status "implementing"
    And BoardStore still groups AUTH-001 in the "implementing" column

  Scenario: WorkUnitsLoaded syncs legacy fallback slots for the focused session
    Given an App with legacy fallback slots current_work_unit_id="AUTH-001" current_work_unit_status="backlog"
    When Action::WorkUnitsLoaded carrying AUTH-001 status "testing" is dispatched
    Then AgentViewStore.current_work_unit_status equals Some("testing")

  Scenario: WorkUnitsLoaded does not destroy a binding whose unit was deleted
    Given an App with open session s-1 bound to work unit AUTH-001 with status "backlog"
    When Action::WorkUnitsLoaded carrying a snapshot that no longer contains AUTH-001 is dispatched
    Then AgentViewStore.work_unit_context_for(s-1) still returns Some with status "backlog"

  Scenario: WorkUnitsLoaded leaves detached sessions untouched
    Given an App with open session s-1 bound to AUTH-001 and open session s-2 with NO work unit bound
    When Action::WorkUnitsLoaded carrying AUTH-001 status "validating" is dispatched
    Then AgentViewStore.work_unit_context_for(s-1) returns Some with status "validating"
    And AgentViewStore.work_unit_context_for(s-2) returns None

  Scenario: SessionHeader repaints with the fresh status after WorkUnitsLoaded
    Given an App with open session s-1 bound to AUTH-001 with status "backlog" rendered in AgentView
    When Action::WorkUnitsLoaded carrying AUTH-001 status "implementing" is dispatched
    And the AgentView is rendered against an 80x10 TestBackend
    Then the rendered top row contains the substring "(AUTH-001: implementing)"
    And the rendered top row does NOT contain the substring "(AUTH-001: backlog)"

  Scenario: WorkUnitsLoaded syncs the title along with the status
    Given an App with open session s-1 bound to AUTH-001 (title "AUTH-001", status "backlog")
    When Action::WorkUnitsLoaded carrying AUTH-001 with title "User Login" and status "specifying" is dispatched
    Then AgentViewStore.work_unit_context_for(s-1) returns Some with title "User Login" and status "specifying"

