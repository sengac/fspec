@done
@tui-component
@scrollback
@agent-view
@tui
@rust
@RPC-091
Feature: AgentView Rust port: streaming Text accumulation + ToolCall/ToolResult/ToolProgress + Done finalisation rendering parity with TS Ink chunkProcessor
  """
  Add in_flight_assistant: Option<usize> to SessionContext — index into scrollback.chunks pointing at the current accumulating assistant bubble. Cleared by Done/ToolCall/Error/Interrupted (flush triggers).
  Move '● ' out of chunk_to_message and into wrap_source (or ScrollbackList::render) — apply only on lineIndex==0 for chunks tagged ChunkKind::AssistantText. Add a ChunkKind enum to ChunkSource so the renderer knows which prefix to use.
  Port chunkProcessor.ts Text branch (lines 444-461): when in_flight_assistant.is_some(), mutate scrollback.chunks[i].source.text via append and re-wrap that one chunk; else push new chunk with ChunkKind::AssistantText and record its index.
  Port chunkProcessor.ts ToolCall branch (lines 468-505): drop empty in-flight, finalise non-empty in-flight (clear in_flight_assistant index), parse args via extractToolArgsDisplay equivalent, push ChunkKind::ToolCall with tool_call_id.
  Port chunkProcessor.ts ToolResult branch (lines 507-536): walk back to matching tool-call card, append result body + isError, push fresh empty in-flight assistant-text placeholder.
  Port chunkProcessor.ts Done branch (lines 538-558): pop trailing empty in-flight; if still in-flight, run formatMarkdownTables on accumulated text and clear isStreaming/in_flight_assistant.
  Port extractToolArgsDisplay from src/tui/utils/toolFormatters.ts — per-tool argument summary (Bash→command, Read→file_path, Write→file_path, Edit→file_path, Grep→pattern, Glob→pattern, Fspec→command, default→first JSON value).
  Markdown rendering (table formatting via pulldown-cmark or simple TS-port of formatMarkdownTables) is in scope only for Done finalisation parity — broader markdown→ratatui span rendering is OUT OF SCOPE (separate work unit).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Consecutive StreamChunk::Text deltas for the same session MUST accumulate into a single in-flight assistant-text RenderedChunk (append to chunk.source.text + re-wrap), not push a new RenderedChunk per delta
  #   2. The bullet glyph '● ' MUST be applied by the renderer only on lineIndex==0 of the first wrapped line of an assistant-text chunk — never baked into the stored chunk.source.text
  #   3. StreamChunk::Done MUST clear the in-flight assistant slot (no more appending) and run formatMarkdownTables on the accumulated assistant content — Done itself emits no new scrollback line
  #   4. StreamChunk::ToolCall MUST act as an implicit flush: finalise any in-flight assistant-text (or drop it if empty), then push a tool-call card whose visible text is the formatted header '● {toolName}({argsDisplay})'
  #   5. StreamChunk::ToolResult MUST attach the collapsed result body to the most recent tool-call card (matching by tool_call_id), set isError from the result, then push a fresh isStreaming assistant-text placeholder so the next Text delta starts a new bubble
  #   6. StreamChunk::ToolProgress MUST render under the matching tool-call card with a streaming-window suffix (stderr-style marker) and MUST NOT push a new top-level scrollback chunk
  #   7. StreamChunk::Error MUST drop the trailing empty isStreaming placeholder (if any) before pushing the 'API Error: {error}' line — mirroring the TS error-path pop+push semantics
  #   8. Tool argument display MUST be derived via the TS-equivalent extractToolArgsDisplay rules (per-tool inputs collapsed to a one-line summary), not the raw JSON input
  #
  # EXAMPLES:
  #   1. User types 'hi' → Anthropic streams 'Hello' then ' world' then Done → scrollback shows EXACTLY 'You: hi' and '● Hello world' (two chunks total, not three)
  #   2. User asks for fspec board → assistant streams 'Let me ' 'check the ' 'board' → ToolCall(Fspec, args={command:'board'}) → ToolResult(success, content='...') → assistant streams 'Here are the ' 'work units' → Done → scrollback shows: You/ assistant-bubble ('● Let me check the board')/ tool-call card with result body attached / next assistant-bubble ('● Here are the work units')
  #   3. Assistant streams 'one' then 'two' then ToolCall finalises with empty content (i.e. only got '') — the empty in-flight placeholder is dropped, not left as an empty '● ' row
  #   4. Done arrives after 'hello' with a markdown table in content → accumulated text is run through formatMarkdownTables before isStreaming clears, so the rendered chunk shows the aligned-pipe table
  #   5. ToolCall with name='Bash' input='{"command":"ls"}' renders header '● Bash(ls)' (extractToolArgsDisplay collapses to command), not '● Bash({"command":"ls"})'
  #   6. ToolResult with isError=true attaches to the matching tool-call header and the rendered chunk carries the isError flag (renderer colours the body red)
  #
  # ========================================
  Background: User Story
    As a user typing into a Rust TUI session
    I want to see each StreamChunk variant render with the same accumulation, prefixes, and flush semantics as the TypeScript Ink chunkProcessor
    So that the scrollback shows a single coherent assistant bubble per turn (not one bullet per delta), tool calls/results appear as cards, and Done finalises markdown — byte-for-byte parity with the Ink reference

  Scenario: Consecutive Text deltas accumulate into a single in-flight assistant chunk
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Text { text: "Hello" } for s-1
    And the chunks subscriber forwards StreamChunk::Text { text: " world" } for s-1
    Then the s-1 scrollback contains exactly one rendered chunk
    And that chunk's source.text equals "Hello world" without any bullet glyph
    And the SessionContext in_flight_assistant slot is Some(<that chunk's index>)
    And the chunk's source.kind is ChunkKind::AssistantText

  Scenario: Bullet glyph is applied by the renderer only on lineIndex==0 of the first wrapped line
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Text { text: "first line\nsecond line" } for s-1
    Then the wrapped lines produced for that chunk are exactly:
      | line index | visible text |
      | 0          | ● first line |
      | 1          | second line  |
    And the stored chunk.source.text is exactly "first line\nsecond line" (no bullet baked in)

  Scenario: Done flushes the in-flight assistant slot and emits no new chunk
    Given an AgentView with a fresh SessionContext for session s-1
    And the chunks subscriber has forwarded StreamChunk::Text { text: "hello" } for s-1
    When the chunks subscriber forwards StreamChunk::Done for s-1
    Then the s-1 scrollback still contains exactly one rendered chunk
    And the SessionContext in_flight_assistant slot is None
    And the chunk's is_streaming flag is false

  Scenario: Done runs formatMarkdownTables over the accumulated assistant text
    Given an AgentView with a fresh SessionContext for session s-1
    And the chunks subscriber has forwarded StreamChunk::Text deltas concatenating to "| col1 | col2 |\n|---|---|\n| a | bb |" for s-1
    When the chunks subscriber forwards StreamChunk::Done for s-1
    Then the final in-flight chunk's source.text equals a pipe-aligned table where every row uses equal column widths
    And the chunk's is_streaming flag is false

  Scenario: ToolCall flushes the in-flight assistant text and pushes a tool-call card
    Given an AgentView with a fresh SessionContext for session s-1
    And the chunks subscriber has forwarded StreamChunk::Text { text: "Let me check the board" } for s-1
    When the chunks subscriber forwards StreamChunk::ToolCall { tool_call: { id: "tc-1", name: "Fspec", input: "{\"command\":\"board\"}" } } for s-1
    Then the s-1 scrollback contains exactly two rendered chunks in order:
      | index | kind          | visible text (after render) |
      | 0     | AssistantText | ● Let me check the board    |
      | 1     | ToolCall      | ● Fspec(board)              |
    And the SessionContext in_flight_assistant slot is None
    And the tool-call chunk's tool_call_id equals "tc-1"

  Scenario: ToolCall drops an empty in-flight assistant placeholder instead of finalising it
    Given an AgentView with a fresh SessionContext for session s-1
    And the SessionContext in_flight_assistant slot points at an existing empty AssistantText chunk
    When the chunks subscriber forwards StreamChunk::ToolCall { tool_call: { id: "tc-2", name: "Bash", input: "{\"command\":\"ls\"}" } } for s-1
    Then the empty AssistantText chunk has been removed from scrollback
    And the s-1 scrollback contains exactly one rendered chunk of kind ToolCall

  Scenario: ToolResult attaches to the matching tool-call header and pushes a fresh placeholder
    Given an AgentView with a fresh SessionContext for session s-1
    And the scrollback contains a ToolCall chunk with tool_call_id "tc-1" and header "● Fspec(board)"
    When the chunks subscriber forwards StreamChunk::ToolResult { tool_result: { tool_call_id: "tc-1", content: "AUTH-001  AUTH-002  AUTH-003", is_error: false } } for s-1
    Then the matching ToolCall chunk's source.text equals "● Fspec(board)\nAUTH-001  AUTH-002  AUTH-003"
    And the matching ToolCall chunk's is_error flag is false
    And the s-1 scrollback ends with a fresh empty AssistantText chunk with is_streaming true
    And the SessionContext in_flight_assistant slot is Some(<index of that fresh placeholder>)

  Scenario: ToolResult with is_error true colours the body via the isError flag
    Given an AgentView with a fresh SessionContext for session s-1
    And the scrollback contains a ToolCall chunk with tool_call_id "tc-3" and header "● Bash(false)"
    When the chunks subscriber forwards StreamChunk::ToolResult { tool_result: { tool_call_id: "tc-3", content: "exit code 1", is_error: true } } for s-1
    Then the matching ToolCall chunk's is_error flag is true
    And the rendered lines for that chunk carry foreground colour RED on the result body

  Scenario: Continuation Text after ToolResult starts a new AssistantText bubble
    Given an AgentView with a fresh SessionContext for session s-1
    And the scrollback ends with a fresh empty AssistantText placeholder created by a prior ToolResult
    When the chunks subscriber forwards StreamChunk::Text { text: "Here are the " } for s-1
    And the chunks subscriber forwards StreamChunk::Text { text: "work units" } for s-1
    Then the trailing AssistantText chunk's source.text equals "Here are the work units"
    And the SessionContext in_flight_assistant slot points at that trailing chunk

  Scenario: ToolProgress is folded under the matching tool-call card and does not push a new top-level chunk
    Given an AgentView with a fresh SessionContext for session s-1
    And the scrollback contains a ToolCall chunk with tool_call_id "tc-4" and header "● Bash(npm test)"
    When the chunks subscriber forwards StreamChunk::ToolProgress { tool_call_id: "tc-4", chunk: "PASS src/foo.test.ts\n", stream: stderr } for s-1
    Then the matching ToolCall chunk's source.text ends with "\nPASS src/foo.test.ts" within a streaming window
    And no new top-level RenderedChunk has been appended to the scrollback

  Scenario: Error drops a trailing empty in-flight placeholder before pushing the API Error line
    Given an AgentView with a fresh SessionContext for session s-1
    And the SessionContext in_flight_assistant slot points at an existing empty AssistantText chunk
    When the chunks subscriber forwards StreamChunk::Error { error: "rate limit exceeded" } for s-1
    Then the empty AssistantText chunk has been removed from scrollback
    And the s-1 scrollback ends with a chunk whose source.text equals "API Error: rate limit exceeded"
    And the SessionContext in_flight_assistant slot is None

  Scenario: extractToolArgsDisplay collapses Bash command to a one-line summary
    Given a ToolCall with name "Bash" and input JSON "{\"command\":\"ls -la\",\"timeout\":5000}"
    When the renderer formats the tool-call header
    Then the header text equals "● Bash(ls -la)"

  Scenario: extractToolArgsDisplay collapses Fspec command to the command subcommand
    Given a ToolCall with name "Fspec" and input JSON "{\"command\":\"show-work-unit\",\"args\":\"{\\\"_\\\":[\\\"AUTH-001\\\"]}\"}"
    When the renderer formats the tool-call header
    Then the header text equals "● Fspec(show-work-unit)"

  Scenario: Full round-trip: user asks, assistant thinks, calls a tool, continues, finishes
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards the following chunks in order for s-1:
      | chunk                                                                                   |
      | UserInput { text: "what cards are open?" }                                              |
      | Text { text: "Let me " }                                                                |
      | Text { text: "check the board" }                                                        |
      | ToolCall { tool_call: { id: "tc-1", name: "Fspec", input: "{\"command\":\"board\"}" } } |
      | ToolResult { tool_result: { tool_call_id: "tc-1", content: "ok", is_error: false } }    |
      | Text { text: "Here are the " }                                                          |
      | Text { text: "open work units" }                                                        |
      | Done                                                                                    |
    Then the s-1 scrollback contains exactly four rendered chunks in order:
      | index | visible text (after render)    |
      | 0     | You: what cards are open?      |
      | 1     | ● Let me check the board       |
      | 2     | ● Fspec(board) ok             |
      | 3     | ● Here are the open work units |
    And the SessionContext in_flight_assistant slot is None
    And no chunk has a bullet baked into its stored source.text
