@done
@store
@header
@multi-session
@tokens
@rpc
@agent-view
@tui
@RPC-099
Feature: AgentView SessionHeader per-session token tracking parity — reasoning_tokens, tokens_per_second, cache_read/cache_creation, compaction_reduction not per-session in Rust port

  """
  TS REFERENCE (DeepSearch confirmed): src/tui/components/SessionHeader.tsx:104-206 is the renderer. It receives `tokenUsage` (local React state from current session's stream) and `rustTokens` (per-session SessionTokens via useSyncExternalStore on currentSessionId) as props and calls getMaxTokens(tokenUsage, rustTokens) at line 127. The display at lines 196-197 is `tokens: {input}↓ {output}↑ {reasoningTokens > 0 ? ` ${reasoningTokens}🧠` : ''}`. The Zustand sessionStore does NOT hold tokens — Rust SessionManager (per-session, NAPI sessionGetTokens) is the authoritative source.
  TS PARALLEL state: AgentView.tsx:836-839 holds local `tokenUsage` (TokenTracker) state. updateTokenStateFromChunk (1110-1125) calls setTokenUsage(chunk.tokens) on every TokenUpdate for the CURRENT session. On session switch via resumeSessionById (3532+), setTokenUsage is called again with restored tokens (3549) or extractTokenStateFromChunks (3609) for background sessions. Rust port can skip the parallel local-state mirror since token_state_by_session HashMap is already the single source per-session.
  RUST CURRENT GAP (verified by AST read):  TokenState in store/agent_view.rs:40-45 only has {input_tokens, output_tokens, context_fill_pct}. apply_token_tracker (lines 59-62) only copies input+output, drops cache_read_input_tokens, cache_creation_input_tokens, reasoning_tokens, tokens_per_second. chrome_paint.rs:58-60 HARDCODES tokens_per_second:None, reasoning_tokens:0, compaction_reduction:None — they never read from the per-session map. header.rs:66-71 declares fields tokens_per_second/reasoning_tokens/compaction_reduction but they always arrive as defaults at the render site.
  ROUTING IS ALREADY CORRECT: app/dispatch.rs:32 `apply_chunk_to_token_state(id, chunk)` uses the chunk's source id (NOT current_session). RPC-045 lifted the active-session filter in bootstrap.rs:92 so background chunks reach the App. Thus the fix is purely about (a) extending TokenState fields and (b) wiring chrome_paint.rs to read them per-session — no routing changes needed.
  TokenTracker shape (codelet/rpc-types/src/lib.rs:766-788): {input_tokens: i32, output_tokens: i32, cache_read_input_tokens: Option<u32>, cache_creation_input_tokens: Option<u32>, tokens_per_second: Option<f64>, reasoning_tokens: Option<u32>}. Rust TokenState should mirror these with appropriate widened types (u64 for counts, Option<f32 or f64> for tps).
  INTEGRATION TEST FIXTURE PLAN: use real App + real AgentViewStore (no mocks of those) + MockBackend; seed two sessions s-1, s-2 with attach_session/append_session; dispatch Action::ChunkReceived(s-1, TokenUpdate{...}) and Action::ChunkReceived(s-2, TokenUpdate{...}); render into 80x24 ratatui::TestBackend and assert the header line text via Buffer::get_line scraping; toggle focus via Action::SessionNext / Action::SessionPrev and re-render + re-assert. Use the existing helpers in codelet/fspec-tui/tests/common/mod.rs (sid(...), build_app, render_into, etc.).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. TokenState in AgentViewStore (codelet/fspec-tui/src/store/agent_view.rs:40-67) MUST be extended to track all TokenTracker fields: input_tokens, output_tokens, cache_read_input_tokens, cache_creation_input_tokens, reasoning_tokens, tokens_per_second, AND context_fill_pct — all persisted per SessionId in token_state_by_session HashMap
  #   2. TokenState::apply_token_tracker (agent_view.rs:59-62) MUST copy ALL TokenTracker fields from the incoming chunk — currently it only copies input_tokens and output_tokens and silently drops cache_read_input_tokens, cache_creation_input_tokens, reasoning_tokens, tokens_per_second
  #   3. chrome_paint::paint_header_and_role (codelet/fspec-tui/src/views/agent/chrome_paint.rs:25-71) MUST source tokens_per_second, reasoning_tokens, and compaction_reduction from store.token_state_for(sid) — currently these are hardcoded None/0/None at lines 58-60
  #   4. When the user does Shift+Left/Right (Action::SessionPrev / Action::SessionNext → handle_session_cycle → switch_to_session_index → focus_session_index), the next render frame's SessionHeader MUST display the NEW focused session's tokens — not the previous session's tokens and not zeros (unless the new session genuinely has no token state yet)
  #   5. Background sessions (those not currently focused) MUST continue to accumulate per-session TokenState as their TokenUpdate chunks arrive via Action::ChunkReceived(id, chunk) — apply_chunk_to_token_state already routes by source id (RPC-045 lifted the active-session filter), so background updates must NOT be filtered
  #   6. Per TS parity (src/tui/components/SessionHeader.tsx lines 196-197) the displayed header text MUST format as `tokens: {input}↓ {output}↑ {reasoning}🧠` when reasoning_tokens > 0, omitting the reasoning suffix otherwise — and when tokens_per_second is Some, it MUST drive the existing tok/sec indicator in build_right_line
  #
  # EXAMPLES:
  #   1. Two sessions s-1 and s-2 are open in AgentView. Action::ChunkReceived(s-1, TokenUpdate{input:100, output:50, reasoning_tokens:Some(20), tokens_per_second:Some(8.5)}) is dispatched. Action::ChunkReceived(s-2, TokenUpdate{input:200, output:75, reasoning_tokens:Some(60), tokens_per_second:Some(12.0)}) is dispatched. Render frame with s-1 focused shows `tokens: 100↓ 50↑ 20🧠` and tok/sec=8.5. Render frame after Action::SessionNext (now s-2 focused) shows `tokens: 200↓ 75↑ 60🧠` and tok/sec=12.0. Action::SessionPrev shows s-1 values again.
  #   2. Background session accumulation: s-1 is focused, s-2 is open but background. Action::ChunkReceived(s-2, TokenUpdate{input:200, output:75, reasoning_tokens:Some(60)}) is dispatched (user never visits s-2 between updates). User does Shift+Right to focus s-2. Header IMMEDIATELY displays 200↓ 75↑ 60🧠 — proving the background session's TokenState was being maintained while not visible.
  #   3. Cache token fields: Action::ChunkReceived(s-1, TokenUpdate{input:100, output:50, cache_read_input_tokens:Some(5000), cache_creation_input_tokens:Some(800)}) is dispatched. token_state_for(s-1) returns TokenState with cache_read = 5000 and cache_creation = 800 — these survive a switch to s-2 and a switch back.
  #   4. Empty-state on switch to never-updated session: s-1 has tokens 1234↓ 567↑. User opens fresh s-2 (no TokenUpdate dispatched yet) and does Shift+Right. Header shows `tokens: 0↓ 0↑` for s-2 (NOT 1234↓ 567↑ carry-over from s-1) because token_state_for(s-2) returns None and unwrap_or_default() yields zeros.
  #   5. Reasoning suffix toggling: s-1 has reasoning_tokens=0 (Default), s-2 has reasoning_tokens=Some(45). When focused on s-1, header text contains `tokens: in↓ out↑` (no 🧠 suffix). When focused on s-2, header text contains `tokens: in↓ out↑ 45🧠`.
  #
  # ========================================

  Background: User Story
    As a fspec TUI user with multiple concurrent sessions
    I want to see each session's own reasoning_tokens, tokens_per_second, cache_read/cache_creation token counts, and compaction reduction in the SessionHeader when I Shift+Left/Right between sessions
    So that I can monitor each session's resource usage independently and the Rust port behaves at parity with the original TS Ink implementation

  Scenario: Shift+Right swaps SessionHeader to the new session's full token totals
    Given two sessions "s-1" and "s-2" are open in AgentView with "s-1" focused
    And Action::ChunkReceived("s-1", StreamChunk::TokenUpdate { tokens: TokenTracker { input_tokens: 100, output_tokens: 50, reasoning_tokens: Some(20), tokens_per_second: Some(8.5), cache_read_input_tokens: Some(0), cache_creation_input_tokens: Some(0) } }) has been dispatched
    And Action::ChunkReceived("s-2", StreamChunk::TokenUpdate { tokens: TokenTracker { input_tokens: 200, output_tokens: 75, reasoning_tokens: Some(60), tokens_per_second: Some(12.0), cache_read_input_tokens: Some(0), cache_creation_input_tokens: Some(0) } }) has been dispatched
    When the App renders the AgentView into a 100x24 TestBackend with "s-1" focused
    Then the SessionHeader text contains "tokens: 100↓ 50↑" and "20🧠" and reflects tokens_per_second=8.5
    When the App dispatches Action::SessionNext and re-renders
    Then the SessionHeader text contains "tokens: 200↓ 75↑" and "60🧠" and reflects tokens_per_second=12.0
    When the App dispatches Action::SessionPrev and re-renders
    Then the SessionHeader text contains "tokens: 100↓ 50↑" and "20🧠" and reflects tokens_per_second=8.5

  Scenario: Background session accumulates token state while not focused
    Given two sessions "s-1" and "s-2" are open in AgentView with "s-1" focused
    When Action::ChunkReceived("s-2", StreamChunk::TokenUpdate { tokens: TokenTracker { input_tokens: 200, output_tokens: 75, reasoning_tokens: Some(60), tokens_per_second: None, cache_read_input_tokens: Some(0), cache_creation_input_tokens: Some(0) } }) is dispatched while "s-1" remains focused
    And the App dispatches Action::SessionNext to focus "s-2" and renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text immediately contains "tokens: 200↓ 75↑" and "60🧠" with no intermediate zero-state frame

  Scenario: cache_read_input_tokens and cache_creation_input_tokens are persisted per-session
    Given two sessions "s-1" and "s-2" are open in AgentView with "s-1" focused
    When Action::ChunkReceived("s-1", StreamChunk::TokenUpdate { tokens: TokenTracker { input_tokens: 100, output_tokens: 50, cache_read_input_tokens: Some(5000), cache_creation_input_tokens: Some(800), reasoning_tokens: None, tokens_per_second: None } }) is dispatched
    Then agent_view_store.token_state_for(SessionId("s-1")) returns Some(TokenState) with cache_read_input_tokens = 5000 and cache_creation_input_tokens = 800
    When the App dispatches Action::SessionNext (focus "s-2"), then Action::SessionPrev (focus "s-1") again
    Then agent_view_store.token_state_for(SessionId("s-1")) still returns Some(TokenState) with cache_read_input_tokens = 5000 and cache_creation_input_tokens = 800

  Scenario: Switching to a never-updated session displays zeros (no carry-over)
    Given session "s-1" is focused with TokenState { input_tokens: 1234, output_tokens: 567 } from a prior dispatched TokenUpdate
    And a fresh session "s-2" has been opened with append_session and no Action::ChunkReceived has been dispatched for it
    When the App dispatches Action::SessionNext to focus "s-2" and renders the AgentView into a 100x24 TestBackend
    Then the SessionHeader text contains "tokens: 0↓ 0↑"
    And the SessionHeader text does NOT contain "1234↓"
    And the SessionHeader text does NOT contain "567↑"

  Scenario: Reasoning brain suffix toggles based on the focused session's reasoning_tokens
    Given two sessions "s-1" and "s-2" are open with "s-1" focused
    And Action::ChunkReceived("s-1", StreamChunk::TokenUpdate { tokens: TokenTracker { input_tokens: 100, output_tokens: 50, reasoning_tokens: None, tokens_per_second: None, cache_read_input_tokens: Some(0), cache_creation_input_tokens: Some(0) } }) has been dispatched
    And Action::ChunkReceived("s-2", StreamChunk::TokenUpdate { tokens: TokenTracker { input_tokens: 200, output_tokens: 75, reasoning_tokens: Some(45), tokens_per_second: None, cache_read_input_tokens: Some(0), cache_creation_input_tokens: Some(0) } }) has been dispatched
    When the App renders the AgentView into a 100x24 TestBackend with "s-1" focused
    Then the SessionHeader text contains "tokens: 100↓ 50↑"
    And the SessionHeader text does NOT contain "🧠"
    When the App dispatches Action::SessionNext and re-renders
    Then the SessionHeader text contains "tokens: 200↓ 75↑" and "45🧠"
