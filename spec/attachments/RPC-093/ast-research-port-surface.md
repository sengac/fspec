# RPC-093 AST Research — Rust port surface

Conducted via `AstGrep` against `codelet/fspec-tui` + `Grep` /
`Read` against authoritative TS sources. Documents every code
surface that must change (or stay frozen) to land RPC-093 without
violating the RPC-024 / RPC-091 / RPC-078 conventions.

## 1. SessionContext (codelet/fspec-tui/src/store/agent_view/session_context.rs)

Existing surface (AstGrep `pub fn $NAME($$$ARGS) { $$$BODY }`):

| Line | Function                                                | Visibility   |
|------|---------------------------------------------------------|--------------|
| 47   | `new(id: SessionId) -> Self`                            | `pub`        |
| 58   | `with_work_unit(id, work_unit_id) -> Self`              | `pub`        |
| 68   | `record_chunk(&mut self, chunk: &StreamChunk)`          | `pub`        |
| 146  | `reset_scrollback(&mut self)`                           | `pub`        |
| 154  | `push_chunk(&mut self, source: ChunkSource)`            | `pub(crate)` |
| 160  | `push_source(&mut self, source: ChunkSource)`           | `pub(crate)` |

Field surface (struct lines 33–44):

- `id: SessionId`
- `work_unit_id: Option<String>`
- `scrollback: ScrollbackList`
- `scrollback_next_seq: u64`
- `input_draft: String`
- `in_flight_assistant: Option<usize>`  ← RPC-091

### Required additions (RPC-093):

- New field: `in_flight_thinking: Option<usize>` (default `None`,
  cleared by `reset_scrollback`).
- New `pub(crate)` helper: `insert_source_at(&mut self, idx: usize, source: ChunkSource)`
  — mirrors `push_source` but uses `ScrollbackList::insert` (new
  primitive on the data structure). Allocates the seq cursor and
  performs initial `wrap_source` against `DEFAULT_WRAP_WIDTH`.

### Required mutation: `record_chunk` arm `StreamChunk::Thinking`

Currently (lines 80–86):

```rust
StreamChunk::Thinking { thinking, .. } => {
    self.push_chunk(ChunkSource {
        text: format!("[Thinking]\n{thinking}"),
        color: Color::Yellow,
        kind: ChunkKind::Thinking,
        is_streaming: false,
    });
}
```

Becomes:

```rust
StreamChunk::Thinking { thinking, .. } => {
    append_thinking(self, thinking);
}
```

### Required slot-only clears (no chunk mutation):

- `StreamChunk::UserInput` arm: after `flush_in_flight_drop_empty(self)`,
  also `self.in_flight_thinking = None;`.
- `StreamChunk::Interrupted` arm: same as UserInput.
- `handle_done` (chunk_processor): clear `in_flight_thinking`.
- `handle_error` (chunk_processor): clear `in_flight_thinking`.

### `reset_scrollback` must also clear `in_flight_thinking`.

## 2. chunk_processor (codelet/fspec-tui/src/store/agent_view/chunk_processor.rs)

Existing function surface (AstGrep):

| Line | Function                                                  |
|------|-----------------------------------------------------------|
| 19   | `append_assistant_text(ctx, text)`                        |
| 42   | `handle_tool_call(ctx, info)`                             |
| 58   | `handle_tool_result(ctx, info)`                           |
| 96   | `handle_tool_progress(ctx, info)`                         |
| 123  | `handle_done(ctx)`                                        |
| 147  | `handle_error(ctx, error)`                                |
| 161  | `flush_in_flight_drop_empty(ctx)`                         |

### Required additions:

- `pub fn append_thinking(ctx: &mut SessionContext, delta: &str)` —
  mirrors `appendThinking` + `findActiveThinkingBlock`. If
  `in_flight_thinking.is_some()`, append delta to the chunk's
  `source.text` and `scrollback.rewrap_at(idx)`. Else create a new
  `ChunkKind::Thinking` chunk with `is_streaming: true`. If
  `in_flight_assistant.is_some()`, insert BEFORE that index via
  `ctx.insert_source_at(assist_idx, source)`; bump `in_flight_assistant`
  by 1; set `in_flight_thinking = Some(assist_idx)`. Else push to tail
  and set `in_flight_thinking = Some(new_idx)`.
- `pub fn finalize_in_flight_thinking(ctx: &mut SessionContext)` —
  if `in_flight_thinking.take().is_some()`, set the chunk's
  `is_streaming = false` and `rewrap_at(idx)`.

### Required wiring:

- `handle_tool_call`: call `finalize_in_flight_thinking(ctx)` BEFORE
  the existing `flush_in_flight_drop_empty(ctx)` and tool-card push.
- `handle_done`: at the end (after the assistant flush block), clear
  `ctx.in_flight_thinking = None;` WITHOUT mutating the chunk.
- `handle_error`: at the end, `ctx.in_flight_thinking = None;`.
- `flush_in_flight_drop_empty`: leave alone — UserInput / Interrupted
  arms in `record_chunk` will set the slot to None inline (Drop-empty
  semantics apply only to assistant text, not thinking).

## 3. ScrollbackList (codelet/fspec-tui/src/views/agent/scrollback.rs)

Existing surface (AstGrep):

| Line | Function                                            |
|------|-----------------------------------------------------|
| 60   | `push(&mut self, chunk: RenderedChunk)`             |
| 72   | `chunk_count() -> usize`                            |
| 77   | `chunks() -> &[RenderedChunk]`                      |
| 83   | `chunks_mut() -> &mut Vec<RenderedChunk>`           |
| 89   | `rewrap_at(&mut self, i: usize)`                    |
| ...  | viewport / scroll helpers                           |

### Required addition (per DeepSearch Q0 answer):

```rust
/// Insert a chunk at `idx`, shifting subsequent chunks right.
/// Re-wraps the inserted chunk if viewport width is known.
/// Mirrors [`Vec::insert`]. **RPC-093**.
pub fn insert(&mut self, idx: usize, chunk: RenderedChunk) {
    self.chunks.insert(idx, chunk);
    if self.viewport_width != 0 {
        if let Some(inserted) = self.chunks.get_mut(idx) {
            rewrap_chunk(inserted, self.viewport_width);
        }
    }
    if self.scroll_state.stick_to_bottom {
        self.recompute_offset_for_stick();
    }
}
```

## 4. Render layer (wrap_source in session_context.rs lines 175–212)

`ChunkKind::Thinking` currently maps to prefix `""` (line 184–188).

### Required change:

Map `ChunkKind::Thinking` to a per-variant render that prepends
`"[Thinking]\n"` BEFORE the body when wrapping — so a chunk whose
stored `source.text` is `"first line\nsecond line"` renders as:

```
[Thinking]
first line
second line
```

This must compose with the existing `wrap_to_width` per-paragraph
loop. Suggested approach: split `prefix_lines` (containing
`"[Thinking]"`) and emit them as their own `Line`s before iterating
`hard_lines` from `source.text`.

## 5. rendered_chunk.rs (codelet/fspec-tui/src/views/agent/rendered_chunk.rs)

No changes required — `ChunkKind::Thinking` enum variant already
exists with the comment "Yellow `[Thinking]\n…` block; no `● ` prefix".
The prefix policy update lives entirely in `wrap_source`.

## 6. ts-port source of truth (frozen reference)

- `src/tui/utils/thinkingBlockManager.ts:36` — `THINKING_PREFIX = '[Thinking]\n'`
- `src/tui/utils/thinkingBlockManager.ts:58-79` — `findActiveThinkingBlock`
- `src/tui/utils/thinkingBlockManager.ts:139-181` — `appendThinking`
- `src/tui/utils/thinkingBlockManager.ts:194-207` — `finalizeThinkingBlock`
- `src/tui/utils/chunkProcessor.ts:463-466` — Thinking branch
- `src/tui/utils/chunkProcessor.ts:469` — ToolCall finalize (only
  explicit `finalizeThinkingBlock` call site)

## 7. Test plumbing (already in repo)

`codelet/fspec-tui/tests/chunkprocessor_parity_rpc091.rs` documents
the assertion-style we must follow. RPC-093 test file will live at:

```
codelet/fspec-tui/tests/thinking_streaming_parity_rpc093.rs
```

Uses the same `MockBackend` + `App::dispatch(Action::ChunkReceived(...))`
plumbing as RPC-091. Helpers `session_chunk_count`, `session_lines`,
`nth_chunk_source_text` are duplicated locally (or extracted to
`tests/common/` if both files share them).
