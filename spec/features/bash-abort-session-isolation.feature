@BUG-129
Feature: BASH_ABORT_FLAG global AtomicBool causes ESC in one session to abort bash commands in all sessions

  """
  Primary: bash.rs (lines 36-75). Callers: session_manager.rs:1250 (request_bash_abort), session_manager.rs:1260 (clear_bash_abort), bash.rs internal (is_bash_abort_requested at 4 sites, clear_bash_abort at 2 sites). Re-exports: lib.rs:90. Cleanup: unregister_bash_abort_flag called in session_manager.rs destroy_session().
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. BASH_ABORT_FLAG must be a per-session HashMap<Uuid, Arc<AtomicBool>> instead of a global AtomicBool
  #   2. request_bash_abort and clear_bash_abort must accept session_id: Uuid as first parameter
  #   3. is_bash_abort_requested must accept session_id: Uuid and check only that session's flag
  #   4. Requesting abort for one session must not affect other sessions' bash commands
  #   5. All callers in bash.rs (spawn_stdout_reader, spawn_stderr_reader, wait_for_tasks_with_abort, BashTool::call) must thread session_id through to is_bash_abort_requested
  #   6. Checking abort for an unknown session_id must return false (not panic)
  #
  # EXAMPLES:
  #   1. Session A sets abort — only session A's is_bash_abort_requested returns true, session B's returns false
  #   2. Session A clears abort — session B remains unaffected
  #   3. Checking abort for an unknown session returns false without error
  #
  # ========================================

  Background: User Story
    As a developer running multiple concurrent agent sessions
    I want to have bash abort signals isolated per-session
    So that pressing ESC in one session only aborts bash commands in that session, not all sessions

  @unit
  Scenario: Per-session abort isolation — abort affects only the targeted session
    Given session A and session B both have bash abort flags registered
    When abort is requested for session A
    Then session A's abort flag is true
    And session B's abort flag is false

  @unit
  Scenario: Clearing abort for one session does not affect another session
    Given session A has abort requested
    And session B has abort requested
    When abort is cleared for session A
    Then session A's abort flag is false
    And session B's abort flag is still true

  @unit
  Scenario: Checking abort for an unknown session returns false without error
    Given no abort flag is registered for session C
    When abort status is checked for session C
    Then the result is false
    And no error or panic occurs
