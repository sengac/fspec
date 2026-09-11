# AST research: per-message markdown table finalization (RPC-432)

Scope: `rust/fspec-tui/src/store/agent_view` — the chunk-processing surface that
finalizes assistant messages in the Rust TUI AgentView.

## Findings

### Flush-trigger call sites of `flush_in_flight_drop_empty`

`flush_in_flight_drop_empty` (chunk_processor.rs:157) is the single per-message
finalizer. Call sites (via Grep on `flush_in_flight_drop_empty`):

| Call site | File:line | Chunk |
|---|---|---|
| UserInput arm | session_context.rs:85 | `StreamChunk::UserInput` |
| Interrupted arm | session_context.rs:113 | `StreamChunk::Interrupted` |
| `handle_error` | chunk_processor.rs:140 | `StreamChunk::Error` |
| `handle_tool_call` | chunk_tool_result.rs:28 | `StreamChunk::ToolCall` |

Note: `handle_tool_result` (chunk_tool_result.rs:51) does NOT call
`flush_in_flight_drop_empty` — it attaches the result body to the matching
tool-call card and pushes a fresh empty assistant placeholder. So the
assistant message before a ToolResult has already been flushed by the
corresponding ToolCall.

### `handle_done` (chunk_processor.rs:109)

```rust
pub fn handle_done(ctx: &mut SessionContext) {
    if let Some(idx) = ctx.in_flight_assistant {
        let is_empty = ...;
        if is_empty {
            ctx.scrollback.chunks_mut().remove(idx);
        } else if let Some(chunk) = ctx.scrollback.chunks_mut().get_mut(idx) {
            if let Some(source) = chunk.source.as_mut() {
                source.text = format_markdown_tables(&source.text); // <-- the only table-format call site
                source.is_streaming = false;
            }
            ctx.scrollback.rewrap_at(idx);
        }
        ctx.in_flight_assistant = None;
    }
    ctx.in_flight_thinking = None;
}
```

### `flush_in_flight_drop_empty` (chunk_processor.rs:157)

```rust
pub fn flush_in_flight_drop_empty(ctx: &mut SessionContext) {
    if let Some(idx) = ctx.in_flight_assistant.take() {
        let is_empty = ...;
        if is_empty {
            ctx.scrollback.chunks_mut().remove(idx);
        } else if let Some(chunk) = ctx.scrollback.chunks_mut().get_mut(idx) {
            if let Some(source) = chunk.source.as_mut() {
                source.is_streaming = false;   // <-- no format_markdown_tables here today
            }
            ctx.scrollback.rewrap_at(idx);
        }
    }
}
```

### The only `format_markdown_tables` call site

`chunk_processor.rs:122` inside `handle_done`. No other call site exists in
the crate (verified via Grep for `format_markdown_tables` in
rust/fspec-tui/src/store/agent_view).

## Conclusion

Moving `source.text = format_markdown_tables(&source.text);` from `handle_done`
into `flush_in_flight_drop_empty` (immediately before `source.is_streaming =
false;`) routes every flush trigger (ToolCall, Error, Interrupted, UserInput)
through the box-drawing grid. `handle_done` then reduces to the shared flush +
`ctx.in_flight_thinking = None`. No new call sites needed.

Idempotency: `format_markdown_tables` leaves box-drawing grid output unchanged
(the grid rows contain no `|...|` rows with a `---` separator row), so a
second pass on already-formatted text is a no-op. This is why routing every
finalization through one format call is safe even when Done follows a ToolCall.

## Existing tests that pin current behavior

- `rust/fspec-tui/tests/chunkprocessor_parity_rpc091.rs` — pins Done →
  formatMarkdownTables (scenario at line ~74 of the RPC-091 feature) and
  ToolCall flush (line 310). These tests use non-table prose in the
  flushed message, so they remain green after the move.
- `rust/fspec-tui/src/store/agent_view/markdown_tables_tests.rs` — unit tests
  for `format_markdown_tables` itself (unchanged).

## Test strategy for RPC-432

Add an integration test file `rust/fspec-tui/tests/per_message_table_finalization_rpc432.rs`
mirroring the 7 scenarios in
`spec/features/markdown-tables-per-assistant-message-finalization.feature`.
Each test drives `Action::ChunkReceived` through the AgentView store (same
harness as `chunkprocessor_parity_rpc091.rs`) and asserts on the chunk's
`source.text` after the flush trigger.
