@done
@BUG-464
@session
@session-management
@rust
Feature: Remaining session methods report failures
  """
  Found while doing BUG-445. Outside its 17 getters, SessionManagerHandle
  still hid missing sessions and session managers: get_thinking_level
  returned Off (trait default, sessions impl for an unknown session and the
  RPC without a session manager); get_supervisors returned an empty list;
  set_pending_input and set_debug_enabled had no error channel and silently
  did nothing for an unknown session or without a session manager; the
  trait defaults of set_role, clear_history, compact_session (a zeroed
  result), restore_session_messages, restore_session_token_state,
  set_work_unit_context, pause_resume / pause_confirm / pause_triple,
  send_hitl_response, toggle_debug and set_debug_directory returned Ok;
  and get_session_model / get_model_info turned a poisoned lock or an unset
  provider / model id into an empty string.
  Now get_thinking_level and get_supervisors return Result<T, String>,
  set_pending_input and set_debug_enabled return Result<(), String> (trait,
  sessions, tarpc in its RPC-449 position, MCP and both TUI transports),
  every listed default refuses, and the model reads report the lock or the
  unset id.
  Tests: rust/fspec-tui/tests/bug464_remaining_session_methods_report_failures.rs.
  """

  Background: User Story
    As a TUI, RPC or MCP client acting on a session
    I want to have every remaining session method report an unknown session, a missing session manager or an unimplemented method
    So that no read returns a default and no write silently does nothing

  Scenario: Thinking level, supervisors, pending input and debug writes report an unknown session
    Given a session manager with no session "00000000-0000-4000-8000-000000000464"
    When the thinking level and supervisors are read and the pending input and debug flag are written for that session
    Then every call fails naming "00000000-0000-4000-8000-000000000464"

  Scenario: Handle defaults refuse instead of answering
    Given a session manager handle that does not implement set_role or get_thinking_level
    When the role is set and the thinking level is read through it
    Then both calls fail saying the method is not implemented by this session manager

  Scenario: Session writes over RPC are refused without a session manager
    Given an RPC service with no session manager attached
    When the pending input is written and the thinking level is read over the embedded transport
    Then both calls fail saying no session manager is attached
