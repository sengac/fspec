# Research: done() Immediate Termination (VERIFIED 2026-07-10, deep-search pass)

## Problem Statement

When the agent calls `done(summary)`, the agent loop does **not** immediately terminate.
The acceptance is written into a registry at tool-execution time but only consumed at the
`FinalResponse` settle point — after the provider naturally stops streaming. Between those
two points the model can keep generating text and tool calls, making the recorded summary
stale/misleading.

**Root cause (verified):** the patched rig-core multi-turn loop *unconditionally re-prompts
the model after any turn that executed tools* (CONT-001 fix). `done()` is itself a tool
call, so an accepted `done()` guarantees at least one more model segment.

```rust
// codelet/patches/rig-core/src/agent/prompt_request/streaming.rs:810-831
// CONT-001: Exit only when the turn produced NO tool calls. A turn
// that executed tools must always continue so the tool results are
// fed back to the model...
if !did_call_tool && !turn_called_tools {
    yield Ok(MultiTurnStreamItem::final_response_with_stop_reason(...));  // :829
    break;                                                                 // :830
}
```

## Verified Current Flow

### 1. Tool execution — `codelet/tools/src/done.rs`

Registries (session-scoped statics):
- `CONTRACT_STATE` (armed + goal + rejections) — done.rs:74-75
- `DONE_ACCEPTANCE` (session_id → accepted summary) — done.rs:78-79

`DoneTool::call()` paths (done.rs:315-363):

| Path | Lines | Behavior |
|---|---|---|
| Tier-0 empty summary | 317-323 | `Err(ToolError::Validation)` |
| Stale (not armed) | 327-329 | inert `Ok("Acknowledged (auto-continue is off).")` |
| Tier 1 fail (goal) | 332-344 | `record_rejection()` (:335) + `Err(Validation)` |
| Tier 2 fail (verify) | 346-354 | `record_rejection()` (:348) + `Err(Validation)` |
| **Accepted** | 357-362 | `DONE_ACCEPTANCE.insert` (:359-361), returns `"Completion recorded. The turn will finish with your summary."` |

`take_done_acceptance` is read-and-clear (done.rs:162-167); `clear_done_acceptance`
(done.rs:171-175) also runs from `set_continue_armed(false)` (done.rs:91-93).

### 2. Single settle point — `codelet/cli/src/interactive/stream_loop.rs`

- `take_done_acceptance(session_id)` is called at **stream_loop.rs:1458**, inside the
  `FinalResponse` arm (arm spans :1054-1615; decision block :1451-1610).
- `FinishWithSummary` handling at :1510-1526; `emit_done_with_stop_reason` + `break` at
  :1613-1614.
- **`codelet/napi/src/agent_loop.rs` has NO settle point of its own.** It delegates to
  `run_agent_stream_with_images` (napi agent_loop.rs:125, :1064, :1188 → wrapper at
  stream_loop.rs:226-256 → shared `run_agent_stream_internal` at stream_loop.rs:269).
  Same for `codelet/agent-loop/src/agent_loop.rs` (:1042, :1181, :1247) and
  `agent-loop/src/dispatch.rs:88`. **A fix in stream_loop.rs covers all surfaces.**

### 3. What the outer loop sees — match arms (stream_loop.rs `match chunk`, :886-2094)

| Arm | Lines | Notes |
|---|---|---|
| `StreamAssistantItem(Text)` | :887-902 | accumulates `assistant_text` |
| `StreamAssistantItem(ToolCall)` | :903-941 | **tool name available** (`tool_call.function.name`, :922); `last_tool_name` set in stream_handlers.rs:143; `tool_execution_in_progress = true` (:940) |
| `StreamAssistantItem(ReasoningDelta)` | :942-964 | thinking |
| `StreamUserItem(ToolResult)` | :965-1011 | **outer loop DOES see tool results.** Result string extracted at :982-994. ⚠️ rig's `ToolResult` carries only `id`/`call_id`/`content` — **no tool name** (yielded at rig streaming.rs:685-686); identify done() via `last_tool_name` or the acceptance registry |
| `Usage` | :1012-1053 | token updates |
| `FinalResponse` | :1054-1615 | the settle point |
| `Some(Err(e))` | :1616-2058 | compaction/network/error cascade |
| `None` | :2059-2089 | flush + `emit_done_with_stop_reason` + break |

The done() ToolResult chunk (content = the acceptance string) reaches the outer loop
**before** rig re-prompts the model — so an early-exit check at the ToolResult arm is viable.

### 4. Existing early-exit mechanisms

- **User interrupt (ESC):** `is_interrupted.store(true)` at stream_loop.rs:804 (CLI select
  :801-808) / NAPI `interrupt_notify` (:835-858) → yields `None` → loop-top check at
  :756-776 flushes text (`handle_final_response` :770), emits done (:774), breaks (:775).
- **rig-core `CancelSignal`** (rig streaming.rs:461, checked at :508-513, :559-561,
  :585-587, :608-671, :702-704, :743-748): yields `Err(PromptError::PromptCancelled(history))`
  — an **error-path** exit used by the CompactionHook. The outer loop classifies it via
  `classify_compaction_branch` (stream_loop.rs:1634) into the compaction recovery cascade.
  **Reusing it verbatim for done() would mis-route into compaction recovery.**

### 5. History invariant — early break is SAFE

fspec keeps its own history (`session.messages`) via `stream_handlers.rs`:
- `handle_tool_call` (:106-154) only buffers (`tool_calls_buffer.push` :146).
- `handle_tool_result` (:157-242) flushes the buffered Assistant tool_use message
  (:168-171) **and** pushes the tool_result User message (:174-177).

**Breaking immediately after the done() ToolResult chunk preserves the tool_use/tool_result
pairing** — both are already in `session.messages` before the break would run. rig's own
`chat_history` copy is discarded with the dropped stream (only read on `PromptCancelled`
recovery via `reconcile_session_messages`, stream_loop.rs:1657-1667). CONT-001's invariant
(rig test `codelet/patches/rig-core/tests/multi_turn_tool_continuation.rs`) concerns rig
ending a turn without feeding results back to the *model* — the outer loop deliberately
abandoning the stream after persisting both halves does not violate it.

## Solution Options (re-evaluated)

| Option | Verdict |
|---|---|
| A. AtomicBool early-exit polled before `stream.next()` | Works but redundant — the acceptance registry already IS the flag |
| B. rig-core `Terminate` variant | Most invasive; touches patched vendored code + all consumers |
| C. Match on the tool-result string | Fragile; ToolResult has no tool name, string coupling |
| **D. Post-ToolResult registry check (RECOMMENDED)** | Check `DONE_ACCEPTANCE` in the ToolResult arm (after `handle_tool_result`, ~stream_loop.rs:1010); if accepted → run the FinishWithSummary teardown, emit done, break |
| CancelSignal reuse | Rejected — routes into the compaction error cascade |

### Option D caveats
- rig may batch multiple ToolCalls before their ToolResults in one segment; the check fires
  on done()'s own result, but results for *other* tools in the same batch that haven't been
  yielded yet would be lost. Decide: defer break until the segment's results are drained
  (`tool_execution_in_progress == false`) or accept done()-last-in-batch semantics.
- Peek vs take: use a non-clearing peek at the ToolResult arm OR take-and-act; if taking,
  the FinalResponse fallback must not double-fire (it reads `None` afterwards — safe, but
  the nudge path must not then fire either → break immediately after taking).

## What an early-exit path must replicate (from the FinalResponse FinishWithSummary arm)

At stream_loop.rs:1510-1526 + :1613-1614 today:
1. AutoContinue mode: `emit_status("✓ done: {summary}")` (:1523)
2. Goal mode: `apply_goal_acceptance` + `set_session_goal(None)` (:1516-1519) — **scoped to
   CONT-006**, keep the teardown in a shared helper so both cards use one code path
3. `session.continue_nudges_used = 0` (:1525)
4. Flush any accumulated `assistant_text` (pattern: `handle_final_response`, interrupt path :770)
5. `emit_done_with_stop_reason` (:1613) — decide stop_reason value (see open questions)
6. `break` (:1614)
7. Post-loop disarm/cleanup already handled at `agent_runner.rs:110-111` (CLI).

Keep the existing FinalResponse-arm check as **fallback** (e.g. acceptance raced with
stream end).

## Side finding (spun out)

`codelet/napi/src/agent_loop.rs` never calls `set_continue_armed`/`set_session_goal`
(zero matches), unlike `codelet/agent-loop/src/agent_loop.rs:529-530`. On the production
NAPI surface DoneTool is never registered at all. **Tracked as CONT-009** — independent
bug, but must land for immediate termination to matter on the TUI surface.

## Test Coverage Required

- done() as last tool of a segment → loop breaks before any further model segment
- done() mid-batch with other pending tool results → chosen semantics hold, no orphaned
  tool_use/tool_result pairs in `session.messages`
- Rejected done() (goal Tier 1/2) → loop continues (no early exit on rejection)
- Stale done() (disarmed) → inert, no early exit
- FinalResponse fallback still works if the ToolResult-arm check is bypassed
- Nudge counter reset on early exit; no nudge fires after early exit
- Interrupt during the same segment still wins (interrupt check precedes chunk handling)

## Open Questions

1. Batch semantics: break immediately on done()'s ToolResult, or drain the current
   segment's remaining tool results first? (Recommend: drain-then-break.)
2. stop_reason for early termination: reuse `"stop"`, or introduce `"done"`?
   (`emit_done_with_stop_reason` currently passes through provider stop reasons.)
3. Should assistant text generated *before* done() in the same segment be flushed to
   history (recommend yes, mirroring the interrupt path)?
