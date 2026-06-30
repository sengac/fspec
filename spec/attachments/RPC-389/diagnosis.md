# RPC-389 — Tool-result body collapse + streaming window parity

## Symptom

When a tool produces output, the Rust TUI appends the **entire** result (and the
entire streamed progress) to the tool-call card verbatim. A 500-line build log is
dumped in full into the scrollback. The TypeScript reference instead shows a
short **collapsed** view inline and keeps the full text behind an expand action.

## Reference behaviour (the contract)

`src/tui/components/AgentView.tsx:533-605`.

### Constants
```ts
const STREAMING_WINDOW_SIZE = 10;   // lines visible during live streaming
const COLLAPSED_LINES       = 8;    // lines visible when collapsed (settled)
```
(`DIFF_COLLAPSED_LINES = 25` is for diff output — **out of scope**, see below.)

### Settled output — `formatCollapsedOutput` (568-581)
- Split body into lines.
- If `lines.length <= 8` → show all (no truncation).
- Else → keep the **first 8** lines, then append a single indicator line:
  `... +${remaining} lines (Enter to view full)` where `remaining = lines.length - 8`.
- Head-clip, line-count based.

### Live streaming — `createStreamingWindow` (593-605)
- While output is streaming, show a **tail window**: if `lines.length > 10`, keep
  only the **last 10** lines (`lines.slice(-10)`). No indicator text — it scrolls.

### Full copy
- TS stores both `toolResultContent` (collapsed) and `toolResultFullContent`
  (full). The full copy feeds the Enter/`/expand` modal (TUI-043).

## Rust port — what already exists

- **Full copy + expand modal ALREADY EXIST.** `TurnContentModal` (RPC-382/383,
  `codelet/fspec-tui/src/views/agent/turn_modal.rs`) renders the full
  `ChunkSource::text` for a selected turn, opened by **Enter in SELECT mode**
  (RPC-381). It reads `ctx.scrollback.full_text_for_seq(seq)` →
  `scrollback_select.rs:113`. So the "view full" path is the existing modal; the
  indicator text `(Enter to view full)` maps to it directly.
- **No inline collapse / streaming window.** `handle_tool_result`
  (`chunk_processor.rs:128-163`) and `handle_tool_progress` (166-189) push the
  full text into `source.text`; `wrap_source` (`chunk_wrap.rs:28-71`) wraps every
  line with no cap. There is no `COLLAPSED_LINES`/`STREAMING_WINDOW_SIZE`
  equivalent anywhere (verified by grep).

## Architecture for the fix

**Invariant: `ChunkSource::text` must remain the FULL untruncated body** so the
existing `TurnContentModal` keeps showing everything. Collapsing must happen at
the **inline render** layer that produces `RenderedChunk::lines`, NOT by mutating
`source.text`.

The clean insertion point is `wrap_source(source, width)` (or a thin wrapper it
calls), which already owns the `ChunkSource → Vec<Line>` transform and has access
to `source.kind` and `source.is_streaming`:

1. Produce the full wrapped lines as today.
2. If `source.kind` is `ChunkKind::ToolCall { .. }`:
   - **Streaming** (`source.is_streaming == true`): keep only the **last 10**
     wrapped *body* lines (the header `● ToolName(args)` line should remain the
     anchor — decide whether the window counts the header; match TS which windows
     the OUTPUT body, so keep the header line + last 10 body lines). No indicator.
   - **Settled** (`source.is_streaming == false`): if body lines > 8, keep the
     header + first 8 body lines + one indicator line
     `... +N lines (Enter to view full)`.
3. Non-ToolCall chunks: unchanged.

> Decision to make explicit in scenarios: does the 8/10 count include the header
> line? TS operates on the *result content* only (header is separate). So in
> Rust, count **body** lines (everything after the `● ...` header line). Keep the
> header always visible.

Because `wrap_source` is re-run on every progress append and on resize (RPC-078),
the collapse is naturally recomputed — no extra state needed. `total_visual_rows`
(scrollback) sums `chunk.lines`, so scrolling math adjusts automatically.

### Indicator-line styling
Render the `... +N lines (Enter to view full)` line dimmed/secondary (match the
existing dim style used elsewhere, e.g. footer hints). Keep it a normal wrapped
line so selection/scroll treat it like any row.

### Width interaction
Collapse counts **wrapped visual lines** (post width-wrap), mirroring TS which
counts `content.split('\n')` BEFORE wrap — NOTE the divergence: TS counts
*logical* lines, Rust wraps first. To match TS as closely as practical, count
**hard `\n`-delimited body lines** for the 8/10 threshold (pre-wrap), then wrap
the retained lines. This avoids width-dependent collapse behaviour and matches
the TS semantics. Encode this choice in a scenario.

## Examples / behaviour table (target)

| State | body lines | inline shows |
|-------|-----------|--------------|
| settled | 5 | all 5 (no indicator) |
| settled | 8 | all 8 (no indicator) |
| settled | 20 | first 8 + `... +12 lines (Enter to view full)` |
| streaming | 25 | last 10 (tail), no indicator |
| streaming → settled | 25 | becomes first 8 + `... +17 lines (Enter to view full)` |
| any | (full text) | modal (Enter in SELECT) still shows ALL lines |

## Out of scope (documented, separate future work)

- **Diff-style collapse** (`DIFF_COLLAPSED_LINES = 25`, `CONTEXT_LINES = 3`,
  `[R]-`/`[A]+` prefixes, `formatDiffForDisplay` AgentView.tsx:670-771). The Rust
  port has **no inline diff renderer** for Edit/Write tool results (verified:
  no `formatDiff`/`pendingDiff`/`[R]-` in `fspec-tui`). Porting the diff renderer
  is a distinct feature, not a truncation fix, and is excluded from this card.
- **Assistant-text streaming window.** TS `createStreamingWindow` is also used
  for streaming assistant text; this card scopes the window to **tool-call body**
  only (the subject is tool calls). Assistant-streaming window parity is separate.

## Files

| File | Role |
|------|------|
| `src/tui/components/AgentView.tsx:533-605` | TS reference (contract) |
| `codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs:28-71` | **Primary fix site** (`wrap_source`) |
| `codelet/fspec-tui/src/store/agent_view/chunk_processor.rs:128-189` | result/progress accumulation — keeps `source.text` full (unchanged behaviour) |
| `codelet/fspec-tui/src/views/agent/turn_modal.rs` | existing full-content modal (the "view full" target) — unchanged |
| `codelet/fspec-tui/src/views/agent/scrollback*.rs` | scrollback row math — recomputes from `chunk.lines` automatically |

## Dependencies

Depends on **RPC-388** (same tool-call display subsystem; land arg-header parity
first to avoid overlapping edits to the tool-call rendering path and tests).
