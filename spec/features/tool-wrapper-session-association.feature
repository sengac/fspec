@BRIDGE-021
@providers
@session-management
@tool-execution
@session
@tools
@TOOL-012
Feature: Tool Wrappers Should Store Session ID Instead of Using Thread-Local Current Session
  """
  Flow change: OLD: create_rig_agent() → tools call get_current_session() at runtime. NEW: create_rig_agent(session_id) → tools store session_id at construction → tools use self.session_id at call time.
  Files to modify: (1) tools/src/facade/wrapper.rs - add session_id field to FspecToolFacadeWrapper and BridgeToolFacadeWrapper, (2) tools/src/facade/fspec_registration.rs - add session_id param to all *_fspec_tool() functions, (3) tools/src/facade/bridge_registration.rs - add session_id param to all *_bridge_tool() functions, (4) providers/src/*.rs - all create_rig_agent() signatures, (5) napi/src/session_manager.rs - run_with_provider! macro, (6) cli/src/lib.rs - standalone CLI calls
  Breaking API change: create_rig_agent() gains mandatory session_id first parameter. All callers must be updated. No default/optional session - explicit is better than implicit.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. FspecToolFacadeWrapper must have a session_id: Uuid field set at construction time
  #   2. BridgeToolFacadeWrapper must have a session_id: Uuid field set at construction time
  #   3. Tool call() must use self.session_id to look up handler, not thread-local get_current_*_session()
  #   4. Registration functions (claude_fspec_tool, etc.) must accept session_id as parameter
  #   5. Thread-local CURRENT_*_SESSION storage must be removed (deprecated)
  #   6. create_rig_agent() signature must change to accept session_id as FIRST parameter since it's required for tool construction. All providers (Claude, Gemini, OpenAI, ZAI) must be updated consistently.
  #   7. session.id is available in run_with_provider! macro context. Macro must pass session.id to create_rig_agent(). The macro expansion changes from 'provider.create_rig_agent(None, $thinking)' to 'provider.create_rig_agent(session.id, None, $thinking)'
  #   8. Yes, CURRENT_BRIDGE_SESSION uses global RwLock (line 62 bridge_handler.rs), which is WORSE than thread_local - it can be overwritten by any concurrent session. Both Fspec and Bridge tools have the same architectural problem: session lookup at call time. The fix (session_id at construction) applies identically to both.
  #   9. Rust CLI single-shot mode (lib.rs) must generate new session_id with Uuid::new_v4() before calling create_rig_agent(session_id, None, None). Tests and examples can use Uuid::nil() when testing tools that don't need handler routing. The handler must still be registered for the session_id if Fspec tool is used.
  #   10. set_fspec_handler_for_session(session_id, handler) REMAINS - still needed to register handlers per session. set_current_fspec_session() REMOVED - no longer needed since tools carry session_id. Same for bridge: set_bridge_session_context() REMAINS, set_current_bridge_session() REMOVED.
  #
  # EXAMPLES:
  #   1. Session manager creates claude_fspec_tool(session_id) → wrapper stores session_id → call() uses self.session_id → correct handler found
  #   2. Tool call crosses async boundary → self.session_id still valid → handler lookup succeeds (thread-local would fail)
  #   3. Two sessions A and B each create their own tool instances → A's tool uses A's handler, B's tool uses B's handler → no cross-contamination
  #   4. Claude Code CLI scenario: Agent built once at startup → no session context at creation time → Current architecture fails. New architecture: CLI must create session_id UUID before building agent, pass to create_rig_agent().
  #   5. Watcher session A monitors parent session B: A has its own Fspec tool (session_id=A), B has its own Fspec tool (session_id=B). When A's tool is called, handler for A is used. When B's tool is called, handler for B is used. No confusion possible.
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should create_rig_agent() signature change to accept session_id, or should provider build methods receive session_id?
  #   A: create_rig_agent() signature must change to accept session_id as FIRST parameter since it's required for tool construction. All providers (Claude, Gemini, OpenAI, ZAI) must be updated consistently.
  #
  #   Q: Where does session_id come from when create_rig_agent() is called in run_with_provider! macro?
  #   A: session.id is available in run_with_provider! macro context. Macro must pass session.id to create_rig_agent(). The macro expansion changes from 'provider.create_rig_agent(None, $thinking)' to 'provider.create_rig_agent(session.id, None, $thinking)'
  #
  #   Q: Bridge uses global RwLock not thread_local - is the problem there too?
  #   A: Yes, CURRENT_BRIDGE_SESSION uses global RwLock (line 62 bridge_handler.rs), which is WORSE than thread_local - it can be overwritten by any concurrent session. Both Fspec and Bridge tools have the same architectural problem: session lookup at call time. The fix (session_id at construction) applies identically to both.
  #
  #   Q: What about callers outside NAPI (Rust CLI, tests, examples) that call create_rig_agent()?
  #   A: Rust CLI single-shot mode (lib.rs) must generate new session_id with Uuid::new_v4() before calling create_rig_agent(session_id, None, None). Tests and examples can use Uuid::nil() when testing tools that don't need handler routing. The handler must still be registered for the session_id if Fspec tool is used.
  #
  #   Q: What happens to the handler registration functions set_fspec_handler_for_session and set_current_fspec_session after this change?
  #   A: set_fspec_handler_for_session(session_id, handler) REMAINS - still needed to register handlers per session. set_current_fspec_session() REMOVED - no longer needed since tools carry session_id. Same for bridge: set_bridge_session_context() REMAINS, set_current_bridge_session() REMOVED.
  #
  # ========================================
  Background: User Story
    As a session manager
    I want to create tool instances with explicit session association
    So that tools always know which session's handler to use without relying on thread-local state

  # Example 1: Happy path - session_id stored at construction
  @unit
  Scenario: Fspec tool wrapper stores session_id at construction and uses it at call time
    Given a session manager has created a session with ID "session-A"
    And a handler has been registered for session "session-A"
    When the session manager creates an Fspec tool with claude_fspec_tool(session_id)
    Then the tool wrapper should store session_id as a field
    When the LLM invokes the Fspec tool with command "board"
    Then the tool should use self.session_id to look up the handler
    And the correct handler for "session-A" should be invoked
    And the command should execute successfully

  @unit
  Scenario: Fspec tool call succeeds across async boundaries
  # Example 2: Async boundary - session_id survives where thread-local would fail
    Given a session manager has created a session with ID "session-B"
    And a handler has been registered for session "session-B"
    And an Fspec tool has been created with session_id "session-B"
    When the tool call crosses an async boundary via tokio task spawn
    Then self.session_id should still be valid
    And the handler lookup should succeed
    And the command should execute on the correct session

  @unit
  Scenario: Concurrent sessions use their own isolated tool instances
  # Example 3: Multi-session isolation - no cross-contamination
    Given session "session-A" exists with its own registered handler
    And session "session-B" exists with its own registered handler
    When session A creates its Fspec tool with claude_fspec_tool(session_id_A)
    And session B creates its Fspec tool with claude_fspec_tool(session_id_B)
    Then A's tool should have session_id field set to session_id_A
    And B's tool should have session_id field set to session_id_B
    When A's tool is invoked
    Then handler for session A should be called
    When B's tool is invoked
    Then handler for session B should be called
    And there should be no cross-contamination between sessions

  @integration
  Scenario: Rust CLI creates session UUID before building agent
  # Example 4: Standalone CLI creates session_id before building agent
    Given the Rust CLI is running in single-shot mode
    When the CLI prepares to build a rig agent
    Then the CLI should generate a new session_id with Uuid::new_v4()
    And the CLI should call create_rig_agent(session_id, None, None)
    And the Fspec tool in the agent should have the generated session_id

  @integration
  Scenario: Watcher session and parent session use separate Fspec tool instances
  # Example 5: Watcher session and parent session have isolated handlers
    Given parent session "parent-P" exists with its Fspec tool
    And watcher session "watcher-W" is monitoring "parent-P"
    And watcher session "watcher-W" has its own Fspec tool
    When the parent session's Fspec tool is invoked
    Then the handler for "parent-P" should be used
    When the watcher session's Fspec tool is invoked
    Then the handler for "watcher-W" should be used
    And each session operates independently with no confusion

  @unit
  Scenario: Bridge tool wrapper stores session_id at construction
  # Bridge tool follows same pattern
    Given a session manager has created a session with ID "session-C"
    And bridge session context has been set for "session-C"
    When the session manager creates a Bridge tool with claude_bridge_tool(session_id)
    Then the Bridge tool wrapper should store session_id as a field
    When the LLM invokes the Bridge tool with action "list"
    Then the Bridge tool should use self.session_id for context lookup
    And the correct session context for "session-C" should be used

  @unit
  Scenario: create_rig_agent accepts session_id as first parameter
  # API signature change
    Given a provider instance (Claude, Gemini, OpenAI, or ZAI)
    When I call create_rig_agent(session_id, preamble, thinking_config)
    Then the method should accept session_id as the first parameter
    And the Fspec tool in the agent should be constructed with session_id
    And the Bridge tool in the agent should be constructed with session_id

  @unit
  Scenario: Thread-local current session functions are no longer needed
  # Deprecated functions - no longer needed but kept for backward compatibility
    Given the new session-at-construction architecture is implemented
    Then tools work without calling set_current_fspec_session()
    And tools work without calling set_current_bridge_session()
    But set_fspec_handler_for_session() should still exist
    And set_bridge_session_context() should still exist
