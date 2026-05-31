@done
@session-management
@rust
@multi-session
@rpc
@agent-view
@tui
@RPC-050
Feature: Work-unit attach binding (BoardView attach + SessionHeader chip)
  """
  Phase 6.6 of the RPC-030 roadmap. Wires the BoardView attach path
  end-to-end: a new Action::AttachWorkUnitToSession(work_unit_id)
  routes through App::dispatch into backend.set_work_unit_context(
  session_id, Some(ctx)) and folds the result into a per-session map
  on AgentViewStore. The SessionHeader chip then renders id+status from
  that per-session map, falling back to the legacy current_work_unit_id
  slots when no per-session binding exists (so RPC-029 chrome-parity
  tests keep passing).

  Companion to spec/features/slash-command-detach-and-work-unit-binding.feature
  which holds the /detach teardown side of the same binding.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. BoardView Enter on a work unit dispatches Action::AttachWorkUnitToSession(work_unit_id) which calls backend.set_work_unit_context(session_id, Some(ctx)) and navigates to AgentView
  #   2. AttachWorkUnitToSession with NO current session is a silent no-op
  #   3. On Ok(()) from backend.set_work_unit_context(Some), the dispatcher emits Action::WorkUnitAttached(SessionId, WorkUnitContext) which folds the context into AgentViewStore.work_unit_context_by_session[session_id]
  #   4. SessionHeader renders the work-unit chip (id + status) from AgentViewStore.work_unit_context_for(session_id) when Some, falling back to the legacy slots when None
  #
  # ========================================
  Background: User Story
    As a fspec user navigating the BoardView
    I want pressing Enter on a work unit to attach it to my active AgentView session
    So that the SessionHeader chip and the backend's per-session work-unit binding both reflect the link

  Scenario: AttachWorkUnitToSession with a current session calls the backend and folds the context into AgentViewStore
    Given an App wired to a MockBackend with open session s-1 as the current session
    And the BoardStore contains work unit AUTH-001 in the "implementing" column
    When Action::AttachWorkUnitToSession("AUTH-001") is dispatched
    Then within 1 second backend.set_work_unit_context is called exactly once with (s-1, Some(WorkUnitContext{id:"AUTH-001", title:"AUTH-001", status:"implementing"}))
    And within 1 second AgentViewStore.work_unit_context_for(s-1) returns Some(ctx) with id "AUTH-001" and status "implementing"
    And the Navigator's active_view equals ViewMode::Agent

  Scenario: AttachWorkUnitToSession with NO current session is a silent no-op
    Given an App wired to a MockBackend with NO open session
    And the BoardStore contains work unit AUTH-001
    When Action::AttachWorkUnitToSession("AUTH-001") is dispatched
    Then backend.set_work_unit_context is NEVER called

  Scenario: SessionHeader renders the work-unit chip from per-session context
    Given an App with open session s-1 bound to work unit AUTH-001 with status "implementing"
    When the AgentView is rendered against an 80x10 TestBackend
    Then the rendered top row contains the substring "(AUTH-001: implementing)"
