@done
@agent-view
@slash-command
@tui
@rust
@RPC-074
Feature: /clear emits TS-divergent scrollback notice — match TypeScript contract exactly

  """
  RPC-074 reverts a divergence introduced in RPC-046 (synthetic '[notice] /clear: history cleared' scrollback line) and in the StubSessionManagerHandle (StreamChunk::UserNotification with message 'history cleared'). The Rust port MUST behave exactly like src/tui/components/AgentView.tsx:1554-1564 (handleClearCommand): only clear the input, call backend.clear_history(session_id), and let the SessionStateChange { state: Cleared } chunk drive any downstream UI updates. Errors go to tracing::error!, never to scrollback. The StubSessionManagerHandle MUST emit a SessionStateChange { state: Cleared } chunk to mirror BackgroundSession::clear_history so cross-transport parity tests pass against the stub identically to the real impl.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. TS handleClearCommand (AgentView.tsx:1554-1564) only blanks the input and calls sessionClearHistory(currentSessionId) — it does NOT push any conversation entry
  #   2. On clear_history Err, TS routes the error to logger.error — never to the user-visible scrollback. The Rust port MUST mirror this and use tracing::error! only
  #   3. The reactive scrollback reset comes from the StreamChunk::SessionStateChange { state: Cleared } chunk emitted by BackgroundSession::clear_history (matches TS TUI-066 contract)
  #   4. The StubSessionManagerHandle MUST emit the same SessionStateChange { state: Cleared } chunk on clear_history so cross-transport parity tests can observe identical behaviour on both transports
  #   5. Source-shape regression: dispatch_slash_clear.rs MUST NOT contain the literal strings '[notice] /clear' or 'history cleared'; session_manager_handle.rs MUST NOT broadcast a StreamChunk::UserNotification carrying 'history cleared'
  #
  # EXAMPLES:
  #   1. Given a focused session s-1 with scrollback chunks, when SlashCommandSelected(Clear) is dispatched, then s-1's scrollback chunk_count becomes 0 AND no '[notice] /clear: history cleared' line is ever appended to scrollback
  #   2. Given a focused session and a backend whose clear_history returns Err('boom'), when /clear is dispatched, then no '[error] /clear failed: boom' line appears in scrollback — the error is only emitted via tracing::error!
  #   3. Given a subscriber on backend.chunks_rx(), when backend.clear_history(sid).await is called on either transport against the StubSessionManagerHandle, then within 1 second a StreamChunk::SessionStateChange { state: Cleared } arrives for sid — and NO StreamChunk::UserNotification with message 'history cleared' is observed
  #   4. Given the real fspec binary running under tui-test, when the user opens a Work Agent and types '/clear' followed by Enter, then the rendered scrollback after the keystroke does NOT contain '[notice] /clear:' or '[error] /clear failed:' anywhere
  #   5. Source-shape check: a grep for 'history cleared' in codelet/fspec-tui/src/app/dispatch_slash_clear.rs and codelet/core/src/session_manager_handle.rs returns zero matches
  #
  # ========================================

  Background: User Story
    As a Rust TUI user
    I want to type /clear and have it behave exactly like the TypeScript reference (only clear input + scrollback, no synthetic notice line)
    So that the Rust port matches TS byte-for-byte and does not invent UI text that the user never agreed to

  Scenario: /clear resets scrollback but does NOT append any [notice] line
    Given an App with an open session s-1 with 3 scrollback chunks wired to a MockBackend whose clear_history returns Ok(())
    When SlashCommandSelected(SlashCommandAction::Clear) is dispatched
    Then s-1's scrollback chunk_count becomes 0 synchronously
    Then after draining pending tasks and the action bus, s-1's scrollback contains zero lines matching "[notice] /clear"
    Then after draining pending tasks and the action bus, s-1's scrollback contains zero lines matching "history cleared"


  Scenario: /clear with backend Err does NOT append any [error] line to scrollback
    Given an App with an open session s-1 wired to a MockBackend whose clear_history returns Err("boom")
    When SlashCommandSelected(SlashCommandAction::Clear) is dispatched
    Then after draining pending tasks and the action bus, s-1's scrollback contains zero lines matching "[error] /clear failed"

