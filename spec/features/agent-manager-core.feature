@AMGR-009
Feature: Core AgentManager tool — spawn, list, get_status, close

  """
  Tool module at codelet/tools/src/agent_manager/ with mod.rs (AgentManagerTool struct + Tool impl), handler.rs (static HashMap + set/execute), types.rs (serde-tagged AgentManagerAction enum + result types)
  Handler closure created in codelet-napi/src/agent_manager_handler.rs with access to SessionManager for session creation, destruction, ChainOfCommand, and status queries. Registered/deregistered in agent_loop() like session_search_handler.
  spawn handler calls SessionManager::create_session_with_id() to create the subordinate, then add_supervisor() on ChainOfCommand. The spawned session inherits the spawner's provider_id and model_id. The handler needs Arc<SessionManager> reference.
  close handler calls SessionManager::destroy_session() which already handles ChainOfCommand cleanup (cleanup_subordinate + remove_supervisor). Permission check: handler reads ChainOfCommand to verify the calling session is the spawner.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AgentManagerTool follows the handler-delegated pattern: tool struct in codelet-tools with static HashMap<Uuid, Handler>, handler closure created in codelet-napi with persistence access
  #   2. spawn action creates a new subordinate BackgroundSession running regular agent_loop, inherits the spawner's model, registers relationship in ChainOfCommand via add_supervisor()
  #   3. spawn accepts an optional role string (system prompt overlay). The subordinate starts idle and waits for a message — no initial prompt is sent
  #   4. list action returns all sessions with their session_id, name, role, status, spawner_id, and subordinate_ids — no access control
  #   5. get_status action takes a session_id and returns detailed info: session_id, role, status, model, spawner_id, subordinate_ids, pending_messages. Returns error with code session_not_found if ID is invalid.
  #   6. close action takes a session_id and terminates the subordinate. Only the spawner (supervisor) can close a session — returns error with code permission_denied otherwise. Cleans up ChainOfCommand relationships.
  #   7. All actions return JSON. Error responses use shape: { error: true, code: string, message: string }. Error codes: session_not_found, permission_denied, invalid_parameter
  #   8. The tool is registered in all 5 providers' create_rig_agent() with .tool(AgentManagerTool::new(session_id)) alongside SessionSearch and DeepSearch
  #   9. Handler is registered in agent_loop() before run via set_agent_manager_handler(session_id, Some(handler)) and cleaned up after via set_agent_manager_handler(session_id, None)
  #   10. Actions use serde tagged dispatch: #[serde(tag = "action", rename_all = "snake_case")] on AgentManagerAction enum
  #
  # EXAMPLES:
  #   1. Agent calls AgentManager(action='spawn') with no role — gets back { session_id: 'uuid' }. Subordinate inherits the spawner's model and starts idle.
  #   2. Agent calls AgentManager(action='spawn', role='You are a security reviewer') — gets back { session_id: 'uuid' }. Subordinate has the role set as system prompt overlay.
  #   3. Agent calls AgentManager(action='list') — returns { sessions: [...] } with all active sessions showing their session_id, name, role, status, spawner_id, subordinate_ids.
  #   4. Agent calls AgentManager(action='get_status', session_id='valid-uuid') — returns full status object. Agent calls with session_id='nonexistent' — returns { error: true, code: 'session_not_found', message: '...' }.
  #   5. Spawner agent calls AgentManager(action='close', session_id='subordinate-uuid') — returns { closed: true, session_id: '...' }. ChainOfCommand is cleaned up, session is destroyed.
  #   6. Non-spawner agent calls AgentManager(action='close', session_id='other-session') — returns { error: true, code: 'permission_denied', message: '...' }. Only the spawner can close.
  #   7. Agent calls AgentManager(action='spawn') 3 times — creates 3 subordinate workers. Agent calls list — sees all 3 with spawner_id pointing back to the agent's session.
  #   8. Agent calls AgentManager(action='invalid_action') — returns { error: true, code: 'invalid_parameter', message: 'Unknown action: invalid_action' }.
  #
  # ========================================

  Background: User Story
    As a AI agent running in codelet
    I want to spawn subordinate sessions, list all sessions, get session status, and close subordinates via a single AgentManager tool
    So that I can create and manage worker agents for parallel tasks without manual session management

  @spawn
  Scenario: Spawn subordinate session without role
    Given I am an agent with a registered AgentManager handler
    When I call AgentManager with action "spawn" and no role
    Then I should receive a JSON response with a valid session_id
    And the subordinate session should exist with idle status
    And the subordinate should inherit the spawner's model
    And the ChainOfCommand should record the spawner-subordinate relationship

  @spawn
  Scenario: Spawn subordinate session with role
    Given I am an agent with a registered AgentManager handler
    When I call AgentManager with action "spawn" and role "You are a security reviewer"
    Then I should receive a JSON response with a valid session_id
    And the subordinate session should have role "You are a security reviewer"

  @list
  Scenario: List all sessions with relationships
    Given I am an agent with a registered AgentManager handler
    And I have spawned 2 subordinate sessions
    When I call AgentManager with action "list"
    Then I should receive a JSON response with a sessions array
    And each session entry should include session_id, name, role, status, spawner_id, and subordinate_ids
    And my session should show 2 subordinate_ids
    And each subordinate should show my session as spawner_id

  @get_status
  Scenario: Get status of an existing session
    Given I am an agent with a registered AgentManager handler
    And I have spawned a subordinate session
    When I call AgentManager with action "get_status" and the subordinate's session_id
    Then I should receive a JSON response with session_id, role, status, model, spawner_id, subordinate_ids, and pending_messages

  @get_status @error
  Scenario: Get status of a nonexistent session
    Given I am an agent with a registered AgentManager handler
    When I call AgentManager with action "get_status" and session_id "nonexistent-uuid"
    Then I should receive an error response with code "session_not_found"

  @close
  Scenario: Close subordinate session as spawner
    Given I am an agent with a registered AgentManager handler
    And I have spawned a subordinate session
    When I call AgentManager with action "close" and the subordinate's session_id
    Then I should receive a JSON response with closed true and the session_id
    And the subordinate session should no longer exist
    And the ChainOfCommand should have no record of the closed subordinate

  @close @error
  Scenario: Close session without spawner permission
    Given I am an agent with a registered AgentManager handler
    And another agent has spawned a subordinate session
    When I call AgentManager with action "close" and that subordinate's session_id
    Then I should receive an error response with code "permission_denied"
    And the subordinate session should still exist

  @spawn @list
  Scenario: Spawn multiple subordinates and list them
    Given I am an agent with a registered AgentManager handler
    When I call AgentManager with action "spawn" 3 times
    Then each spawn should return a unique session_id
    When I call AgentManager with action "list"
    Then the sessions array should include all 3 subordinates
    And each subordinate should show my session as spawner_id

  @error
  Scenario: Call with invalid action
    Given I am an agent with a registered AgentManager handler
    When I call AgentManager with action "invalid_action"
    Then I should receive an error response with code "invalid_parameter"
    And the error message should mention "invalid_action"

  @handler
  Scenario: Handler lifecycle in agent loop
    Given a session is starting its agent loop
    When the agent loop registers the AgentManager handler
    Then the handler should be available for the session
    When the agent loop completes and deregisters the handler
    Then the handler should no longer be available for the session

  @integration
  Scenario: Tool is registered in all providers
    Given the AgentManager tool is implemented
    When a session is created with any of the 5 providers
    Then the AgentManagerTool should be included in the agent's tool set
    And the tool should accept the session_id parameter
