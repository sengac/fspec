@BUG-154
Feature: Missing set_owning_manager in create_session_with_id breaks AgentManager communication
  """
  The fix adds session.set_owning_manager(self.self_weak.get().cloned().unwrap_or_default()) call in SessionManager::create_session_with_id() after create_background_session_inner() returns, mirroring the existing call in create_session_from_manifest(). This ensures the AgentManager handler registered by register_agent_manager_handler() captures a non-None owning manager reference, enabling correct session lookup for spawn/list/close/message operations.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every session creation path (create_session_with_id, create_session_from_manifest, create_isolated_session_with_id) MUST call session.set_owning_manager() before spawning the agent loop
  #   2. The owning_manager reference must be set to self.self_weak.get().cloned().unwrap_or_default() so the AgentManager handler binds to the correct SessionManager instance
  #   3. The fix must be applied in the session_creation_helper shared helper OR in create_session_with_id after the helper returns, ensuring all three creation paths get the owning manager set
  #
  # EXAMPLES:
  #   1. When create_session_with_id is called, the returned session must have owning_manager set to the SessionManager's self_weak reference before spawn_agent_loop is called
  #   2. When a supervisor agent spawns a subordinate via AgentManager, the subordinate's AgentManager handler must be able to look up the supervisor's sessions through the owning manager reference
  #   3. When session_creation_helper returns a session, the caller (create_session_with_id) must call set_owning_manager before inserting the session into the sessions map
  #
  # ========================================
  Background: User Story
    As a Rust TUI session manager
    I want to ensure set_owning_manager is called in create_session_with_id
    So that AgentManager spawns and communicates correctly between agents

  Scenario: create_session_with_id sets owning_manager before spawning agent loop
    Given a SessionManager instance with no existing sessions
    When create_session_with_id is called with a valid UUID and model string
    Then the created session must have owning_manager set to the SessionManager's self_weak reference
    And the owning_manager must be set before spawn_agent_loop is called

  Scenario: AgentManager handler receives non-None owning_manager from create_session_with_id
    Given a session created via create_session_with_id
    When the agent loop registers the AgentManager handler via register_agent_manager_handler
    Then the handler must capture a non-None owning_manager reference
    And the handler must be able to look up sessions through the owning manager

  Scenario: create_session_with_id and create_session_from_manifest set owning_manager consistently
    Given a SessionManager instance
    When a session is created via create_session_with_id
    And another session is created via create_session_from_manifest
    Then both sessions must have owning_manager set to the same SessionManager instance
