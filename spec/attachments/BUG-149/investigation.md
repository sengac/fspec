# BUG-149 — Live tool output not folded into TUI card (empty `tool_call_id` on ToolProgress)

**Date:** 2026-06-30
**Component:** `codelet-cli` streaming loop + `codelet-fspec-tui` agent view
**Related prior work:** RPC-398 (session-id key mismatch — DONE), BUG-126 (per-session tool-progress isolation), RPC-389 (tool-call output collapse / streaming window)

---

## 1. Symptom

In the ratatui `fspec-tui`, bash/tool output does **not** stream incrementally into the tool-call
card while the command is running. Output only appears **once the command finishes**, when the card
collapses to the settled 8-line view. This is the same user-visible symptom RPC-398 targeted, but
RPC-398's fix was **necessary but not sufficient** — a second, independent defect remained.

## 2. Pipeline (verified end-to-end)

```
BashTool (bash_streams.rs)
  read_until('\n') per line
  -> emit_tool_progress(session_id, line, is_stderr)              [tools/src/bash_streams.rs:73,114]
     -> TOOL_PROGRESS_CALLBACKS.with(&session_id, cb)             [tools/src/tool_progress.rs:67]
        (RPC-398: now registered under the REAL session id — FIRES)
        -> cb = closure set in stream_loop.rs                      [cli/.../stream_loop.rs:471-476]
           -> emitter.emit_tool_progress("", "bash", chunk, is_stderr)   <-- EMPTY id
              -> StreamEvent::ToolProgress(ToolProgressEvent{ tool_call_id: "", .. })
                 -> BackgroundProgressEmitter / BackgroundOutput
                    -> StreamChunk::ToolProgress{ tool_call_id: "", .. } (broadcast)
                       -> fspec-tui chunks_rx -> session_context.rs:107
                          -> handle_tool_progress(ctx, info)        [fspec-tui/.../chunk_processor.rs:199]
                             -> rposition ChunkKind::ToolCall{ tool_call_id } == info.tool_call_id("")
                                -> NO MATCH (cards hold the real provider id) -> DROPPED
```

## 3. Root cause

Two facts collide:

1. **Emit side (`stream_loop.rs:474`)** passes an **empty string** as the tool_call_id:
   ```rust
   emitter.emit_tool_progress("", "bash", chunk, is_stderr);
   ```
   The callback is registered **once before the stream loop** (capturing only the `emitter`), so at
   registration time it has no knowledge of which tool call is currently executing.

2. **Match side (`chunk_processor.rs:199-209`)** folds progress into a card by **exact**
   `tool_call_id` equality:
   ```rust
   .rposition(|c| matches!(c.kind, ChunkKind::ToolCall{ tool_call_id, .. } if *tool_call_id == info.tool_call_id))
   ```
   ToolCall cards are pushed with the **real** provider-assigned id
   (`handle_tool_call` -> `info.id.clone()`, `chunk_processor.rs:128`).

An empty id therefore matches **no** card, so every `ToolProgress` chunk is silently discarded.
The final output still appears because `ToolResult` travels a **separate** path
(`handle_tool_result`) and **does** carry the real id (`emit_tool_result(&tool_result.id, ...)`).

## 4. Why RPC-398 didn't catch it

RPC-398's behavioral tests operate at the `tools` crate level: they register a callback and call
`BashTool::call`, asserting the callback receives the lines. They verify the **session-id** key
agreement (registration key == emit key) but stop at the callback boundary — they never assert the
`tool_call_id` carried on the resulting `StreamEvent::ToolProgress`, nor exercise the TUI
`handle_tool_progress` matching. So the empty-id defect sat downstream of RPC-398's test surface.

## 5. Execution model (why a simple fix is safe)

Within a single turn, tool execution is **serial**: `stream_loop` sets
`tool_execution_in_progress = true` on a `ToolCall` chunk and back to `false` on the matching
`ToolResult` chunk (stream_loop.rs:896/961). `stream.next()` blocks on the tool between those two
events, so at most **one** tool call is in flight when `ToolProgress` is emitted. Tracking the
"current tool_call_id" in shared state and emitting it is therefore unambiguous.

## 6. Chosen fix (Option A — thread the real id)

Maintain the **active tool_call_id** in state the progress callback can read, and emit it instead of
`""`:

- Introduce a shared `Arc<Mutex<Option<String>>>` (or equivalent) in `stream_loop`, captured by the
  progress callback closure.
- On `handle_tool_call`, set it to `tool_call.id`.
- On `handle_tool_result`, clear it (back to `None`).
- The callback emits the current id (fallback to `""` only if unexpectedly absent — which the TUI
  will still drop, preserving today's behavior for that edge).

This keeps BUG-126 session isolation (registration key unchanged) and RPC-398 (session-id agreement)
intact; it only fixes the *tool_call_id* field on the emitted progress.

### Alternative considered (Option B — TUI fallback)

In `handle_tool_progress`, when `info.tool_call_id` is empty, fold into the most-recent **streaming**
ToolCall card. Smaller (one file), correct under serial execution, but:
- Encodes a "last card wins" heuristic that is fragile if concurrent tool calls are ever introduced.
- Leaves an empty id on the wire (worse for any future consumer keyed on id).

Option A is preferred; Option B could be added as defense-in-depth but is out of scope.

## 7. Acceptance (what a fix must prove)

1. A `ToolProgress` produced during bash execution carries the **same** `tool_call_id` as the
   preceding `ToolCall` for that tool.
2. The TUI `handle_tool_progress` folds that progress into the matching card **before** the
   `ToolResult` settles it (live streaming, not end-of-command).
3. Session isolation preserved (no cross-session bleed).
4. When no tool is active, a stray progress emit does not panic and does not corrupt an unrelated
   card.

## 8. Key files

| File | Role |
|------|------|
| `codelet/cli/src/interactive/stream_loop.rs` | callback registration (471-476) + tool_call/result handling loop |
| `codelet/cli/src/interactive/stream_handlers.rs` | `handle_tool_call` / `handle_tool_result` |
| `codelet/cli/src/interactive/output.rs` | `emit_tool_progress`, `ToolProgressEvent` (103-112) |
| `codelet/agent-loop/src/background_output.rs` | `StreamEvent::ToolProgress` -> `StreamChunk::ToolProgress` (211) |
| `codelet/fspec-tui/src/store/agent_view/chunk_processor.rs` | `handle_tool_progress` (199) matches by exact id |
| `codelet/fspec-tui/src/store/agent_view/session_context.rs` | ToolProgress dispatch (107) |
