# AST Research: chunkProcessor Parity (RPC-091)

Conducted via the `AstGrep` tool against the Rust TUI + manual `Read`
against the TS Ink reference. Captures every code surface that must
change to satisfy the RPC-091 acceptance criteria.

---

## 1. Rust surfaces to modify

### `codelet/fspec-tui/src/store/agent_view/session_context.rs`

| Function / Item                                          | Line | Change needed |
|----------------------------------------------------------|------|---------------|
| `pub fn record_chunk(&mut self, chunk: &StreamChunk)`    | 89   | **REWRITE.** Currently pushes a fresh `RenderedChunk` per chunk. Must consult/maintain `SessionContext.in_flight_assistant: Option<usize>`, branch on chunk variant to accumulate Text into the in-flight slot, flush on Done/ToolCall/Error/Interrupted, and run formatMarkdownTables on the in-flight accumulated text at Done. |
| `pub struct SessionContext`                              | 33   | **ADD FIELD.** `in_flight_assistant: Option<usize>` — index into `scrollback.chunks` pointing at the currently-accumulating AssistantText. Reset to `None` in `new`, `with_work_unit`, and `reset_scrollback`. |
| `fn chunk_to_message(chunk: &StreamChunk) -> Option<ChunkSource>` | 155 | **REWRITE.** Stop baking `"● "` into `Text` arm output. Tag returned ChunkSource with a `kind: ChunkKind` field so the renderer can apply the bullet only on `lineIndex == 0` of the first wrapped line. Replace `ToolCall/ToolResult/ToolProgress => None` arms with proper card formatting. |

### `codelet/fspec-tui/src/views/agent/rendered_chunk.rs`

| Item                  | Line | Change needed |
|-----------------------|------|---------------|
| `pub struct ChunkSource` | 19 | **ADD FIELDS.** `kind: ChunkKind` (enum: `UserInput`, `AssistantText`, `Thinking`, `ToolCall { tool_call_id: String, is_error: bool }`, `Error`, `Notification`, `Incoming`, `Interrupted`). `is_streaming: bool` (defaults false; true for in-flight AssistantText / ToolResult placeholder). |
| `pub struct RenderedChunk` | 38 | No structural change — `source.kind` carries the discrimination. |

### `codelet/fspec-tui/src/store/agent_view/session_context.rs::wrap_source`

| Function                                                                       | Line | Change needed |
|--------------------------------------------------------------------------------|------|---------------|
| `pub(crate) fn wrap_source(source: &ChunkSource, width: u16) -> Vec<Line<'static>>` | 133  | **REWRITE.** Apply the `"● "` prefix only when `source.kind == ChunkKind::AssistantText | ChunkKind::ToolCall { .. }` AND `lineIndex == 0` of the first hard-line. For ToolCall, render the header line + an indented body (if any) per the TS `<ToolCallMessage>` layout. For `is_streaming` true AssistantText, append `"..."` to the last wrapped line (TS `conversationUtils.ts:88-90`). |

---

## 2. New Rust helpers required

### `codelet/fspec-tui/src/store/agent_view/tool_args.rs` (NEW)

```rust
pub fn extract_tool_args_display(tool_name: &str, input_json: &str) -> String;
```

- Mirrors `src/tui/utils/toolFormatters.ts` `extractToolArgsDisplay`.
- Per-tool dispatch: `Bash → command`, `Read/Write/Edit → file_path`,
  `Grep → pattern`, `Glob → pattern`, `Fspec → command`, default →
  first JSON value rendered compactly. Falls back to the raw input on
  parse failure.

### `codelet/fspec-tui/src/store/agent_view/markdown_tables.rs` (NEW)

```rust
pub fn format_markdown_tables(input: &str) -> String;
```

- Port of `src/tui/utils/formatMarkdownTables.ts`.
- Detects `|...|` table rows + the `|---|` separator. Aligns columns
  by padding cells to the max column width. Leaves non-table lines
  unchanged.

---

## 3. TS reference paths

| Concern                         | TS file                                   | Lines    |
|---------------------------------|-------------------------------------------|----------|
| Text accumulation               | `src/tui/utils/chunkProcessor.ts`         | 444-461  |
| Thinking append                 | `src/tui/utils/chunkProcessor.ts`         | 463-466  |
| ToolCall flush+push             | `src/tui/utils/chunkProcessor.ts`         | 468-505  |
| ToolResult attach+placeholder   | `src/tui/utils/chunkProcessor.ts`         | 507-536  |
| Done finalise+formatMarkdown    | `src/tui/utils/chunkProcessor.ts`         | 538-558  |
| Error pop-empty+push            | `src/tui/utils/chunkProcessor.ts`         | 560-580  |
| Bullet placement (lineIndex==0) | `src/tui/utils/conversationUtils.ts`      | 64-71,84 |
| Streaming "..." suffix          | `src/tui/utils/conversationUtils.ts`      | 87-90    |
| Tool args display               | `src/tui/utils/toolFormatters.ts`         | `extractToolArgsDisplay` |
| Table alignment                 | `src/tui/utils/formatMarkdownTables.ts`   | full     |

---

## 4. Existing Rust call sites that consume `record_chunk`

```
codelet/fspec-tui/src/store/agent_view/session_context.rs:89    (definition)
codelet/fspec-tui/src/store/agent_view.rs                       (Action::ChunkReceived → ctx.record_chunk(chunk))
codelet/fspec-tui/src/views/agent.rs                            (legacy push_line path — unchanged)
```

No external NAPI/wire callers — `record_chunk` is purely UI-side.

---

## 5. Affected tests

| Test file                                                                | Status |
|--------------------------------------------------------------------------|--------|
| `codelet/fspec-tui/src/store/agent_view/session_context.rs` (`#[cfg(test)] mod tests`) | EXISTING — `record_chunk_appends_and_bumps_seq` will need to be updated for accumulation semantics. |
| `codelet/fspec-tui/src/store/agent_view/__tests__/agentview-chunk-rendering-parity.rs` (or equivalent) | EXISTING for RPC-078 — keep, supplement with new RPC-091 cases for accumulation/ToolCall/ToolResult/Done. |
| `codelet/fspec-tui/src/store/agent_view/__tests__/chunkprocessor-parity.rs` | NEW — primary RPC-091 test surface. One test per scenario in `spec/features/agentview-chunkprocessor-parity.feature`. |

---

## 6. Dependency-rule check

All changes stay inside `codelet/fspec-tui` (the TUI crate). No
modifications to `codelet/sessions`, `codelet/agent-loop`,
`codelet/napi`, `codelet/rpc-types`, or any wire surface. New
`tool_args` + `markdown_tables` modules live in
`codelet/fspec-tui/src/store/agent_view/` to satisfy the 300-LoC
ceiling and keep them local to the only crate that uses them.
