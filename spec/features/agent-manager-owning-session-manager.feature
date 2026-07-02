@done
@session
@agent-manager
@rust
@RPC-386
Feature: AgentManager handler binds to global SessionManager singleton instead of the daemon-owned instance
  """
  Root cause: codelet/agent-loop/src/agent_manager_handler.rs calls SessionManager::instance() at lines 43 (create_handler) and 909 (create_async_handler). The fspec binary owns a separate manager built in codelet/fspec/src/common.rs::build_service (SessionManager::new() + FspecAgentHooks), so spawns land in the empty Noop-hooks singleton.
  Fix via dependency injection: give BackgroundSession a Weak<SessionManager> owning-manager back-reference, populated by create_session_with_id / create_isolated_session_with_id from a self-Weak the manager holds when wrapped in Arc. Thread the resolved Arc<SessionManager> into create_handler/create_async_handler instead of SessionManager::instance(); fall back to instance() when the back-reference is absent (NAPI path).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AgentManager spawn must create the subordinate in the SessionManager that owns the spawner session, never in SessionManager::instance()
  #   2. When the owning manager has real hooks (FspecAgentHooks), the spawned subordinate's agent loop must start so a later message is processed
  #   3. Spawning must fire the owning manager's session_created broadcast so the embedded TUI backend (session_created_rx) creates a subordinate tab
  #   4. All AgentManager actions (spawn, list, get_status, close, message, set_role) and the async actions (await_idle, profile) must resolve subordinates on the owning manager, not the singleton
  #   5. In the NAPI path (no owning-manager back-reference present), the handler must fall back to SessionManager::instance() so existing NAPI behaviour is unchanged
  #   6. The fix must not introduce any dependency from codelet-agent-loop or codelet-sessions on codelet-napi
  #
  # EXAMPLES:
  #   1. A foreground session created by a daemon-owned manager M spawns a subordinate; the subordinate appears in M.list_sessions() but NOT in SessionManager::instance().list_sessions()
  #   2. After spawning into manager M, a subscriber to M.session_created_tx() receives the subordinate's SessionInfo
  #   3. A subordinate spawned into a manager whose hooks run the agent loop processes a follow-up message and emits output chunks (proving it is alive, not a dead session object)
  #   4. From the spawner, list/get_status/close on the returned subordinate id all succeed against manager M and reflect the chain-of-command relationship
  #   5. With no owning-manager back-reference set (simulating the NAPI path), spawn still resolves and creates the subordinate on SessionManager::instance()
  #   6. await_idle invoked from the spawner blocks on the subordinate in manager M and returns its idle status (async handler also uses the owning manager)
  #
  # ========================================
  Background: User Story
    As a foreground agent running in the fspec Rust binary
    I want to spawn a subordinate via the AgentManager tool
    So that the subordinate is created in the same SessionManager the daemon/TUI owns, so it actually runs and appears as a tab

  Scenario: Spawn creates the subordinate in the owning manager, not the global singleton
    Given a daemon-owned SessionManager M that is not the global singleton
    And a spawner session created by M with an AgentManager handler bound to M
    When the spawner invokes the AgentManager spawn action
    Then the subordinate appears in M.list_sessions()
    And the subordinate does not appear in SessionManager::instance().list_sessions()

  Scenario: Spawn fires the owning manager's session_created broadcast
    Given a daemon-owned SessionManager M that is not the global singleton
    And a spawner session created by M with an AgentManager handler bound to M
    And a subscriber to M.session_created_tx()
    When the spawner invokes the AgentManager spawn action
    Then the subscriber receives the subordinate's SessionInfo

  Scenario: A subordinate spawned into a manager with real hooks runs its agent loop
    Given a daemon-owned SessionManager M whose hooks start the agent loop
    And a spawner session created by M with an AgentManager handler bound to M
    When the spawner spawns a subordinate and sends it a follow-up message
    Then the subordinate processes the message and emits output chunks

  Scenario: Spawner can list, get status, and close the subordinate on the owning manager
    Given a daemon-owned SessionManager M that is not the global singleton
    And a spawner session created by M with an AgentManager handler bound to M
    When the spawner spawns a subordinate via the AgentManager spawn action
    Then the AgentManager list action returns the subordinate id
    And the AgentManager get_status action resolves the subordinate on M
    And the chain-of-command on M records the spawner as the subordinate's supervisor
    And the AgentManager close action removes the subordinate from M

  Scenario: NAPI path falls back to the global singleton when no owning manager is set
    Given a spawner session with no owning-manager back-reference set
    When the spawner invokes the AgentManager spawn action
    Then the subordinate is created on SessionManager::instance()

  Scenario: set_role and message resolve the subordinate on the owning manager
    Given a daemon-owned SessionManager M whose hooks start the agent loop
    And a spawner session created by M with an AgentManager handler bound to M
    And the spawner has spawned a subordinate via the AgentManager spawn action
    When the spawner invokes the AgentManager set_role action for the subordinate
    And the spawner invokes the AgentManager message action for the subordinate
    Then the role is applied to the subordinate in M
    And the message is delivered to the subordinate in M

  Scenario: await_idle resolves the subordinate on the owning manager
    Given a daemon-owned SessionManager M whose hooks start the agent loop
    And a spawner session created by M with an AgentManager handler bound to M
    And the spawner has spawned a subordinate that becomes idle
    When the spawner invokes the AgentManager await_idle action for the subordinate
    Then await_idle blocks on the subordinate in M and returns its idle status
