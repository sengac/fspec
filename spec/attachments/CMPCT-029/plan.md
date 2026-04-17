# CMPCT-029 — Preserve Mid-Tool-Call State When PromptCancelled Fires

**Parent:** CMPCT-022
**Bug:** BUG 8
**Related:** PROV-050 (split-safe compaction ensuring tool call/result pairs)

## The Problem

Rig's streaming loop in `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs` accumulates tool calls and results in local vectors (lines 443-444):

```rust
let mut tool_calls = vec![];
let mut tool_results = vec![];
```

These are flushed to `chat_history` only if the `while let` completes normally (lines 605-633, in the `is_text_response` path after the final response handler). The inner loop yields intermediate items but does NOT push to `chat_history` as it goes.

### Cancel Sites That Leak

At these yield sites, rig has accumulated state that is NEITHER in fspec's `session.messages` NOR in rig's internal `chat_history`:

| Line | Hook                    | Internal State at Yield                                     |
|------|-------------------------|-------------------------------------------------------------|
| 486  | `on_tool_call`          | `tool_call` emitted but NOT yet in `tool_calls` vec         |
| 509  | `on_tool_result`        | Tool executed; `tool_results` has result; `tool_calls` has call; neither flushed to `chat_history` |
| 542  | `on_tool_call_delta`    | Partial tool args in buffer; call not yet closed            |

### What Gets Lost

When `PromptCancelled` is yielded at site 509, rig has a matching `tool_call` + `tool_result` pair in its local vecs. The current stream_loop.rs:
- Sees only the `Some(Err(e))` from `stream.next().await`.
- Never receives the `StreamedUserContent::ToolResult` chunk (it was swallowed by the cancel check).
- Therefore never calls `handle_tool_result` which would have added the result to `session.messages`.
- Drops the `chat_history: Box<Vec<Message>>` payload from the error variant.

Result: `session.messages` contains:
```
[..., User(prompt), Assistant(tool_call_X)]
```
with a **dangling tool_call** and no matching tool_result. The next API request (after compaction) will either:
- Error out with `"tool_use block must be followed by tool_result"` (Anthropic).
- Silently continue with inconsistent history (OpenAI may or may not accept this).

### Evidence: PromptCancelled carries the chat_history

```rust
// codelet/patches/rig-core/src/completion/request.rs:147-154
#[error("PromptCancelled")]
PromptCancelled { chat_history: Box<Vec<Message>> },

impl PromptError {
    pub(crate) fn prompt_cancelled(chat_history: Vec<Message>) -> Self {
        Self::PromptCancelled {
            chat_history: Box::new(chat_history),
        }
    }
}
```

The payload IS there. `stream_loop.rs:1131-1202` just doesn't read it.

## The Fix

### Step 1 — Extract the chat_history (enabled by CMPCT-025)

After CMPCT-025 lands, we have `extract_prompt_cancelled(&e) -> Option<&Vec<Message>>`. Use it in the compaction-cancel branch:

```rust
if let Some(rig_chat_history) = extract_prompt_cancelled(&e) {
    // rig_chat_history is rig's view of the conversation including any
    // partial tool_calls/tool_results that rig accumulated but hadn't yet
    // flushed to its chat_history writer.
    ...
}
```

**BUT** — check rig's code carefully:
```rust
yield Err(StreamingError::Prompt(PromptError::prompt_cancelled(chat_history.read().await.to_vec()).into()));
```

The `chat_history.read().await.to_vec()` snapshot is taken at cancel time. Does this INCLUDE the in-flight `tool_calls` and `tool_results` from the local vecs? Looking at streaming.rs:441:
```rust
chat_history.write().await.push(current_prompt.clone());
```
The prompt IS pushed. But the tool_calls/tool_results are NOT pushed until lines 605-633 (after the while loop). So the snapshot likely does NOT contain the in-flight state.

**This means we need a change to rig's patch itself** — before yielding PromptCancelled at sites 486, 509, and 542, flush the local `tool_calls` and `tool_results` vecs into `chat_history` so the snapshot includes them.

### Step 2 — Modify rig patch

`codelet/patches/rig-core/src/agent/prompt_request/streaming.rs` — before each cancel-yield:

```rust
// Before yielding PromptCancelled at site 486 (after on_tool_call):
if cancel_signal.is_cancelled() {
    // Flush any pending tool_calls to chat_history so caller sees them
    if !tool_calls.is_empty() {
        let assistant_msg = Message::Assistant {
            id: None,
            content: OneOrMany::many(tool_calls.clone()).unwrap(),
        };
        chat_history.write().await.push(assistant_msg);
    }
    return Err(StreamingError::Prompt(
        PromptError::prompt_cancelled(chat_history.read().await.to_vec()).into()
    ));
}
```

Similar patches at sites 509, 542.

### Step 3 — Reconcile session.messages with rig's chat_history

In the compaction-cancel branch of stream_loop.rs:

```rust
if let Some(rig_chat_history) = extract_prompt_cancelled(&e) {
    // Reconcile: rig may have tool_calls/tool_results that fspec's session.messages
    // doesn't have yet. Take the union (deduplicate by content).
    reconcile_session_messages(&mut session.messages, rig_chat_history);
    
    // Now session.messages is consistent — either all tool_calls have matching
    // tool_results, or neither is present.
    ...
}
```

The `reconcile_session_messages` helper detects orphan tool_calls in `session.messages` (calls without results), looks them up in `rig_chat_history`, and pulls in the matching tool_result. Conversely, if rig has a tool_call that fspec doesn't, fspec must add both call and result.

### Step 4 — Defensive orphan-tool-call detection

Even after the fix above, add a validation step before compaction runs:

```rust
fn validate_no_orphan_tool_calls(messages: &[Message]) -> Result<(), Vec<String>> {
    let mut pending_tool_call_ids: HashSet<String> = HashSet::new();
    let mut orphans = Vec::new();
    
    for msg in messages {
        match msg {
            Message::Assistant { content, .. } => {
                for item in content.iter() {
                    if let AssistantContent::ToolCall(tc) = item {
                        pending_tool_call_ids.insert(tc.call_id.clone());
                    }
                }
            }
            Message::User { content } => {
                for item in content.iter() {
                    if let UserContent::ToolResult(tr) = item {
                        pending_tool_call_ids.remove(&tr.id);
                    }
                }
            }
        }
    }
    
    if !pending_tool_call_ids.is_empty() {
        return Err(pending_tool_call_ids.into_iter().collect());
    }
    Ok(())
}
```

Run this at the start of `execute_compaction`. If there are orphan tool calls, inject a synthetic tool_result (e.g., `{"status": "cancelled_by_context_limit"}`) rather than proceeding with a broken history. This complements PROV-050.

## Acceptance Criteria

1. When PromptCancelled fires at rig site 509 (after on_tool_result) with N tool_calls and N tool_results in rig's local vecs → after fspec's recovery, `session.messages` contains all N pairs.
2. When PromptCancelled fires at rig site 486 (after on_tool_call) with N tool_calls but 0 tool_results → fspec's recovery injects synthetic tool_results before compaction runs.
3. `validate_no_orphan_tool_calls` is called at the start of `execute_compaction` and passes for all recovery paths.
4. The error variant's `chat_history` field is no longer dropped — it's consulted by `extract_prompt_cancelled` (CMPCT-025) and used by the recovery flow.

## Files to Modify

- `codelet/patches/rig-core/src/agent/prompt_request/streaming.rs` (cancel-flush at sites 486, 509, 542)
- `codelet/cli/src/interactive/stream_loop.rs` (reconciliation call in compaction-cancel branch)
- `codelet/cli/src/interactive_helpers.rs` (`validate_no_orphan_tool_calls` and `execute_compaction` guard)

## Relationship to PROV-050

PROV-050 proposes `find_safe_split_point()` during summarization to preserve tool-pair invariants. This card addresses the UPSTREAM source of broken pairs (PromptCancelled mid-tool). PROV-050 is the downstream defensive measure. Both are needed.

## Testing

- Integration test: mock stream that emits `[ToolCall, ToolResult, Err(PromptCancelled)]` → verify `session.messages` after recovery contains the tool pair.
- Integration test: mock stream that emits `[ToolCall, Err(PromptCancelled)]` (cancel before result) → verify synthetic tool_result is injected OR the tool_call is removed (pick one policy, test the chosen behavior).
- Unit test: `validate_no_orphan_tool_calls` flags orphan calls correctly.
