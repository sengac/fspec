@done
@RPC-055
@rpc
@agent-view
@tui
@slash-command
@rust
@multi-session
@session-management
Feature: /debug debug-capture wiring
  """
  Phase 7.2 of the RPC-030 roadmap. The DebugCaptureManager already lives in
  codelet-common::debug_capture (NAPI-free, per-session). The RPC-037
  surface (SessionManagerHandle::toggle_debug + FspecService::toggle_debug
  + EmbeddedFspecBackend/WebSocketFspecBackend forwarders) was wired in
  RPC-037. The DebugStateChange chunk handler in dispatch_stream_chunks.rs was
  wired in RPC-045. The SessionHeader's [DEBUG] badge field was added in
  RPC-029 but is currently hardcoded to false.

  This slice connects the loose ends:

  1. Add a NEW `set_debug_directory(path)` RPC method through the trait,
  FspecService, FspecBackend, and both transports — needed for the
  pre-session global toggle path (the TS reference calls
  `toggleDebug(debugDir)` when no session is active).
  2. Replace the `SlashCommandAction::Debug` notice fallback in
  dispatch_slash_commands.rs::handle_slash_command with a real
  backend.toggle_debug(session_id, debug_dir) round-trip in a new
  app/dispatch_slash_debug.rs file, mirroring the dispatch_slash_clear (/clear)
  and dispatch_provider_settings (/provider) patterns.
  3. Replace the hardcoded `is_debug_enabled: false` in views/agent.rs
  with `agent_view_store.debug_enabled_for(session_id).unwrap_or(false)`
  so the existing SessionHeader [DEBUG] badge reflects live state.

  TS reference: `AgentView.tsx` line 2643 — `sessionToggleDebug(currentSessionId, debugDir)`
  if a session is active, else `toggleDebug(debugDir)` (pre-session global).
  Badge rendering: `src/tui/components/SessionHeader.tsx` line 170
  emits `chalk.red.bold(' [DEBUG]')` when `isDebugEnabled` is true.

  Out of scope: reading captured debug files in-TUI; the pre-session
  global toggle UX (separate concern — the new `set_debug_directory` RPC
  method exists for it but no slash command currently wires it).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SessionManagerHandle MUST expose a default-impl method set_debug_directory(path: PathBuf) -> Result<(), String> so existing handles compile unchanged
  #   2. StubSessionManagerHandle overrides set_debug_directory with deterministic in-memory state and exposes a per-call counter for cross-transport parity tests
  #   3. FspecService (tarpc) declares async fn set_debug_directory(path: String) -> Result<(), String> and FspecServiceImpl routes through self.inner.session_manager()
  #   4. FspecBackend trait exposes async set_debug_directory(path: String) -> Result<()> with a default Ok(()) impl; both EmbeddedFspecBackend and WebSocketFspecBackend forward to the tarpc client
  #   5. SlashCommandAction::Debug dispatch path: with a current session, spawn backend.toggle_debug(session_id, debug_dir) and emit a session-scoped [debug] notice with the resolved file path on Ok or an [error] notice on Err
  #   6. /debug with no current session is a silent no-op: backend.toggle_debug is never called and no notice is emitted
  #   7. The /debug handler resolves debug_dir from the FSPEC_DEBUG_DIR environment variable, falling back to .fspec/debug when unset
  #   8. SessionHeader's [DEBUG] badge MUST read from agent_view_store.debug_enabled_for(current_session) instead of being hardcoded to false, so the badge appears/disappears in sync with DebugStateChange chunks delivered via RPC-045
  #   9. Cross-transport parity: invoking set_debug_directory and toggle_debug through both EmbeddedFspecBackend and WebSocketFspecBackend MUST land on the same StubSessionManagerHandle with identical per-call counter increments
  #
  # ========================================
  Background: User Story
    As a user with an open AgentView session
    I want to use the /debug slash command to toggle debug capture on or off
    So that I can capture per-session LLM diagnostics to a JSONL file on disk for later inspection

  Scenario: /debug calls backend.toggle_debug for the focused session
    Given an App with an open session s-1 wired to a MockBackend whose toggle_debug returns Ok("/tmp/debug/s-1/session-x.jsonl")
    When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    Then within 1 second backend.toggle_debug is called exactly once with session_id s-1
    And the debug_dir argument equals ".fspec/debug"

  Scenario: /debug emits a success notice with the resolved file path on Ok
    Given an App with an open session s-1 wired to a MockBackend whose toggle_debug returns Ok("/tmp/debug/s-1/session-x.jsonl")
    When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    Then within 1 second s-1's scrollback contains a chunk whose text equals "[debug] capture toggled → /tmp/debug/s-1/session-x.jsonl"

  Scenario: /debug emits an error notice on Err
    Given an App with an open session s-1 wired to a MockBackend whose toggle_debug returns Err("disk full")
    When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    Then within 1 second s-1's scrollback contains a chunk whose text equals "[error] /debug failed: disk full"

  Scenario: /debug with no current session is a silent no-op
    Given an App with NO current session
    When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    Then backend.toggle_debug is never called
    And no scrollback chunk is appended to any session

  Scenario: /debug only affects the focused session — background sessions are untouched
    Given an App with two open sessions s-1 (focused) and s-2 (background)
    And the MockBackend's toggle_debug returns Ok("/tmp/debug/s-1.jsonl")
    When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    Then within 1 second backend.toggle_debug is called exactly once with session_id s-1
    And within 1 second s-1's scrollback contains a chunk whose text starts with "[debug] capture toggled"
    And s-2's scrollback does NOT contain any chunk whose text starts with "[debug] capture toggled"

  Scenario: /debug honours the FSPEC_DEBUG_DIR environment variable
    Given the environment variable FSPEC_DEBUG_DIR is set to "/custom/path"
    And an App with an open session s-1 wired to a MockBackend whose toggle_debug returns Ok("/custom/path/s-1/session-x.jsonl")
    When SlashCommandSelected(SlashCommandAction::Debug) is dispatched
    Then within 1 second backend.toggle_debug is called exactly once with session_id s-1
    And the debug_dir argument equals "/custom/path"

  Scenario: SessionHeader [DEBUG] badge reflects per-session debug_enabled state
    Given an App with an open session s-1
    And the AgentViewStore has debug_enabled_by_session[s-1] set to true
    When the AgentView is rendered for s-1
    Then the SessionHeader emits a span with text " [DEBUG]" styled red bold

  Scenario: SessionHeader [DEBUG] badge disappears when debug_enabled flips off
    Given an App with an open session s-1
    And the AgentViewStore has debug_enabled_by_session[s-1] set to false
    When the AgentView is rendered for s-1
    Then the SessionHeader emits NO span with text " [DEBUG]"

  Scenario: DebugStateChange chunk from backend updates the badge state for the focused session
    Given an App with an open session s-1 whose debug_enabled is initially false
    When a StreamChunk::DebugStateChange { enabled: true } is delivered as Action::ChunkReceived(s-1, chunk)
    Then the AgentViewStore.debug_enabled_for(s-1) returns Some(true)
    And the SessionHeader for s-1 emits a span with text " [DEBUG]" styled red bold
