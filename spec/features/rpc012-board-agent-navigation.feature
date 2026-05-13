@done
@RPC-012
@rust
@tui
@infrastructure
@rpc
@critical
Feature: RPC-012 Board ↔ Agent navigation — App-level handoff via stores and Navigator
  """
  RPC-012 — App-level integration contract for the new Board ↔ Agent
  navigator. App owns BoardStore + AgentViewStore + Navigator and applies
  every state mutation synchronously inside App::dispatch (RPC-009
  single-task tenere pattern).

  TS reference: src/tui/components/UnifiedBoardLayout.tsx (Enter +
  Shift+Right handlers) and src/tui/components/AgentView.tsx (ESC + lazy
  session creation + chunks filtering).
  """

  Background: User Story
    As a Rust fspec frontend developer driving App with MockBackend
    I want App::dispatch + App::bootstrap_navigator to coordinate the BoardStore, AgentViewStore, and Navigator stores
    So that Enter / Shift+Right / ESC / SessionCreated flow through the system without any view directly mutating shared state

  Scenario: AgentViewStore is empty after App::bootstrap with no pre-created session
    Given an App constructed against a MockBackend with no scripted session
    When the developer drives App::bootstrap to completion
    Then MockBackend.list_work_units_calls equals 1
    And MockBackend.create_session_calls equals 0
    And AgentViewStore.current_session returns None
    And AgentViewStore.show_create_session_dialog returns false
    And AgentViewStore.current_work_unit_id returns None

  Scenario: Enter on a work unit hands off to AgentView and triggers lazy session creation
    Given an App with bootstrap complete and BoardStore seeded with [AUTH-002 implementing]
    And BoardStore.focused_column() returns "implementing" with selection 0
    And AgentViewStore.current_session is None
    When the App dispatches Action::EnterWorkUnit("AUTH-002")
    Then AgentViewStore.current_work_unit_id equals Some("AUTH-002")
    And AgentViewStore.current_work_unit_status equals Some("implementing")
    And the Navigator's active_view equals ViewMode::Agent
    And the App spawns a pending tokio task that resolves to MockBackend.create_session_calls() == 1

  Scenario: Shift+Right on an unattached work unit raises the create-session dialog flag
    Given an App with bootstrap complete and BoardStore seeded with [AUTH-001 backlog]
    And BoardStore has no session_attachments entry for "AUTH-001"
    When the App dispatches Action::OpenAgentView(None)
    Then AgentViewStore.show_create_session_dialog returns true
    And AgentViewStore.should_auto_create_session returns true
    And the Navigator's active_view equals ViewMode::Agent

  Scenario: Shift+Right on an attached work unit sets the navigation target
    Given an App with bootstrap complete and BoardStore seeded with [AUTH-001 backlog]
    And BoardStore.attach_session("AUTH-001", SessionId::new("s-1")) has been called
    When the App dispatches Action::OpenAgentView(Some(SessionId::new("s-1")))
    Then AgentViewStore.navigation_target_session equals Some(SessionId::new("s-1"))
    And the Navigator's active_view equals ViewMode::Agent

  Scenario: ESC from AgentView returns to BoardView preserving focus and selection
    Given an App with Navigator.active_view = ViewMode::Agent
    And BoardStore.focused_column() returns "implementing"
    And BoardStore.selected_index_for("implementing") returns 0
    When the App dispatches Action::BackToBoard
    Then the Navigator's active_view equals ViewMode::Board
    And BoardStore.focused_column() still returns "implementing"
    And BoardStore.selected_index_for("implementing") still returns 0

  Scenario: Action::SessionCreated with a current work unit emits Action::AttachSession
    Given an App with AgentViewStore.current_work_unit_id = Some("AUTH-002")
    When the App dispatches Action::SessionCreated(SessionId::new("s-1"))
    Then the App emits Action::AttachSession("AUTH-002", SessionId::new("s-1")) onto the action bus
    And after that action is processed BoardStore.session_for("AUTH-002") equals Some(&SessionId::new("s-1"))

  Scenario: Chunks subscriber filter follows AgentViewStore.current_session via watch channel
    Given an App with bootstrap complete and AgentViewStore.current_session = Some(SessionId::new("s-1"))
    When the App dispatches Action::SessionCreated(SessionId::new("s-2"))
    Then AgentViewStore.current_session equals Some(SessionId::new("s-2"))
    And the App publishes Some(SessionId::new("s-2")) onto the chunks watch channel
    And a subsequent chunk for SessionId::new("s-1") is dropped by the chunks subscriber
    And a subsequent chunk for SessionId::new("s-2") becomes Action::ChunkReceived on the action bus

  Scenario: Navigator renders BoardView as the first landing view
    Given an App with bootstrap complete and Navigator.active_view defaulting to ViewMode::Board
    When the App renders against an 80x24 TestBackend
    Then the rendered buffer contains the seven column headers BACKLOG SPECIFYING TESTING IMPLEMENTING VALIDATING DONE BLOCKED
    And the rendered buffer does NOT contain the AgentView "Agent" block title

  Scenario: Navigator renders AgentView when active_view is Agent
    Given an App with Navigator.active_view = ViewMode::Agent
    And AgentViewStore.current_session = Some(SessionId::new("s-1"))
    When the App renders against an 80x24 TestBackend
    Then the rendered buffer contains the AgentView "Agent" block title
    And the rendered buffer does NOT contain the BACKLOG SPECIFYING column headers
