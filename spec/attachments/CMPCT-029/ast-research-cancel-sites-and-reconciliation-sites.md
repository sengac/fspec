# CMPCT-029 AST Research — Cancel Sites and Reconciliation Sites

## Rig cancel sites inside the streaming while-loop

Grep for `cancel_signal.is_cancelled` inside
`codelet/patches/rig-core/src/agent/prompt_request/streaming.rs` yielded
eight hits. Only three are *exits* from the streaming while-loop — the
others are pre-loop guards / post-completion guards:

| Line | Context | Local vecs at cancel | Patchable? |
|------|---------|----------------------|------------|
| 408  | `on_completion_call` (pre-stream) | `tool_calls` not yet created for this turn | Not relevant — nothing to flush. |
| 459  | after `on_text_delta` | `tool_calls`=[], `tool_results`=[] — text-only | Not relevant — no tool state. |
| **485** | after `on_tool_call` (BEFORE tool runs) | `tool_calls` push hasn't happened yet (streaming.rs:515). Rig has yielded the `ToolCall` to fspec, but the tool is about to execute. | **No rig patch**: fspec has the call in `tool_calls_buffer`. CMPCT-029 closes the dangle on the fspec side by injecting a synthetic `"cancelled_by_context_limit"` tool_result. |
| **508** | after `on_tool_result` | `tool_calls.push(...)` happened at line 515 *and* `tool_results.push(...)` at line 516 — wait, those are AFTER the cancel check. Let me re-read. | See analysis below. |
| 541  | after `on_tool_call_delta` | Partial tool args buffer only; no complete call | **Unrecoverable** — no complete call to preserve. |
| 582  | after `on_stream_completion_response_finish` | End-of-response — while-loop will exit naturally, which already flushes at lines 605-633. | Not patchable *and not broken* — the natural-exit flush handles it. |

### Site 508 (the critical one) — precise data-flow

Reading streaming.rs lines 480-531 carefully:

```rust
let tc_result = async {
    // ... args, on_tool_call hook at 484
    if cancel_signal.is_cancelled() { return Err(...); }   // <--- site 485 (was called "486" in the plan)
    // ... execute tool at 493-500
    if let Some(ref hook) = self.hook {
        hook.on_tool_result(...).await;
        if cancel_signal.is_cancelled() {                  // <--- site 508 (was called "509" in the plan)
            return Err(StreamingError::Prompt(PromptError::prompt_cancelled(
                chat_history.read().await.to_vec()
            ).into()));
        }
    }
    let tool_call_msg = AssistantContent::ToolCall(tool_call.clone());
    tool_calls.push(tool_call_msg);                        // <-- 515 (after cancel check)
    tool_results.push((..., tool_result.clone()));          // <-- 516 (after cancel check)
    ...
}.instrument(tool_span).await;
```

**Key insight**: at site 508, the pushes at 515-516 have NOT happened yet.
The `tool_calls` and `tool_results` local vecs might contain earlier-turn
state (multiple tools in a multi-tool turn), but NOT the current
`tool_call` or `tool_result`.

**Revised rig patch**: at site 508, before yielding PromptCancelled, we
need to:

1. Push the current `tool_call` into `tool_calls` (mirrors line 515).
2. Push the current `(id, call_id, tool_result)` triple into `tool_results`
   (mirrors line 516).
3. Flush `tool_calls` + `tool_results` into `chat_history` using the same
   structure as lines 605-633.
4. Then call `chat_history.read().await.to_vec()` which will now contain
   the full pair.

This single-site patch makes site 508's PromptCancelled payload carry
the complete tool pair(s) for this turn.

### Site 485 — no rig patch

At site 485 the tool has NOT yet been invoked. There is no `tool_result`
to flush. The `tool_call` *has* been yielded out of the stream to fspec,
where fspec's `handle_tool_call` pushed it onto `tool_calls_buffer`.
After `handle_tool_call` flushes into `session.messages` on the next
ToolResult chunk (which never comes because we cancelled), the buffer
contents are what matters.

Actually — re-reading fspec's `handle_tool_call` at `stream_handlers.rs`:
```rust
tool_calls_buffer.push(AssistantContent::ToolCall(tool_call.clone()));
```
So the tool_call lives in `tool_calls_buffer`, NOT in `session.messages`.
It would only move to `session.messages` inside `handle_tool_result`
(via `add_assistant_tool_calls_message`).

**Therefore**: at site 485 cancellation:
- `session.messages` has an Assistant text flush (if any text was streamed)
  and does NOT have the in-flight tool_call.
- `tool_calls_buffer` has the in-flight tool_call.

**fspec-side fix**: when handling PromptCancelled in the compaction-cancel
branch, drain `tool_calls_buffer` into `session.messages` as an Assistant
message, then the orphan detector will see a dangling tool_call, and
`inject_synthetic_tool_results_for_orphans` will close it.

But wait — the stream_loop's compaction-cancel branch runs from outside
the match arm that has `tool_calls_buffer` scope. Let me verify.

Looking at stream_loop.rs around the compaction-cancel branch: `tool_calls_buffer`
IS in scope at the match arm for `Some(Err(e))` (it's a local variable of
the outer stream_loop function). So the fix is simple: before calling
`begin_compaction_recovery`, drain the buffer to `session.messages`.

### Site 541 — unrecoverable

Partial tool_call_delta — only a name fragment or args delta. No complete
call to flush. Dropping it is correct behavior.

## Reconciliation sites in fspec

The compaction-cancel branch lives at
`stream_loop.rs:1293-1362` (the `Some(Err(e))` arm of the stream loop).
After CMPCT-028 it looks like:

```rust
let branch = classify_compaction_branch(&e, &token_state);
if matches!(branch, CompactionBranch::Recover { .. }) {
    let policy = super::recovery_compaction::begin_compaction_recovery(...)?;
    in_loop_compaction_restart!(policy);
    continue;
}
```

CMPCT-029 inserts reconciliation + orphan-closure BEFORE the call to
`begin_compaction_recovery`:

```rust
if matches!(branch, CompactionBranch::Recover { .. }) {
    // CMPCT-029: reconcile rig-side tool state into session.messages
    if let Some(rig_ch) = extract_prompt_cancelled(&e) {
        reconcile_session_messages(&mut session.messages, rig_ch);
    }
    // Drain fspec-side tool_calls_buffer (Site 485 recovery)
    if !tool_calls_buffer.is_empty() {
        add_assistant_tool_calls_message(&mut session.messages, tool_calls_buffer.clone())?;
        tool_calls_buffer.clear();
    }
    // Close any remaining orphans with synthetic tool_results
    inject_synthetic_tool_results_for_orphans(&mut session.messages);

    let policy = super::recovery_compaction::begin_compaction_recovery(...)?;
    in_loop_compaction_restart!(policy);
    continue;
}
```

## Types we need from `rig::message`

- `Message::Assistant { id, content: OneOrMany<AssistantContent> }`
- `Message::User { content: OneOrMany<UserContent> }`
- `AssistantContent::ToolCall(ToolCall)` — `ToolCall { id, call_id: Option<String>, function: Function }`
- `UserContent::ToolResult(ToolResult)` — `ToolResult { id, call_id: Option<String>, content: OneOrMany<ToolResultContent> }`
- `ToolResultContent::Text(Text)`

The match on `call_id` (`Option<String>`) must handle both the Anthropic
path (call_id=Some) and the OpenAI path (call_id=None where `id` is the
correlation key).

## Summary of deliverables

1. **Rig patch at site 508 only**: push current call+result into the
   local vecs, then flush both vecs into `chat_history` before yielding
   PromptCancelled.
2. **`validate_no_orphan_tool_calls(&[Message]) -> Result<(), Vec<String>>`**
   in `interactive_helpers.rs` — defensive guard at the start of
   `execute_compaction`.
3. **`reconcile_session_messages(&mut Vec<Message>, &[Message])`** —
   append any tool pairs from rig's history that fspec doesn't have.
4. **`inject_synthetic_tool_results_for_orphans(&mut Vec<Message>) -> usize`**
   — close any dangling tool_calls with `{"status": "cancelled_by_context_limit"}`.
5. **stream_loop.rs wiring** — call (1)-(3) in the Path C compaction-cancel
   branch before `begin_compaction_recovery`.
6. **Integration tests** in `compaction_tool_call_preservation_test.rs`
   exercising the orphan detector, the reconciler, the synthetic injector,
   and the execute_compaction guard.
