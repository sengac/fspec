@done
@rust
@tui
@agent-view
@scrollback
@tui-component
@RPC-093
Feature: AgentView Thinking streaming accumulation parity (port appendThinking + finalizeThinkingBlock to Rust SessionContext) — fix [Thinking] per-delta repetition
  """
  TS source of truth: src/tui/utils/thinkingBlockManager.ts (appendThinking + findActiveThinkingBlock + finalizeThinkingBlock) and src/tui/utils/chunkProcessor.ts:463-466 (Thinking branch) + 469 (ToolCall finalize)
  Rust touch points: rust/fspec-tui/src/store/agent_view/session_context.rs (add in_flight_thinking, replace Thinking arm), chunk_processor.rs (add append_thinking + finalize_in_flight_thinking, wire from handle_tool_call), views/agent/scrollback.rs (move [Thinking]\n prefix to render-time), views/agent.rs ScrollbackList (add insert_chunk_at helper)
  Architecture: prefix moves render-side (parity with RPC-091 '● ' decision); source.text stays as raw opaque model output; in_flight_thinking is the Option<usize> equivalent of TS findActiveThinkingBlock's 'last streaming thinking with no UserInput after it'
  No batching/throttling layer needed: TS PERF-003 deliberately removed useDeferredValue because Ink uses LegacyRoot (synchronous). Rust ratatui has its own draw loop; rewrap_at(idx) per chunk mutation is sufficient — the renderer coalesces frames
  OUT of scope (separate cards if needed): streaming '...' indicator parity (deferred per RPC-091 note), appendThinkingBulk hydration (Rust doesn't rehydrate yet), markdown formatting inside thinking blocks (TS also doesn't)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Consecutive StreamChunk::Thinking deltas for the same logical thought MUST accumulate into a single RenderedChunk (parity with TS appendThinking)
  #   2. The '[Thinking]\n' prefix MUST live in the render layer (ChunkKind::Thinking) — NOT baked into source.text per delta
  #   3. SessionContext.in_flight_thinking: Option<usize> tracks the currently-streaming thinking chunk index (analogue of TS findActiveThinkingBlock)
  #   4. Only handle_tool_call calls finalize_in_flight_thinking (sets is_streaming=false). Done/Error/UserInput/Interrupted only CLEAR the slot — they never mutate the existing thinking chunk
  #   5. When a new thinking block is created while in_flight_assistant is Some, the new chunk MUST be inserted BEFORE the in-flight assistant (mirrors TS splice-before-streaming-assistant); in_flight_assistant index is bumped by 1
  #   6. No throttling, batching, or debouncing at the conversation layer — one chunk → one mutation → rewrap_at(idx), matching TS PERF-003 (useDeferredValue was deliberately removed)
  #
  # EXAMPLES:
  #   1. Replaying the 2026-05-29 screenshot transcript (one logical thought split across 4 Thinking deltas) produces exactly ONE scrollback row, not four
  #   2. The user watches the agent think then call a tool then think again — they see TWO separate yellow [Thinking] blocks on screen with the tool card visually between them
  #   3. The user sees the agent thinking, types a new message, and the agent thinks again — the two thought blocks remain visually distinct (two yellow [Thinking] headers) with the user's message between them
  #   4. While the agent is streaming an assistant text response and a thinking delta arrives, the user sees the [Thinking] block appear ABOVE the in-progress assistant text — not below or interleaved into it
  #   5. After the agent finishes a turn (Done), an earlier thought stays visible on screen unchanged; when a brand new turn starts and the agent thinks again, the user sees a NEW [Thinking] block — the new thoughts do not merge into the old one
  #   6. The user watches three rapid thinking deltas ('The user', ' is asking', ' about cards') arrive — they see ONE yellow [Thinking] block with the combined text 'The user is asking about cards', not three separate [Thinking] headers
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should ScrollbackList::insert_chunk_at(idx, source) live on ScrollbackList itself (mirrors Vec::insert) or on SessionContext (auto-handles scrollback_next_seq + rewrap_at)? Lean: ScrollbackList for the primitive + SessionContext::insert_source_at wrapper.
  #   A: Two-layer: ScrollbackList::insert(idx, RenderedChunk) is a Vec-shaped primitive (mirrors push, re-wraps on insert, recomputes stick-to-bottom — same encapsulation as push). SessionContext::insert_source_at(idx, ChunkSource) is the wrapper that allocates seq from scrollback_next_seq, does initial wrap_source, and delegates. Matches the matched-primitive convention RPC-091 established (push/push_source, chunks_mut().remove + handle_done, rewrap_at + in-place mutate). Keeps both files under the RPC-024 300-LoC ceiling. chunk_processor::append_thinking calls ctx.insert_source_at(idx, source) and bumps in_flight_assistant by one.
  #
  # ========================================
  Background: User Story
    As a fspec TUI user watching streaming thinking
    I want to see consecutive Thinking deltas accumulate into one yellow [Thinking] block
    So that the scrollback shows readable thoughts instead of one [Thinking] header per token

  Scenario: Consecutive Thinking deltas accumulate into a single in-flight thinking chunk
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Thinking { thinking: "The user" } for s-1
    And the chunks subscriber forwards StreamChunk::Thinking { thinking: " is asking" } for s-1
    And the chunks subscriber forwards StreamChunk::Thinking { thinking: " about cards" } for s-1
    Then the s-1 scrollback contains exactly one rendered chunk
    And that chunk's source.text equals "The user is asking about cards" without any "[Thinking]" prefix baked in
    And the SessionContext in_flight_thinking slot is Some(<that chunk's index>)
    And the chunk's source.kind is ChunkKind::Thinking
    And the chunk's is_streaming flag is true

  Scenario: The "[Thinking]\n" prefix is applied by the renderer only on lineIndex==0 of the first wrapped line
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Thinking { thinking: "first line\nsecond line" } for s-1
    Then the wrapped lines produced for that chunk are exactly:
      | line index | visible text |
      | 0          | [Thinking]   |
      | 1          | first line   |
      | 2          | second line  |
    And the stored chunk.source.text is exactly "first line\nsecond line" (no prefix baked in)

  Scenario: ToolCall finalises the in-flight thinking chunk and pushes a tool-call card after it
    Given an AgentView with a fresh SessionContext for session s-1
    And the chunks subscriber has forwarded StreamChunk::Thinking { thinking: "Let me check the files" } for s-1
    When the chunks subscriber forwards StreamChunk::ToolCall { tool_call: { id: "tc-1", name: "Read", input: "{\"file_path\":\"/etc/hosts\"}" } } for s-1
    Then the s-1 scrollback contains exactly two rendered chunks in order:
      | index | kind     | visible first line |
      | 0     | Thinking | [Thinking]         |
      | 1     | ToolCall | ● Read(/etc/hosts) |
    And the Thinking chunk at index 0 has is_streaming false
    And the SessionContext in_flight_thinking slot is None

  Scenario: A second Thinking delta after a ToolCall starts a fresh thinking chunk
    Given an AgentView with a fresh SessionContext for session s-1
    And the chunks subscriber has forwarded StreamChunk::Thinking { thinking: "first thought" } for s-1
    And the chunks subscriber has forwarded StreamChunk::ToolCall { tool_call: { id: "tc-1", name: "Read", input: "{\"file_path\":\"/etc/hosts\"}" } } for s-1
    When the chunks subscriber forwards StreamChunk::Thinking { thinking: "second thought" } for s-1
    Then the s-1 scrollback contains exactly three rendered chunks in order:
      | index | kind     | source.text      |
      | 0     | Thinking | first thought    |
      | 1     | ToolCall | Read(/etc/hosts) |
      | 2     | Thinking | second thought   |
    And the chunk at index 0 has is_streaming false
    And the chunk at index 2 has is_streaming true
    And the SessionContext in_flight_thinking slot is Some(2)

  Scenario: UserInput is a turn boundary that clears the in-flight thinking slot
    Given an AgentView with a fresh SessionContext for session s-1
    And the chunks subscriber has forwarded StreamChunk::Thinking { thinking: "first thought" } for s-1
    And the chunks subscriber has forwarded StreamChunk::UserInput { text: "carry on" } for s-1
    When the chunks subscriber forwards StreamChunk::Thinking { thinking: "second thought" } for s-1
    Then the s-1 scrollback contains exactly three rendered chunks in order:
      | index | kind      | source.text    |
      | 0     | Thinking  | first thought  |
      | 1     | UserInput | carry on       |
      | 2     | Thinking  | second thought |
    And the chunk at index 0 has is_streaming true (left untouched by UserInput)
    And the chunk at index 2 has is_streaming true
    And the SessionContext in_flight_thinking slot is Some(2)

  Scenario: A Thinking delta arriving while in_flight_assistant is Some inserts the thinking chunk BEFORE the assistant chunk
    Given an AgentView with a fresh SessionContext for session s-1
    And the chunks subscriber has forwarded StreamChunk::Text { text: "Reading" } for s-1
    And the SessionContext in_flight_assistant slot is Some(0)
    When the chunks subscriber forwards StreamChunk::Thinking { thinking: "Hmm" } for s-1
    Then the s-1 scrollback contains exactly two rendered chunks in order:
      | index | kind          | source.text |
      | 0     | Thinking      | Hmm         |
      | 1     | AssistantText | Reading     |
    And the SessionContext in_flight_thinking slot is Some(0)
    And the SessionContext in_flight_assistant slot is Some(1)

  Scenario: Done clears the in_flight_thinking slot without mutating the existing thinking chunk
    Given an AgentView with a fresh SessionContext for session s-1
    And the chunks subscriber has forwarded StreamChunk::Thinking { thinking: "settled thought" } for s-1
    When the chunks subscriber forwards StreamChunk::Done for s-1
    Then the s-1 scrollback contains exactly one rendered chunk
    And that chunk's source.text equals "settled thought"
    And that chunk's is_streaming flag is true (Done does not mutate the thinking chunk)
    And the SessionContext in_flight_thinking slot is None

  Scenario: A second Thinking delta after Done starts a fresh thinking chunk (turn boundary)
    Given an AgentView with a fresh SessionContext for session s-1
    And the chunks subscriber has forwarded StreamChunk::Thinking { thinking: "old thought" } for s-1
    And the chunks subscriber has forwarded StreamChunk::Done for s-1
    When the chunks subscriber forwards StreamChunk::Thinking { thinking: "new thought" } for s-1
    Then the s-1 scrollback contains exactly two rendered chunks in order:
      | index | kind     | source.text |
      | 0     | Thinking | old thought |
      | 1     | Thinking | new thought |
    And the SessionContext in_flight_thinking slot is Some(1)

  Scenario: Error clears the in_flight_thinking slot without mutating the existing thinking chunk
    Given an AgentView with a fresh SessionContext for session s-1
    And the chunks subscriber has forwarded StreamChunk::Thinking { thinking: "mid thought" } for s-1
    When the chunks subscriber forwards StreamChunk::Error { error: "rate limit exceeded" } for s-1
    Then the s-1 scrollback contains exactly two rendered chunks in order:
      | index | kind     | source.text                    |
      | 0     | Thinking | mid thought                    |
      | 1     | Error    | API Error: rate limit exceeded |
    And the chunk at index 0 has is_streaming true (Error does not mutate it)
    And the SessionContext in_flight_thinking slot is None

  Scenario: Interrupted clears the in_flight_thinking slot without mutating the existing thinking chunk
    Given an AgentView with a fresh SessionContext for session s-1
    And the chunks subscriber has forwarded StreamChunk::Thinking { thinking: "interrupted mid-flight" } for s-1
    When the chunks subscriber forwards StreamChunk::Interrupted {} for s-1
    Then the s-1 scrollback contains exactly two rendered chunks in order:
      | index | kind        | source.text            |
      | 0     | Thinking    | interrupted mid-flight |
      | 1     | Interrupted | ⚠ Interrupted          |
    And the chunk at index 0 has is_streaming true (Interrupted does not mutate it)
    And the SessionContext in_flight_thinking slot is None

  Scenario: Screenshot transcript - four deltas of one logical thought produce one scrollback row
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards the following Thinking deltas in order for s-1:
      | thinking                                                  |
      | The user is asking about                                  |
      | how well a "card" was done. This is likely referring to a |
      | work unit in fspec. Let me check the board to see what    |
      | work units are in progress or recently completed.         |
    Then the s-1 scrollback contains exactly one rendered chunk
    And the rendered first wrapped line of that chunk is "[Thinking]"
    And that chunk's source.text concatenates all four deltas with no "[Thinking]" prefix baked in
    And the SessionContext in_flight_thinking slot is Some(0)

  Scenario: Full round-trip - user asks, agent thinks, calls tool, thinks again, answers, finishes
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards the following chunks in order for s-1:
      | chunk                                                                                            |
      | UserInput { text: "how is RPC-093 going?" }                                                      |
      | Thinking { thinking: "Let me " }                                                                 |
      | Thinking { thinking: "check the card" }                                                          |
      | ToolCall { tool_call: { id: "tc-1", name: "Fspec", input: "{\"command\":\"show-work-unit\"}" } } |
      | ToolResult { tool_result: { tool_call_id: "tc-1", content: "ok", is_error: false } }             |
      | Thinking { thinking: "It is in " }                                                               |
      | Thinking { thinking: "specifying" }                                                              |
      | Text { text: "RPC-093 is in specifying." }                                                       |
      | Done                                                                                             |
    Then the s-1 scrollback contains exactly five rendered chunks in order:
      | index | kind          | source.text                 |
      | 0     | UserInput     | how is RPC-093 going?       |
      | 1     | Thinking      | Let me check the card       |
      | 2     | ToolCall      | Fspec(show-work-unit)<NL>ok |
      | 3     | Thinking      | It is in specifying         |
      | 4     | AssistantText | RPC-093 is in specifying.   |
    And the chunk at index 1 has is_streaming false (finalised by the ToolCall at index 2)
    And the chunk at index 3 has is_streaming true (Done does not mutate it)
    And the chunk at index 4 has is_streaming false (Done finalised the assistant text)
    And the SessionContext in_flight_thinking slot is None
    And the SessionContext in_flight_assistant slot is None
    And no chunk has "[Thinking]" baked into its stored source.text
