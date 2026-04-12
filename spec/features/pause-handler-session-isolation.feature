@BUG-127
Feature: PAUSE_HANDLER global singleton causes pause interactions to route to wrong session

  """
  Uses the same Lazy<RwLock<HashMap<Uuid, T>>> pattern as BUG-126 TOOL_PROGRESS_CALLBACKS and existing FSPEC_HANDLERS
  Primary: tool_pause.rs. Callers: session_manager.rs:4789 (set), session_manager.rs:5435 (clear). pause_for_user callers: web_search.rs (3 sites), blocklist/middleware.rs (2 sites).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. PAUSE_HANDLER must be a per-session HashMap<Uuid, PauseHandler> instead of a global Option<PauseHandler>
  #   2. set_pause_handler, pause_for_user, and has_pause_handler must all accept session_id: Uuid as first parameter
  #   3. Clearing a handler for one session must not affect handlers registered for other sessions
  #   4. Calling pause_for_user with a session_id that has no registered handler must return PauseResponse::Resumed (not panic)
  #   5. All callers of pause_for_user (WebSearch, blocklist middleware) must pass the session_id from the tool wrapper
  #
  # EXAMPLES:
  #   1. Session A registers a pause handler, Session B registers a different one — pause_for_user(session_a_id, ...) invokes only session A's handler
  #   2. Session B clears its handler — session A's pause interactions still work correctly
  #   3. Pausing for a session with no registered handler returns Resumed without error
  #   4. has_pause_handler checks only whether the specified session has a handler, not any session
  #
  # ========================================

  Background: User Story
    As a developer running multiple concurrent agent sessions
    I want to have pause interactions isolated per-session
    So that blocklist prompts and WebSearch pause dialogs appear in the correct session's TUI

  @unit
  Scenario: Per-session handler isolation — pause dispatches only to the registered session
    Given session A has registered a pause handler that returns Approved
    And session B has registered a pause handler that returns Denied
    When pause_for_user is called with session A's ID
    Then only session A's handler is invoked
    And the response is Approved

  @unit
  Scenario: Clearing one session's handler does not affect another session
    Given session A has registered a pause handler
    And session B has registered a pause handler
    When session B's handler is cleared
    And pause_for_user is called with session A's ID
    Then session A's handler is invoked normally
    And pause_for_user with session B's ID returns Resumed

  @unit
  Scenario: Pausing for an unregistered session returns Resumed without error
    Given no pause handler is registered for session C
    When pause_for_user is called with session C's ID
    Then the response is Resumed
    And no error or panic occurs

  @unit
  Scenario: has_pause_handler checks only the specified session
    Given session A has registered a pause handler
    And session B has no registered pause handler
    When has_pause_handler is queried for session A
    Then it returns true
    When has_pause_handler is queried for session B
    Then it returns false
