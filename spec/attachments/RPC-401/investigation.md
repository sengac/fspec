# RPC-401 — AgentView message line-spacing parity

## Problem

The Rust ratatui `AgentView` scrollback renders consecutive messages
(user, assistant, tool cards) **back-to-back with zero blank lines**.
The TypeScript reference TUI renders **exactly one blank line between
every pair of messages**. The Rust view therefore looks visually
cramped and diverges from the reference.

## Root cause (confirmed via DeepSearch + Read)

### TypeScript reference — emits a per-message separator

`src/tui/utils/conversationUtils.ts` → `wrapMessageToLines()` (lines 50–130):

```ts
export const wrapMessageToLines = (
  msg, msgIndex, maxWidth,
  addSeparator: boolean = true      // <-- DEFAULT true
): ConversationLine[] => {
  ...
  // Add separator line after message for visual grouping (TUI-042)
  if (addSeparator) {                // lines 117-127
    lines.push({ role, content: ' ', messageIndex: msgIndex,
                 isSeparator: true, isThinking, isError });
  }
  return lines;
};
```

- `AgentView.tsx:4889` calls `wrapMessageToLines(effectiveMsg, msgIndex, maxWidth)`
  **without** overriding `addSeparator`, so every message gets a trailing
  blank separator row.
- At render (`AgentView.tsx:5325–5343`) that separator row renders blank
  normally, and only in turn-select mode is it repurposed into the gray
  ▼/▲ arrow bar via `getSelectionSeparatorType` / `generateArrowBar`.

**Net TS effect:** exactly 1 blank line between every pair of messages.

### Rust TUI — never emits the separator

`codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs` → `wrap_source()`
(lines 53–105) returns immediately after the content loop and **never
pushes a trailing blank line**. Downstream confirms zero gap:

- `push_source` (`session_context.rs:186–195`) appends the chunk verbatim.
- `paint_chunk_rows` (`scrollback_paint.rs:60–100`) paints lines back-to-back,
  advancing `y` by exactly 1 per line, **no gap between chunks** (line 91).
- `total_visual_rows` (`scrollback.rs:256–257`) = `sum(chunk.lines.len())` —
  no spacer rows accounted for.

The selection arrow-bars WERE ported (`scrollback_arrows.rs`), but since
there is no separator row they overwrite the content row immediately
above/below the selection (`fy.saturating_sub(1)` / `ly + 1`).

## Fix design

Append **one empty `Line::from("")`** at the end of `wrap_source` — the
single wrap entry point used by `push_source`, `insert_source_at`, and
`rewrap_chunk`. Because every downstream consumer derives from
`RenderedChunk::lines`, the separator automatically flows into:

- `total_visual_rows` accounting,
- resize rewrap (`set_viewport_width` → `rewrap_chunk`),
- `paint_chunk_rows` painting,
- scroll math (`skip_rows`, `stick_to_bottom`).

### All return sites in `wrap_source` must append the separator

`wrap_source` has three return paths — each must end with the blank line
so all `ChunkKind` variants get it uniformly:

1. Thinking-then-ToolCall early return (`chunk_wrap.rs:79–82` → `wrap_tool_call`)
2. diff early return inside `wrap_tool_call` (`chunk_wrap.rs:150–158`)
3. default path (`chunk_wrap.rs:104`)

Cleanest implementation: append the blank line **once** at the very end
of `wrap_source`, and have the `ToolCall` branch NOT return early but
fall through — OR wrap the two early-returning helpers so the caller
appends. Preferred: build `out` in `wrap_source`, call `wrap_tool_call`
to get its lines, then `out.push(Line::from(""))` in one place before the
final `return`. The worker chooses the least-invasive shape that keeps
files < 300 LoC.

### Modal must not show the separator

`TurnContentModal` sources full text from `full_text_for_seq` /
`ChunkSource.text` (`scrollback_select.rs:113–135`), **not** the cached
`lines`, so the wrap-level separator does not leak into the modal.
Must be verified by a test.

### Arrow-bar parity

`paint_selection_arrow_bars` (`scrollback_arrows.rs:97–121`) paints ▼ on
`fy-1` and ▲ on `ly+1`. With a trailing blank separator now occupying
`ly+1` (the selected chunk's own gutter) and `fy-1` landing on the
PREVIOUS chunk's gutter, the arrow bars now sit on blank rows and no
longer overwrite content. `scroll_selected_into_view`
(`scrollback_select.rs:213–214`) already reserves ±1 rows. Verify with a
test that selecting a middle turn keeps all content visible.

## Loci table

| Concern | File | Lines |
|---|---|---|
| TS separator source | `src/tui/utils/conversationUtils.ts` | 50–130 (117–127) |
| TS call site | `src/tui/components/AgentView.tsx` | 4889 |
| TS separator render / arrow bars | `src/tui/components/AgentView.tsx` | 5325–5343 |
| Rust wrap entry (FIX HERE) | `codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs` | 53–105 |
| Rust tool-call wrap | `codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs` | 115–191 |
| Rust push/insert/rewrap | `codelet/fspec-tui/src/store/agent_view/session_context.rs` | 186–215 |
| Rust row accounting | `codelet/fspec-tui/src/views/agent/scrollback.rs` | 256–257 |
| Rust painter | `codelet/fspec-tui/src/views/agent/scrollback_paint.rs` | 60–100 |
| Rust arrow bars | `codelet/fspec-tui/src/views/agent/scrollback_arrows.rs` | 58–122 |
| Rust select scroll reserve | `codelet/fspec-tui/src/views/agent/scrollback_select.rs` | 198–223 |
| Modal source (no leak) | `codelet/fspec-tui/src/views/agent/scrollback_select.rs` | 113–135 |

## Acceptance signals

- One blank row after every chunk's content, for every `ChunkKind`.
- `total_visual_rows` = content rows + N chunks.
- Modal full text unaffected (no trailing blank).
- Item-mode arrow bars land on gutter rows, content stays visible.
- `cargo test -p codelet-fspec-tui` green; clippy + fmt clean.

## Build/verify note

Release binary must be rebuilt to see the fix in a live TUI:
`cd codelet && cargo build --release -p codelet-cli`.
