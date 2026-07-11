# CONT-001 — Design: rig-core streaming loop exits with unanswered tool results (Failure Mode F)

**Type:** Bug (structural fix)
**Epic:** completion-contract
**Depended on by:** CONT-002 (auto-continue engine assumes this loop-exit bug is fixed so nudges are not wasted on it)

---

## 1. Problem Statement

Some providers return `stop_reason = "stop"` / `"end_turn"` on a turn that ALSO contains tool calls,
or emit trailing text/reasoning chunks *after* the tool-call chunks. In the patched rig-core
multi-turn streaming loop, the exit decision is based solely on a `did_call_tool` flag that is
**reset to `false` by any subsequent Text/Reasoning chunk**. Result: the loop exits, the tool was
executed but its result is never fed back to the model, and the turn ends mid-task. This is
"Failure Mode F" from the auto-continue investigation (cf. opencode's fix in `session/prompt.ts:1103`:
ignore finish reason when tool calls are present).

## 2. Exact Fault Location (verified against source)

File: `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs` (~1454 lines)

Inside `async_stream::stream!` (line ~469):

| Line (approx) | Code | Role |
|---|---|---|
| 473 | `'outer: loop` | multi-turn loop |
| 543–544 | `tool_calls` / `tool_results` buffers | |
| 549 | `while let Some(content) = stream.next().await` | inner provider-chunk loop |
| 564 | `Text` arm sets `did_call_tool = false` | **fault contributor** |
| 719, 723 | `Reasoning`/`ReasoningDelta` arms set `did_call_tool = false` | **fault contributor** |
| 566–692 | `ToolCall` arm: executes tool via `agent.tool_server_handle.call_tool(...)` (594), pushes into buffers (676–677), sets `did_call_tool = true` (679) | |
| 729–756 | `Final` arm captures `last_stop_reason` (731–734) | stop_reason never compared to pending tool calls |
| 766–774 | tool calls flushed to `chat_history` | |
| 777–794 | tool results flushed to `chat_history` | |
| 797–800 | `current_prompt = chat_history.pop()` | early break also DROPS the popped tool result |
| 802–818 | `if !did_call_tool { yield final_response_with_stop_reason(...); break; }` | **exit decision — the bug** |

### Repro sequence
1. Provider streams: `ToolCall(X)` → `Text("Now let me check...")` → `Final(stop_reason="stop")`
2. `ToolCall` arm executes X, `did_call_tool = true`; buffered in `tool_calls`/`tool_results`
3. `Text` arm resets `did_call_tool = false`
4. Inner loop ends; tool calls AND tool results are flushed into `chat_history`
5. Exit check `!did_call_tool` → **true** → loop breaks
6. Conversation history now ends with unanswered tool results (and the popped tool result at
   797 is dropped); model never sees them.

## 3. Required Fix

At the exit decision, the criterion must be **"did this turn produce any tool calls"**, not
"was the *last* chunk a tool call". Concretely:

- Capture `let turn_called_tools = !tool_calls.is_empty();` **before** the buffers are moved at
  lines 766/777 (or an equivalent per-turn flag set in the ToolCall arm and never reset by
  Text/Reasoning; reset only at the top of each `'outer` iteration).
- Exit condition becomes: continue the loop when the turn produced tool calls, **regardless of
  `last_stop_reason`** — e.g. `if !did_call_tool && !turn_called_tools { yield ...; break; }`
  (or simply `if !turn_called_tools`).
- `did_call_tool` may remain for any other semantics it serves, but MUST NOT drive the exit
  decision on its own.
- `last_stop_reason` propagation through `final_response_with_stop_reason` (line 330,
  `FinalResponse` struct lines 285–312) is unchanged — the *final* exit (a genuinely tool-free
  turn) still reports the provider's stop reason (PROV-039 behavior preserved).

## 4. Non-Goals / Guard Rails

- NO nudging, NO synthetic messages, NO budget logic here — that is CONT-002. This card is purely
  the structural loop-exit correctness fix.
- Do not alter tool execution ordering, `chat_history` flush order (calls at 766, results at 777),
  or `current_prompt = chat_history.pop()` (797–800).
- Depth guard (474–478) must still apply: continuing due to tool calls still counts against max
  depth exactly as before.
- Preserve existing PROV-039 (stop_reason surfacing) and PROV-040 (truncated tool call recovery)
  behavior — regression tests for those must stay green.

## 5. Acceptance Rules (recorded as Example Map on CONT-001)

1. A turn that executed >= 1 tool call continues the multi-turn loop even when the provider
   reports stop_reason "stop"/"end_turn" and even when trailing text/reasoning chunks follow the
   tool call.
2. A turn with zero tool calls and stop_reason "stop" exits the loop and yields FinalResponse with
   that stop_reason (unchanged today-behavior).
3. Trailing Reasoning/ReasoningDelta chunks after a tool call do not cause loop exit (same rule as
   text).
4. Max-depth guard still terminates the loop even when every turn has tool calls.

## 6. Testing (implemented in testing phase)

Feature file: `spec/features/multi-turn-tool-continuation.feature` (@CONT-001).
Test file: `codelet/patches/rig-core/tests/multi_turn_tool_continuation.rs` — `ScriptedModel`
harness implementing `CompletionModel`, recording every `CompletionRequest` and playing back
per-turn `RawStreamingChoice` queues; stop_reason via `GetTokenUsage::stop_reason` (PROV-039
path); driven through `AgentBuilder::new(model).build()` + `stream_prompt(..).multi_turn(5)`.
Red phase proven: trailing-text and trailing-reasoning scenarios fail with "loop exited after
turn 1"; text-only and tool_use regression guards pass.

## 7. Definition of Done

- Feature file tagged `@CONT-001` with the scenarios above, validated.
- Failing tests written first (red), then fix applied (green).
- `cargo build` + full `cargo test` for the workspace pass; PROV-039/PROV-040 tests unaffected.
- Coverage linked (test + impl line ranges) via fspec link-coverage.
