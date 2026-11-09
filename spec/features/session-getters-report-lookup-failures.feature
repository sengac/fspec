@done
@BUG-445
@session
@session-management
@rust
Feature: Session getters report lookup failures
  """
  About 20 SessionManagerHandle getters returned a plain Option or a default
  value: get_pause_state, get_hitl_request, get_exec_stdin_request,
  get_compaction_progress, get_work_unit_context, get_pending_input,
  get_role and get_goal_state returned None; get_session_status Idle;
  get_session_tokens zeros; get_session_model / get_model_info zero-filled;
  get_debug_enabled false; get_continue_state (false, 10);
  get_buffered_output empty; schedule_list and blocklist_list an empty list
  (BUG-437 logged a missing start workspace). An invalid id, an unknown
  session and "nothing pending" read the same to the TUI, RPC clients and MCP.
  Now each getter returns Result (Result<Option<T>, String> where nothing
  pending is a real answer, Result<T, String> otherwise — the wire shape of
  BUG-437 get_effective_cwd and RPC-451 list_providers) on the handle trait
  (defaults refuse), the sessions implementation (the session lookup error
  propagates), the tarpc service in its frozen RPC-449 position (no session
  manager is refused), the MCP tools and both TUI transports.
  Tests: rust/fspec-tui/tests/bug445_session_getters_report_lookup_failures.rs.
  """

  Background: User Story
    As a TUI, RPC or MCP client reading session state
    I want to have a getter for an unknown session, or with no session manager, report the error instead of a default value
    So that an invalid id, an unknown session and nothing pending no longer read the same

  Scenario: Every session getter reports an unknown session
    Given a session manager with no session "00000000-0000-4000-8000-000000000445"
    When each session getter reads that session
    Then every getter fails naming "00000000-0000-4000-8000-000000000445"

  Scenario: A live session with nothing pending reads as nothing pending
    Given a session manager with a live session that has no role, pause, question or pending input
    When the role, pause state, question and pending input are read
    Then each read is Ok(None)

  Scenario: Session getters over RPC are refused without a session manager
    Given an RPC service with no session manager attached
    When the session role is read over the embedded transport
    Then the read fails saying no session manager is attached

  Scenario: Schedule and blocklist listings without a start workspace are errors
    Given a session manager with no start workspace
    When the scheduled jobs are listed
    Then the listing fails instead of returning an empty list
