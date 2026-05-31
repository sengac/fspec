@done
@session-management
@rust
@multi-session
@rpc
@agent-view
@tui
@slash-command
@RPC-046
Feature: /clear slash command end-to-end
  """
  Phase 6.4 of the RPC-030 roadmap. This slice extends the RPC-020 /clear
  handler so it also calls the backend's clear_history RPC, mirroring the TS
  handleClearCommand() path (src/tui/views/AgentView.tsx line ~2730).

  TS parity (RPC-074, 2026-05-27): the previous "success notice" / "error
  notice" scrollback lines (`[notice] /clear: history cleared` and
  `[error] /clear failed: <reason>`) were pure Rust-side invention with no
  counterpart in the TS reference (AgentView.tsx:1554-1564 handleClearCommand
  only blanks the input and calls sessionClearHistory; errors route to
  logger.error). RPC-074 removed those notice lines from
  dispatch_rpc046::handle_slash_clear so the Rust port behaves exactly like
  TS. The reactive UI reset is driven by the
  `StreamChunk::SessionStateChange { state: Cleared }` chunk emitted by
  `BackgroundSession::clear_history` (TUI-066 contract). See
  spec/features/rpc074-clear-ts-parity.feature for the parity contract.

  Wiring lives in app/dispatch_rpc046.rs::handle_slash_clear. Spawned
  tokio task pattern matches dispatch_rpc022's
  handle_thinking_level_selected — clone backend, spawn .await,
  log any Err via tracing::error! (NOT via Action::EmitSessionNotice).

  FspecBackend::clear_history was widened in RPC-037 with a default Ok(())
  impl. EmbeddedFspecBackend and WebSocketFspecBackend both delegate to
  FspecService::clear_history. No transport-level work required for
  RPC-046.

  Out of scope: confirm-dialog before destructive clear (TS does not have
  one either — out of parity scope per the attachment).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SlashCommandAction::Clear MUST call backend.clear_history(session_id) in addition to resetting the local scrollback
  #   2. (RPC-074 superseded) On a successful clear_history response, NO scrollback notice line is emitted — the reactive UI reset is driven by the SessionStateChange { state: Cleared } chunk
  #   3. (RPC-074 superseded) On a failed clear_history response, NO scrollback error line is emitted — errors route to tracing::error! only (TS parity with logger.error)
  #   4. /clear with no current session is a no-op (does not call the backend; does not emit a notice)
  #   5. The local scrollback reset MUST happen synchronously and unconditionally (it does not wait for the backend response)
  #   6. Backend round-trip happens via tokio::spawn so it does not block the App dispatch task
  #
  # EXAMPLES:
  #   1. Given an App with an open session s-1 and 5 scrollback chunks, when SlashCommandSelected(Clear) is dispatched, then s-1's scrollback chunk_count becomes 0 synchronously
  #   2. Given an App with an open session s-1 and a MockBackend whose clear_history returns Ok(()), when SlashCommandSelected(Clear) is dispatched, then within 1 second backend.clear_history(s-1) is called exactly once
  #   3. (RPC-074 retired) — covered by spec/features/rpc074-clear-ts-parity.feature
  #   4. (RPC-074 retired) — covered by spec/features/rpc074-clear-ts-parity.feature
  #   5. Given an App with NO current session, when SlashCommandSelected(Clear) is dispatched, then backend.clear_history is never called and no notice/error line is appended
  #   6. Given two open sessions s-1 (focused) and s-2 (background) both with scrollback chunks, when SlashCommandSelected(Clear) is dispatched, then ONLY s-1's scrollback is reset; s-2's scrollback is untouched
  #
  # ========================================
  Background: User Story
    As a user with an open AgentView session
    I want to use the /clear slash command
    So that the conversation history is removed from BOTH the local scrollback AND the backend's persisted session manifest in one action

  Scenario: /clear resets local scrollback synchronously for the focused session
    Given an App with an open session s-1 whose scrollback has 5 chunks
    When SlashCommandSelected(SlashCommandAction::Clear) is dispatched
    Then s-1's scrollback chunk_count becomes 0 synchronously
    And the MultiLineInput's buffer is empty

  Scenario: /clear calls backend.clear_history for the focused session
    Given an App with an open session s-1 wired to a MockBackend whose clear_history returns Ok(())
    When SlashCommandSelected(SlashCommandAction::Clear) is dispatched
    Then within 1 second backend.clear_history is called exactly once with session_id s-1

  Scenario: /clear with no current session is a silent no-op
    Given an App with NO current session
    When SlashCommandSelected(SlashCommandAction::Clear) is dispatched
    Then backend.clear_history is never called
    And no scrollback chunk is appended to any session

  Scenario: /clear only affects the focused session — background sessions are untouched
    Given an App with two open sessions s-1 (focused) and s-2 (background), each with 3 scrollback chunks
    And the MockBackend's clear_history returns Ok(())
    When SlashCommandSelected(SlashCommandAction::Clear) is dispatched
    Then s-1's scrollback chunk_count becomes 0 synchronously
    And s-2's scrollback chunk_count remains 3
    And within 1 second backend.clear_history is called exactly once with session_id s-1
