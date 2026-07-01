# RPC-399 — Settled tool card must stay pinned to end of output

## Symptom (user report)

> "when it finishes streaming, it shouldn't set the window back to the start,
> it should keep it at the end" … "it should just have the end pinned.. it
> shouldn't go back to the start"

While a tool (e.g. `bash`) is streaming, the inline tool-call card in the
fspec-tui AgentView shows the **last** lines of output (an end-pinned tail
window). The moment the tool finishes and the card settles/collapses, the
visible body **jumps back to the first lines** of the output. The last lines
the user was reading disappear.

## Architecture context

- Active TUI: `codelet/fspec-tui/` (ratatui). The `src/tui/` (TS/Ink) tree is
  the **legacy reference** and is out of scope for this fix.
- The inline body of a tool-call card is windowed at render time in
  `codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs`
  (`wrap_source` → `wrap_tool_call` → `collapse_tool_body`), introduced by
  RPC-389. The full untruncated text stays in `ChunkSource::text` for the
  `TurnContentModal` ("Enter to view full").

## Root cause

There is **no scroll-offset reset bug**. The "jump to start" is caused by the
settled branch of `collapse_tool_body` windowing from the **start** while the
streaming branch windows from the **end**.

`codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs`:

```rust
const COLLAPSED_LINES: usize = 8;        // settled window
const STREAMING_WINDOW_SIZE: usize = 10; // streaming tail window

fn collapse_tool_body<'a>(
    body_lines: &[&'a str],
    is_streaming: bool,
) -> (Vec<&'a str>, Option<String>) {
    let total = body_lines.len();
    if is_streaming {
        if total > STREAMING_WINDOW_SIZE {
            let start = total - STREAMING_WINDOW_SIZE; // END-pinned tail
            return (body_lines[start..].to_vec(), None);
        }
        return (body_lines.to_vec(), None);
    }
    if total > COLLAPSED_LINES {
        let remaining = total - COLLAPSED_LINES;
        let indicator = format!("... +{remaining} lines (Enter to view full)");
        return (body_lines[..COLLAPSED_LINES].to_vec(), Some(indicator)); // START-pinned  <-- BUG
    }
    (body_lines.to_vec(), None)
}
```

The `is_streaming` flag flips `true → false` when a `ToolResult` settles the
card (`chunk_processor.rs::handle_tool_result` sets `source.is_streaming = false`
then re-wraps). On that re-wrap the visible slice flips from a **suffix**
(`body_lines[total-10..]`, end-anchored) to a **prefix** (`body_lines[..8]`,
start-anchored). That flip is the visible "jump back to the start".

### Loci

| Concern | File:Line |
|---|---|
| Settled slice takes first N (bug) | `chunk_wrap.rs:206` `body_lines[..COLLAPSED_LINES]` |
| Streaming slice takes last N (correct) | `chunk_wrap.rs:198` `total - STREAMING_WINDOW_SIZE` |
| Flag flip on settle | `chunk_processor.rs::handle_tool_result` `source.is_streaming = false` |
| Re-wrap trigger | `chunk_processor.rs` `ctx.scrollback.rewrap_at(idx)` |
| Outer scrollback offset | `views/agent/scrollback.rs` — stays `stick_to_bottom`, NOT the cause |

## Decision

**Keep the streaming tail exactly.** On settle, the card must continue to show
the **last lines** it was showing while streaming — i.e. an end-pinned window —
rather than snapping to the first lines. The window must therefore be anchored
to the END in the settled branch too.

- Scope: **Rust TUI only** (`codelet/fspec-tui`). The legacy TS `src/tui`
  reference is intentionally left unchanged.
- Behavior: settled overflow shows the **last N lines** pinned to the END.

## Design of the fix

`collapse_tool_body` settled branch changes from a start-slice to an
end-slice so both streaming and settled windows are end-anchored.

The `... +N lines (Enter to view full)` indicator communicates that earlier
lines are hidden. Because hidden lines are now **above** the visible window
(not below), the indicator wording/placement must reflect "lines hidden above".
Concretely the settled branch returns the **last** `COLLAPSED_LINES` lines and
an indicator describing the `N` lines hidden above them, so the full output is
still reachable via the turn modal ("Enter to view full").

### Invariants that must hold

1. Settled overflow body: shows the **last** `COLLAPSED_LINES` lines (end-pinned).
2. Settled non-overflow body (`<= COLLAPSED_LINES`): shows all lines, no indicator (unchanged).
3. Streaming behavior: unchanged (last `STREAMING_WINDOW_SIZE` lines).
4. The settled indicator reflects the count of lines hidden **above** the window.
5. `ChunkSource::text` remains the full untruncated body (turn modal unaffected).
6. Diff cards (`is_diff: true`) bypass this collapse entirely — unchanged.

## Impact / regressions to check

- RPC-389 tests: `tool_call_output_collapse_rpc389.rs` and the in-file unit
  tests in `chunk_wrap.rs` assert the OLD first-8 behavior — these must be
  updated to the new end-pinned contract (this is the intended behavior change,
  not a regression).
- `tool-call-output-collapse.feature` (RPC-389) scenarios describing the
  settled first-8 window must be revised to the end-pinned contract.

## Files in scope

- `codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs` (impl + unit tests)
- `codelet/fspec-tui/tests/tool_call_output_collapse_rpc389.rs` (integration tests)
- `spec/features/tool-call-output-collapse.feature` (RPC-389 feature — settled scenarios)
- New feature: `spec/features/settled-tool-card-pinned-to-end.feature` (RPC-399)
