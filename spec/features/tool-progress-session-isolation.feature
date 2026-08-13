@BUG-126
Feature: TOOL_PROGRESS_CALLBACK global singleton causes tool output to leak between concurrent sessions
  """
  Uses the same Lazy<RwLock<HashMap<Uuid, T>>> pattern as FSPEC_HANDLERS, SESSION_SEARCH_HANDLERS, etc.
  Primary file: rust/tools/src/tool_progress.rs. Callers: bash.rs (emit), stream_loop.rs (set/clear), gemini_continuation.rs (clear x5), compaction_retry.rs (clear x1). Re-exports: lib.rs:169.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. TOOL_PROGRESS_CALLBACK must be a per-session HashMap<Uuid, ToolProgressCallback> instead of a global Option<ToolProgressCallback>
  #   2. set_tool_progress_callback must accept session_id: Uuid as its first parameter
  #   3. emit_tool_progress must accept session_id: Uuid as its first parameter and dispatch only to that session's callback
  #   4. Clearing a callback for one session must not affect callbacks registered for other sessions
  #   5. All callers of emit_tool_progress (BashTool stdout/stderr readers) must thread the session_id from BashTool.session_id
  #   6. All callers of set_tool_progress_callback (stream_loop.rs, gemini_continuation.rs, compaction_retry.rs) must pass the session_id
  #   7. Emitting to a session_id with no registered callback must be a silent no-op (not a panic)
  #
  # EXAMPLES:
  #   1. Session A registers a progress callback, Session B registers a different one — emitting via Session A's ID only invokes Session A's callback
  #   2. Session B clears its callback — Session A continues to receive its tool progress normally
  #   3. Emitting tool progress for a session that never registered a callback is a silent no-op
  #   4. Session A's bash tool runs `ls -la` — only Session A's TUI displays the streaming output, Session B's TUI remains unaffected
  #   5. Multiple callbacks can be registered and active concurrently without interfering with each other
  #
  # ========================================
  Background: User Story
    As a developer running multiple concurrent agent sessions
    I want to have tool progress streaming isolated per-session
    So that bash tool output from one session never leaks into another session's TUI

  @unit
  Scenario: Per-session callback isolation — emit dispatches only to the registered session
    Given session A has registered a tool progress callback
    And session B has registered a different tool progress callback
    When tool progress is emitted for session A
    Then only session A's callback is invoked
    And session B's callback is not invoked

  @unit
  Scenario: Clearing one session's callback does not affect another session
    Given session A has registered a tool progress callback
    And session B has registered a tool progress callback
    When session B's callback is cleared
    And tool progress is emitted for session A
    Then session A's callback is invoked normally
    And emitting tool progress for session B is a no-op

  @unit
  Scenario: Emitting tool progress for an unregistered session is a silent no-op
    Given no callback is registered for session C
    When tool progress is emitted for session C
    Then no callback is invoked
    And no error or panic occurs

  @unit
  Scenario: Multiple concurrent callbacks operate independently
    Given session A has registered a tool progress callback capturing output to buffer A
    And session B has registered a tool progress callback capturing output to buffer B
    When tool progress "stdout line A" is emitted for session A
    And tool progress "stdout line B" is emitted for session B
    Then buffer A contains only "stdout line A"
    And buffer B contains only "stdout line B"

  @integration
  Scenario: Registering and clearing callbacks for many sessions concurrently
    Given 10 sessions have each registered a tool progress callback
    When tool progress is emitted for each session with a unique message
    Then each session's callback received only its own message
    And clearing all callbacks leaves the registry empty
