@RPC-420
@done
@tui-component
@slash-command
@tui
@agent-view
@rpc
@multi-session
@rust
@session-management
@RPC-047
Feature: /compact slash command + compaction progress footer
  """
  Wire-shape: backend.compact_session is the FspecBackend trait method widened in RPC-037; both EmbeddedFspecBackend and WebSocketFspecBackend already delegate to FspecService::compact_session. No transport-level work required for RPC-047.
  Slash-command wiring goes into app/dispatch_slash_commands.rs::handle_slash_command(Compact) — extending the existing arm (which currently falls through to the `[notice] /compact not yet implemented` fallback). Pattern matches RPC-046's Clear arm: synchronous nothing on the focused session, then spawn tokio task -> Action::EmitSessionNotice.
  CompactionComplete StreamChunk handler lives in dispatch_stream_chunks.rs::handle_stream_chunk_state_updates. A new arm for StreamChunk::CompactionComplete clears the per-session compaction_progress entry AND dispatches Action::EmitSessionNotice via self.action_tx so the notice lands in the originating session's scrollback regardless of focus.
  AgentViewStore extension: new pub(crate) compaction_progress_by_session: HashMap<SessionId, CompactionProgress> field + accessors get_compaction_progress / set_compaction_progress / clear_compaction_progress. Accessors live in store/agent_view/isolation_state.rs (existing per-session push-state sub-module) to keep agent_view.rs under its 300-LoC ceiling per the RPC-025 source-shape invariant.
  SessionFooter widget gains a `compaction_progress: Option<&CompactionProgress>` field, painted left-aligned BEFORE the existing right-aligned cwd/branch block. Bar uses U+25B0 (▰) for filled and U+25B1 (▱) for empty, fixed 10 cells. Caller (views/agent.rs::render_with_store) reads agent_view_store.compaction_progress_for(current_session).
  Notice-line format constant: helper `format_compaction_notice(result: &CompactionResult) -> String` lives in app/dispatch_slash_commands.rs (next to the /compact handler). Used by BOTH the slash-handler success branch AND the CompactionComplete chunk handler in dispatch_stream_chunks.rs so the formatting stays single-sourced.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SlashCommandAction::Compact MUST call backend.compact_session(session_id) on the focused session via a spawned tokio task
  #   2. On a successful compact_session response, a `[compaction] X.X% reduction (Y → Z tokens, T turns summarised)` line is emitted into the originating session's scrollback (X.X = compression_ratio rendered directly to 1 decimal place — the wire value is already the percent of tokens removed, RPC-420)
  #   3. On a failed compact_session response, a `[error] /compact failed: {reason}` line is emitted into the originating session's scrollback
  #   4. /compact with no current session is a silent no-op (backend.compact_session is never called; no notice is emitted)
  #   5. AgentViewStore tracks per-session compaction progress in a `compaction_progress_by_session: HashMap<SessionId, CompactionProgress>` accessor pair (get + set + clear)
  #   6. Receiving a StreamChunk::CompactionComplete in App::dispatch MUST clear the per-session compaction_progress entry AND emit the `[compaction] ...` notice into that session's scrollback (mirrors auto-compaction in the agent loop)
  #   7. SessionFooter MUST render a `[compacting: <phase> <current>/<total>]` segment + a 10-cell `▰▰▰▰▰▱▱▱▱▱`-style progress bar (filled = current/total * 10) on the left of the footer row whenever compaction_progress_for(focused_session) is Some
  #   8. When compaction_progress_for(focused_session) is None, SessionFooter paints the existing RPC-029 layout unchanged (no progress segment; cwd + branch on the right)
  #   9. Backend round-trip happens via tokio::spawn so it does not block the App dispatch task; spawned task dispatches Action::EmitSessionNotice(session_id, text) so the notice lands on the originating session even after a focus switch (mirrors RPC-046)
  #
  # EXAMPLES:
  #   1. Given an App with an open session s-1 wired to a MockBackend whose compact_session returns Ok(CompactionResult { compression_ratio: 60.0, original_tokens: 10000, compacted_tokens: 4000, turns_summarized: 12, turns_kept: 3 }), when SlashCommandSelected(Compact) is dispatched, then within 1 second backend.compact_session(s-1) is called exactly once
  #   2. Given the same Ok response, when SlashCommandSelected(Compact) is dispatched, then within 1 second s-1's scrollback contains the line `[compaction] 60.0% reduction (10000 → 4000 tokens, 12 turns summarised)`
  #   3. Given an App with an open session s-1 and a MockBackend whose compact_session returns Err("out of memory"), when SlashCommandSelected(Compact) is dispatched, then within 1 second s-1's scrollback contains `[error] /compact failed: out of memory`
  #   4. Given an App with NO current session, when SlashCommandSelected(Compact) is dispatched, then backend.compact_session is never called and no scrollback line is appended
  #   5. Given an App with an open session s-1 whose compaction_progress is Some(CompactionProgress { phase: "summarising messages", current: 5, total: 10 }), when SessionFooter is rendered, then the left side contains the substring `[compacting: summarising messages 5/10]` followed by a 10-char bar with 5 filled (`▰▰▰▰▰▱▱▱▱▱`)
  #   6. Given an App with an open session s-1 and compaction_progress is Some, when ChunkReceived(s-1, StreamChunk::CompactionComplete { compaction_result }) is dispatched, then compaction_progress_for(s-1) becomes None AND s-1's scrollback gains the `[compaction] ...` notice line
  #   7. Given two open sessions s-1 (focused) and s-2 (background) and the MockBackend's compact_session returns Ok(default CompactionResult), when SlashCommandSelected(Compact) is dispatched, then ONLY s-1 receives the `[compaction] ...` notice (s-2's scrollback is untouched)
  #   8. Given an App with no open session, when SessionFooter is rendered with workspace = None, then the row paints the existing dark-grey background and contains NO `[compacting: ...]` substring
  #
  # ========================================
  Background: User Story
    As a user with an open AgentView session
    I want to use the /compact slash command
    So that the AI conversation history is summarised on the backend AND I see live progress + a final compression-ratio notice in the focused session's scrollback

  Scenario: /compact calls backend.compact_session for the focused session
    Given an App with an open session s-1 wired to a MockBackend whose compact_session returns Ok with compression_ratio 60.0, original_tokens 10000, compacted_tokens 4000, turns_summarized 12, turns_kept 3
    When SlashCommandSelected(SlashCommandAction::Compact) is dispatched
    Then within 1 second backend.compact_session is called exactly once with session_id s-1

  Scenario: /compact emits a success notice into the originating session's scrollback on Ok
    Given an App with an open session s-1 wired to a MockBackend whose compact_session returns Ok with compression_ratio 60.0, original_tokens 10000, compacted_tokens 4000, turns_summarized 12, turns_kept 3
    When SlashCommandSelected(SlashCommandAction::Compact) is dispatched
    Then within 1 second s-1's scrollback contains a chunk whose text equals "[compaction] 60.0% reduction (10000 → 4000 tokens, 12 turns summarised)"

  Scenario: /compact emits an error notice into the originating session's scrollback on Err
    Given an App with an open session s-1 wired to a MockBackend whose compact_session returns Err("out of memory")
    When SlashCommandSelected(SlashCommandAction::Compact) is dispatched
    Then within 1 second s-1's scrollback contains a chunk whose text equals "[error] /compact failed: out of memory"

  Scenario: /compact with no current session is a silent no-op
    Given an App with NO current session
    When SlashCommandSelected(SlashCommandAction::Compact) is dispatched
    Then backend.compact_session is never called
    And no scrollback chunk is appended to any session

  Scenario: /compact only affects the focused session — background sessions are untouched
    Given an App with two open sessions s-1 (focused) and s-2 (background)
    And the MockBackend's compact_session returns Ok with compression_ratio 50.0, original_tokens 1000, compacted_tokens 500, turns_summarized 4, turns_kept 1
    When SlashCommandSelected(SlashCommandAction::Compact) is dispatched
    Then within 1 second backend.compact_session is called exactly once with session_id s-1
    And within 1 second s-1's scrollback contains a chunk whose text equals "[compaction] 50.0% reduction (1000 → 500 tokens, 4 turns summarised)"
    And s-2's scrollback does NOT contain any chunk whose text starts with "[compaction]"

  Scenario: SessionFooter renders the compaction progress segment when progress is Some
    Given an App with an open session s-1 whose compaction_progress is Some with phase "summarising messages", current 5, total 10
    When SessionFooter is rendered into an 80x1 buffer
    Then the rendered row contains the substring "[compacting: summarising messages 5/10]"
    And the rendered row contains the substring "▰▰▰▰▰▱▱▱▱▱"

  Scenario: SessionFooter omits the compaction segment when progress is None
    Given an App with an open session s-1 whose compaction_progress is None
    When SessionFooter is rendered into an 80x1 buffer
    Then the rendered row does NOT contain the substring "[compacting:"
    And the rendered row does NOT contain the substring "▰"

  Scenario: CompactionComplete chunk clears progress and emits a completion notice
    Given an App with an open session s-1 whose compaction_progress is Some with phase "summarising", current 3, total 10
    When ChunkReceived(s-1, StreamChunk::CompactionComplete) with compression_ratio 75.0, original_tokens 8000, compacted_tokens 2000, turns_summarized 6, turns_kept 2 is dispatched
    Then compaction_progress_for(s-1) becomes None
    And within 1 second s-1's scrollback contains a chunk whose text equals "[compaction] 75.0% reduction (8000 → 2000 tokens, 6 turns summarised)"
