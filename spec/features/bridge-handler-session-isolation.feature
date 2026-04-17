@BUG-128
Feature: BRIDGE_HANDLER global singleton causes bridge tool actions to route to wrong session
  """
  Primary: bridge_handler.rs. set_bridge_handler callers: session_manager.rs:5144 (set), session_manager.rs:5446 (clear). execute_bridge_command caller: facade/wrapper.rs:1668. has_bridge_handler_for_session caller: facade/wrapper.rs:1650. No change needed in wrapper — it already passes session_id in BridgeRequest.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. BRIDGE_HANDLER must be a per-session HashMap<Uuid, BridgeHandler> instead of a global Option<BridgeHandler>
  #   2. set_bridge_handler must accept session_id: Uuid as first parameter
  #   3. execute_bridge_command must look up the handler by request.session_id from the per-session map
  #   4. has_bridge_handler_for_session must check the per-session BRIDGE_HANDLERS map instead of the global singleton
  #   5. Clearing a handler for one session must not affect handlers registered for other sessions
  #
  # EXAMPLES:
  #   1. Session A registers a bridge handler, Session B registers a different one — execute_bridge_command with session A's ID invokes only A's handler
  #   2. Session B clears its handler — session A's bridge commands still work correctly
  #   3. execute_bridge_command for a session with no handler returns an error result (not panic)
  #   4. has_bridge_handler_for_session returns true only when both per-session handler AND context exist for that session
  #
  # ========================================
  Background: User Story
    As a developer running multiple concurrent agent sessions with bridge connections
    I want to have bridge command dispatch isolated per-session
    So that bridge connect/disconnect/list actions route to the correct session's handler

  @unit
  Scenario: Per-session handler isolation — execute dispatches only to the registered session
    Given session A has registered a bridge handler returning a success result
    And session B has registered a bridge handler returning a different result
    When execute_bridge_command is called with session A's ID
    Then only session A's handler is invoked
    And the result matches session A's handler response

  @unit
  Scenario: Clearing one session's handler does not affect another session
    Given session A has registered a bridge handler
    And session B has registered a bridge handler
    When session B's handler is cleared
    And execute_bridge_command is called with session A's ID
    Then session A's handler is invoked normally
    And execute_bridge_command with session B's ID returns not-configured error

  @unit
  Scenario: execute_bridge_command for an unregistered session returns error
    Given no bridge handler is registered for session C
    When execute_bridge_command is called with session C's ID
    Then the result indicates handler not configured
    And no error or panic occurs

  @unit
  Scenario: has_bridge_handler_for_session checks per-session handler and context
    Given session A has a registered bridge handler and session context
    And session B has neither handler nor context
    When has_bridge_handler_for_session is queried for session A
    Then it returns true
    When has_bridge_handler_for_session is queried for session B
    Then it returns false
