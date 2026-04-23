@BUG-134
Feature: Refactor DebugCaptureManager to be truly per-session (one log file per BackgroundSession)
  """
  RUST LAYER 1 - DebugCaptureManager ownership change: Currently codelet/common/src/debug_capture/mod.rs:25 has static OnceLock singleton. Refactor so DebugCaptureManager is stored on BackgroundSession (session_manager.rs:486+ struct). BackgroundSession already has pub id: Uuid (line 486) as a perfect key. Add a field like pub debug_capture: Arc<PoisonRecoveryMutex<DebugCaptureManager>> initialized in BackgroundSession::new() (line 628+). Remove the static OnceLock.
  RUST LAYER 2 - Free function removal/refactoring: The convenience free functions in mod.rs must change: (a) get_debug_capture_manager() - remove or make private, callers must go through session, (b) capture_event(event_type, data) - remove, callers must have session context, (c) increment_debug_turn() - remove, callers must have session context, (d) handle_debug_command_with_dir(base_dir) - refactor to take a &BackgroundSession or &DebugCaptureManager and toggle only that session's manager. The global toggle_debug() NAPI (session_manager.rs:7512) should be removed since all debug toggles require a session context in multi-session mode.
  RUST LAYER 3 - Capture call site migration (NAPI path). The NAPI call sites in codelet/napi/src/session_manager.rs already have session_id and are easiest to migrate: session_update_debug_metadata(:7529), session_toggle_debug(:7568), session_compact(:7613/:7635/:7661). These already do get_session(session_id) so they can access session.debug_capture directly instead of the global singleton.
  RUST LAYER 3 - Capture call site migration (CLI path). The CLI call sites use codelet CLI Session (NOT BackgroundSession). Files: repl_loop.rs (8 calls), stream_loop.rs (7+ calls), stream_handlers.rs (3 calls), recovery_compaction.rs (3 calls), gemini_continuation.rs (1 call). In CLI mode there is only one session so these can either: (a) keep a single-session DebugCaptureManager on the CLI Session struct, or (b) the CLI codepath may not be affected if fspec TUI always uses NAPI path. Determine which path the TUI agent loop uses.
  RUST LAYER 3 - Capture call site migration (tracing layer). codelet/common/src/logging/mod.rs DebugCaptureLayer (line 28-50) is a tracing subscriber layer with NO session context. Options: (a) Make log.entry events include a session_id span field and route to the correct manager, (b) remove debug capture from the tracing layer entirely, (c) keep the tracing layer as a global fallback that writes to a shared 'system' debug log. Recommend option (a) using tracing spans.
  RUST LAYER 4 - Stream event emission. After session_toggle_debug flips the per-session manager, it must emit StreamChunk::DebugStateChange { enabled: bool } on that session's stream. Add variant to StreamChunk enum in codelet/napi/src/types.rs (near IsolationStateChange at :543/:718). Emit from session_toggle_debug (session_manager.rs:7555+) using session.stream_sender if available, same pattern as IsolationStateChange emission.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The process-wide OnceLock singleton DebugCaptureManager (codelet/common/src/debug_capture/mod.rs:25) must be replaced with per-session managers owned by BackgroundSession
  #   2. Each session's debug JSONL file MUST include the BackgroundSession.id (Uuid) in its path (e.g. ~/.fspec/debug/{session_id}/session-{timestamp}.jsonl) so files are unambiguously owned by one session
  #   3. Toggling /debug in session A MUST NOT affect session B's capture state: B's manager.is_enabled() must remain unchanged, B's file must remain open/closed as it was
  #   4. All capture call sites that currently use the global get_debug_capture_manager() must be refactored to route through the session's own manager
  #   5. The free functions capture_event() and increment_debug_turn() in mod.rs must either be removed or gain a session_id parameter so callers route to the correct session manager
  #   6. session_toggle_debug NAPI function must toggle only the requesting session's manager, not a global toggle
  #   7. Rust must emit StreamChunk::DebugStateChange { session_id, enabled } when a session's debug state toggles, so the TUI layer (BUG-133) can replicate state to Zustand
  #   8. The latest.jsonl symlink should remain functional: it should point to the most recently activated session's debug file (last session to toggle /debug on)
  #
  # EXAMPLES:
  #   1. Session A runs /debug: capture starts writing to ~/.fspec/debug/{session_A_id}/session-{ts}.jsonl. Session B runs concurrently and makes API calls. Session A's JSONL contains ONLY session A's events; B's events are NOT captured (B has not toggled /debug)
  #   2. Both session A and B run /debug: both have independent JSONL files. Toggling /debug off in A stops only A's capture. B's capture continues uninterrupted in its own file
  #   3. Session A toggles /debug on: Rust emits DebugStateChange(session_A_id, true) on A's stream. TUI picks it up and shows [DEBUG] badge only for A. B's header remains unchanged
  #
  # ========================================
  Background: User Story
    As a developer debugging a multi-session TUI
    I want to have each session's /debug capture write to its own isolated JSONL file
    So that debug logs contain only that session's events and toggling debug in one session doesn't affect other sessions' capture state

  @isolation
  Scenario: Debug capture for one session does not leak into another session's log file
    Given session A and session B are running concurrently
    And session A has debug capture enabled
    And session B has debug capture disabled
    When session B makes API calls and processes tool results
    Then session A's JSONL file should contain only session A's events
    And session B should have no debug JSONL file

  @isolation
  Scenario: Each session writes to its own session-specific debug file path
    Given session A has id "aaaa-1111"
    When session A enables debug capture
    Then a debug JSONL file should be created under "~/.fspec/debug/aaaa-1111/"
    And the filename should follow the pattern "session-{timestamp}.jsonl"

  @independence
  Scenario: Toggling debug off in one session does not affect another session's capture
    Given session A and session B both have debug capture enabled
    And each session is writing to its own independent JSONL file
    When session A toggles debug off
    Then session A's capture should stop and its file should be closed
    And session B's capture should continue writing uninterrupted to its own file

  @stream-event
  Scenario: Toggling debug emits a DebugStateChange stream event scoped to that session
    Given session A is running
    When session A toggles debug on via the "/debug" command
    Then Rust should emit a DebugStateChange stream event with enabled true on session A's stream only
    And session B's stream should not receive any DebugStateChange event

  @symlink
  Scenario: latest.jsonl symlink points to the most recently activated session's debug file
    Given session A enables debug capture at time T1
    And session B enables debug capture at time T2 where T2 is after T1
    Then the "latest.jsonl" symlink should point to session B's debug file
    When session B toggles debug off
    Then the "latest.jsonl" symlink should remain pointing to session B's last file

  @per-session-manager
  Scenario: DebugCaptureManager is owned by BackgroundSession not a global singleton
    Given the process has multiple BackgroundSession instances
    Then each BackgroundSession should own its own DebugCaptureManager instance
    And there should be no process-wide OnceLock singleton for debug capture
    And each manager should be independently toggleable
