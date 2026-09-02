# Research: Unrecoverable API Errors — Failed Tool Calls Remain in the Context Window

**Date:** 2026-09-02
**Requested by:** rquast
**Scope:** `rust/cli` (stream loop / recovery), `rust/core` (compaction hook, persistence), `rust/agent-loop` (persistence + dispatch), `rust/patches/rig-core` (multi-turn streaming)

---

## 1. Problem Statement

When an unrecoverable (terminal) API error occurs — most commonly a **failing/oversized tool call** (a huge tool result or oversized tool arguments that push the next request past the model's context window) — the last message in the message history is **not** removed from the context window stack.

Consequences:

1. The failed tool call (+ its result) stays in `session.messages` (in-memory) **and** in the on-disk session manifest (persisted immediately).
2. The **next** user message re-sends the entire history, replaying the same failed tool call. The same unrecoverable error recurs every turn.
3. Cache-token bookkeeping (Anthropic `cache_read_input_tokens` / `cache_creation_input_tokens`, per-request fields in `TokenState` and `TokenTracker`) is **stale** after a failed request — nothing invalidates or zeroes it. (There is no client-side "cache handle" to invalidate; provider caches are content-addressed — see §6.)
4. The pre-prompt compaction check (the safety net that should prevent this) is **gated on the presence of compactable turns**, which fails exactly in the "failed tool call is all that's left" case (see §3, finding F3).

The desired behavior: on an unrecoverable API error, **pop the offending tool call (and its matching tool result, if present) off the context stack**, invalidate stale cache-token state, and ensure the removed message does not get replayed.

---

## 2. How a Turn Flows Today (the "context window stack")

The live context is `session.messages: Vec<rig::message::Message>` (`rust/cli/src/session/mod.rs`). A single agent turn produces, in order:

| Step | Code | What happens to `session.messages` |
|------|------|-----------------------------------|
| T0 | `agent_loop.rs:328` | `persist_user_message(&session.id, input)` — user prompt written to **disk** (manifest + append-only `messages.jsonl`) |
| T1 | `stream_loop.rs:299` | **Pre-prompt compaction check** (`estimated_total > threshold && has_turns_to_compact`) → `execute_compaction` |
| T2 | `stream_loop.rs:373` | Fresh `TokenState { input_tokens: session.token_tracker.input_tokens, cache_read/creation: 0, output_tokens: tracker.output_tokens, compaction_needed: false }` + `CompactionHook` (threshold from model context window) |
| T3 | `stream_loop.rs:480-490` | `agent.prompt_streaming_with_history_and_hook(prompt, &mut session.messages, hook)` — rig clones the history; then the user prompt is pushed: `session.messages.push(Message::User { content: build_user_content_with_images(...) })` |
| T4 | `stream_loop.rs:~900` | ToolCall chunk → `handle_tool_call`: call is **buffered** in `tool_calls_buffer` (NOT in `session.messages` yet); `tool_execution_in_progress = true` |
| T5 | `stream_loop.rs:947-956` | ToolResult chunk → `handle_tool_result`: **flushes `tool_calls_buffer` as an `Assistant` message, then pushes `Message::User { content: UserContent::ToolResult }`**. This is the point where the tool-call pair enters the context stack. `BackgroundOutput::emit(ToolResult)` (agent-loop) **persists both halves to disk immediately** |
| T6 | rig (`patches/rig-core/.../streaming.rs`) | Next API call. `CompactionHook::on_completion_call` runs **before** the call: `effective_total = max(last_known_total, estimated_payload) > threshold` → `cancel_sig.cancel()` → rig yields `PromptError::PromptCancelled { chat_history }` |
| T7 | `stream_loop.rs:1688` | `classify_compaction_branch` routes `PromptCancelled` → in-loop compaction (Paths B/C/D) |
| T8 | Terminal error arm (`stream_loop.rs:2082-2115`) | `output.emit_error(&error_str)` + `return Err(anyhow!("Agent error: {e}"))` — **no message removal, no cache-token invalidation** |

Key invariant (CLI-008): the user prompt and the tool-result half are pushed to `session.messages` **after** rig has cloned the history, so rig never sees a double-sent message — but it also means the failed content is already in the live stack by the time a terminal error returns.

---

## 3. Current Error-Classifier Cascade (what already exists)

`stream_loop.rs` error arm (`Some(Err(e))` at line 1670) runs this cascade, in order:

| # | Classifier | Location | Recovery action | Pops last message? |
|---|-----------|----------|-----------------|--------------------|
| 1 | `classify_compaction_branch` (structural `PromptError::PromptCancelled` downcast) | `error_classifiers.rs:256` | `begin_compaction_recovery` (`pop_user_prompt=true` for Paths B/C) → `in_loop_compaction_restart!` (full DAG compaction, bounded by `MAX_COMPACTION_RETRIES = 3`) | Pops trailing **User** only; compaction wipes conversation |
| 2 | `is_stall_timeout_error` | `error_classifiers.rs:113` | **Terminal** — never retried | No |
| 3 | `is_prompt_too_long_error` | `error_classifiers.rs:15` | Compaction recovery (Path B) — **but only if `has_compactable_turns`** | (see finding F3) |
| 4 | `is_image_content_error` (EXT-016) | `error_classifiers.rs:39` | Pops trailing User, sanitizes images from history, `break` (session stays usable) | Yes (User only) |
| 5 | `is_truncated_tool_call_error` (PROV-040) | `error_classifiers.rs:61` | Retry with recovery prompt (≤ `MAX_TRUNCATION_RETRIES`) | No |
| 6 | `is_transient_network_error` (NET-001) | `error_classifiers.rs:73` | Retry with `"Continue"` (≤ `MAX_NETWORK_RETRIES = 3`, 1s/2s/4s backoff) | No |
| 7 | (else) **Terminal** | `stream_loop.rs:2082-2115` | Emit error, `return Err` | **No — nothing is removed** |

### Finding F1 — The terminal path removes nothing

`stream_loop.rs:2082-2115` (the fall-through terminal arm):

```rust
output.emit_error(&error_str);
return Err(anyhow::anyhow!("Agent error: {e}"));
```

`session.messages` is returned to the agent loop with **every message intact** — including a trailing
`Assistant(ToolCall)` + `User(ToolResult)` pair if the tool result was the content that broke the context.
The only `session.messages.pop()` calls in the whole interactive module are:

- `recovery_compaction.rs:282` — `begin_compaction_recovery` (`pop_user_prompt=true`), pops trailing **User**
- `stream_loop.rs:1867` — EXT-016 image sanitization, pops trailing **User**

Neither ever removes a **tool call** or a **tool result**. There is no `remove_message`, `truncate_messages`,
or tail-pop helper anywhere in the workspace (verified by search: zero matches for
`remove_message|truncate_messages|pop_message|remove_last`).

### Finding F2 — The compaction path is exactly the case the fix targets… and it works

When the *next* API call after a tool result is cancelled by the hook (`PromptCancelled`) or the API
answers "prompt is too long", the existing machinery is robust:

- `begin_compaction_recovery` pops the trailing User prompt (the one not yet consumed by the API),
  flushes partial assistant text, flushes the token tracker, sets `compaction_needed`, emits lifecycle events.
- `execute_compaction` (`interactive_helpers.rs:544`) refuses to run with orphan tool_calls
  (`validate_no_orphan_tool_calls`), and the Path C arm first reconciles with rig's `chat_history`
  payload and injects synthetic `cancelled_by_context_limit` tool_results for dangling calls
  (`inject_synthetic_tool_results_for_orphans`).
- `reset_session_to_reminders` wipes the conversation (system reminders survive), then
  `recalculate_token_tracker` rebuilds `token_tracker.input_tokens` from scratch.

So the in-loop compaction cascade *would* correctly handle a failed tool call — **if it is reached**.

### Finding F3 — The compaction gate fails precisely when a failed tool call is the whole problem

Path B's guard (`stream_loop.rs:1810-1813`):

```rust
let has_compactable_turns = !convert_messages_to_turns(&session.messages).is_empty();
if is_prompt_too_long && has_compactable_turns { ... compaction ... }
// else falls through — to terminal error
```

`convert_messages_to_turns` only counts `(User, Assistant)` **adjacent pairs**. When the tail of the
stack is `... Assistant(ToolCall) → User(ToolResult)` (the failed tool call) and nothing follows it,
that pair is *not* a User/Assistant pair. In sessions where the offending tool call dominates the
context (the reported scenario — "it's usually a failing tool call more than anything else"), this
guard can evaluate `false` **even though compaction would trivially succeed** (the tool result is
compactable content). The error then falls through to the terminal arm and the failed tool call
persists — the replay loop.

Note the pre-prompt check (T1) has the *same* gate (`has_turns_to_compact`), so even the
"pre-prompt compaction" safety net skips the session for the same reason.

### Finding F4 — Interrupt (Esc) also leaves the tail intact

The interrupt break (`stream_loop.rs:741-758`) appends any partial assistant text and breaks. A pending
`tool_calls_buffer` (tool was mid-execution) is **not** drained or closed; the assistant text is pushed
after the trailing user prompt, leaving a stack that ends `... User(prompt) Assistant(partial text)`
with no tool-result — which is *valid* for the API (no dangling tool_use), but the next turn replays the
whole segment. This is out of scope for this card but worth noting: the same tail-surgery helper would
apply.

### Finding F5 — Retry paths re-push synthetic prompts instead of removing the failure

- NET-001 retry (line 2035-2046): restarts with `"Continue"` and **pushes** a `User("Continue")` message.
- PROV-040 truncation retry (line 1929-1941): pushes a recovery-prompt User message.
- PROV-041 thinking-exhaustion retry (line 1367-1378): pushes a recovery message.

These grow the stack instead of shrinking it. Acceptable while the original request is the issue (the
content is legitimately part of the conversation), but combined with F1 they guarantee that once a
terminal error fires, the stack only ever gets bigger.

---

## 4. Cache Tokens — What Exists and What "Invalidation" Means

### 4.1 There is no client-side cache store

Provider prompt caching is **content-addressed and server-side**:

- **Anthropic** — `caching_client.rs` rewrites the request body before send: system prompt becomes an
  array with `cache_control: {"type":"ephemeral"}` blocks, and the **last message's final block** gets
  `cache_control` (incremental caching). The cache lives in Anthropic's backend keyed on the content
  prefix. A `cache_read_input_tokens`/`cache_creation_input_tokens` value is *metadata about the last
  request*, not a handle. Changing any earlier byte of the history naturally invalidates downstream
  cache entries; there is nothing to call to "invalidate".
- **OpenAI-compatible** — `cache_optimization.rs` (PROV-051) only sends an `x-session-affinity` header
  for custom base URLs; again server-side.

**Conclusion: "invalidating cache tokens" in this codebase means resetting the *tracked* cache-token
fields so that (a) the next turn's display/estimates aren't seeded from stale values, and (b) the
compaction hook doesn't make decisions on stale `cache_read_input_tokens`.**

### 4.2 Where cache tokens live

| Store | Fields | Lifecycle |
|-------|--------|-----------|
| `TokenState` (`core/compaction_hook.rs:18`) | `input_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`, `output_tokens`, `compaction_needed` | **Per turn** — recreated at `stream_loop.rs:373` and after every compaction retry (`stream_loop.rs:673-679`). Updated by the hook after each API call (`on_stream_completion_response_finish`). **Never touched on error** |
| `TokenTracker` (`core/compaction/model.rs:57`) | same fields (cache fields `Option<u64>`) + cumulative billing + session-cumulative `reasoning_tokens` | **Per session, in-memory** (`session.token_tracker`). Updated via `update_from_usage` on successful responses only. `reset_after_compaction()` zeroes `output_tokens` + cache fields. **Never reset on error** |
| `StreamingTokenDisplay` | display-side mirror | Re-seeded from tracker on every retry |
| Manifest `TokenUsage` (`core/persistence/manifest.rs:44`) | `current_context_tokens`, `cumulative_billed_*`, `cache_read_tokens`, `cache_creation_tokens`, `reasoning_tokens` | On-disk. `persist_token_state` runs **only on `StreamEvent::Done`** (background_output.rs:411-420) — a terminal error emits `Error`, so stale values are simply not rewritten; they persist from the *last successful* turn |
| Per-message `TokenUsagePerMessage` (`persistence/message_envelope.rs:152`) | usage embedded in assistant envelopes | Written at persistence time (assistant only) |

### 4.3 Finding F6 — Stale cache-token state survives a failed request

Because a failed API call yields no usage, the tracker keeps the *last successful request's* values.
That is mostly harmless while the history is unchanged, but after the proposed tail-surgery **the
history changes** — so:

- `session.token_tracker.input_tokens` no longer reflects the real prefix size.
- `cache_read_input_tokens` / `cache_creation_input_tokens` reference a prefix that no longer exists
  (the cache entry was broken by our own edit).
- The pre-prompt estimate (T1: `current_tokens + prompt_tokens`) will be overstated; the hook's
  `max(last_known_total, estimated_payload)` likewise.

Existing precedent for the fix: `reset_after_compaction` (`compaction/model.rs:210`) zeroes the cache
fields, and `recalculate_token_tracker` (`interactive_helpers.rs:205`) recomputes `input_tokens` from
the actual messages. The tail-surgery path should do the same (or at least zero the cache fields and
recompute input), or seed the next `TokenState` from `from_cache_inclusive_total` of a recomputed
total.

### 4.4 Finding F7 — Per-message cache usage is not rehydrated on resume

`get_session_message_envelopes` rehydrates `message.content` from metadata but the per-message
`usage` block is dropped; `restore_session_token_state` re-seeds from manifest `TokenUsage`
(`handle_impl.rs:211-218`). So the cache-token picture after `/resume` is already "last successful
request only" — consistent with treating these fields as per-request, and consistent with §4.1's
conclusion that invalidation = zeroing the tracked fields.

---

## 5. On-Disk Persistence — the Second Half of the Bug

- **User prompt:** written at turn start, before any API call (`agent_loop.rs:328` →
  `append_message_with_metadata`, `persistence/manifest.rs:882`).
- **Assistant content + tool result:** written **as they stream** — `BackgroundOutput::emit(ToolResult)`
  persists the assistant message then the tool result (`background_output.rs:296-306`); the NAPI twin
  does the same (`napi/src/agent_loop.rs`).
- The message store is **append-only** (`messages.jsonl` + binary index). `MessageStore` exposes
  `store / store_with_metadata / get / update_metadata / cleanup_orphans` — **no delete/remove API**.
- Session ordering lives in the manifest: `SessionManifest.messages: Vec<MessageRef>`
  (`persistence/manifest.rs:106`). `add_message` is the only mutator; there is no `remove_message`.
- Resume replays **every** `MessageRef` in the manifest (`get_session_message_envelopes` →
  `restore_session_messages`, `handle_impl.rs:471`).

**Finding F8:** To stop the failed tool call from being replayed *after a process restart*, the fix
must either (a) remove/neutralize the corresponding `MessageRef`s from the manifest (a new
`remove_last_messages`-style primitive), or (b) tag the tool_result envelope's metadata with a
`removedFromContext: true` marker that `restore_session_messages` skips (mirrors the existing
system-reminder skip rule at `handle_impl.rs:480-482`). Option (a) is cleaner; it leaves
`cleanup_orphaned_messages` to reclaim the blobs later.

---

## 6. Exact Tail States at a Terminal Error (what to pop)

At the terminal arm, `session.messages` can end in any of these shapes (system reminders always at
the front):

| Tail shape | When | Removal to make the next request safe |
|------------|------|----------------------------------------|
| `… User(prompt)` | API rejected the request before streaming (most "prompt too long" cases) | Pop the User prompt (existing `begin_compaction_recovery` semantics) |
| `… Assistant(ToolCall)` | Hook cancelled or rig aborted between the ToolCall chunk and tool execution (site-486 shape); `tool_calls_buffer` may still hold it | Remove/strip the ToolCall item; drain+discard `tool_calls_buffer` |
| `… Assistant(ToolCall) User(ToolResult)` | **The reported case** — tool executed, result landed (T5), next API call failed unrecoverably | Remove the User(ToolResult) **and** the ToolCall item from the preceding Assistant message |
| `… Assistant(Text+ToolCall) User(ToolResult)` | Same, but the assistant turn also carried text | Strip only the ToolCall item(s) from the Assistant message; keep the Text; remove the ToolResult message |
| `… User(prompt) Assistant(partial)` | Interrupted turn (F4) | Out of scope; noted |

Constraints the removal helper must respect:

1. **Tool-pair invariant** — after removal, no Assistant ToolCall may lack a ToolResult
   (`validate_no_orphan_tool_calls` is the existing guard; the removal must *produce* a valid stack,
   not just move the failure).
2. **Never remove a system reminder** — reminders persist through compaction and are re-injected on
   clear; partitioning logic (`system_reminders.rs::partition_for_compaction`) already identifies them.
3. **Idempotent + defensive** — if the tail doesn't match the expected shape (e.g. tail is a plain
   Text assistant message), degrade to "pop trailing User" rather than corrupting the stack.
4. **Recompute token state** after removal (F6): recompute `input_tokens`, zero cache fields, reset
   `output_tokens` per-turn bookkeeping — reuse `recalculate_token_tracker` + cache-zeroing pattern.
5. **Persist the removal** (F8) so resume doesn't replay the pair.

---

## 7. Recommended Fix (shape, not implementation)

A new recovery primitive, e.g. `recovery_unrecoverable.rs` in `rust/cli/src/interactive/`:

```text
fn strip_failed_tool_call_tail(
    messages: &mut Vec<Message>,
    tool_calls_buffer: &mut Vec<AssistantContent>,
) -> StrippedTail {
    // 1. Discard tool_calls_buffer (unexecuted call that never got a result)
    // 2. If tail == User(ToolResult): pop it, then remove its ToolCall item from the
    //    preceding Assistant message (strip, keeping Text/other items; drop the whole
    //    Assistant message if it becomes empty)
    // 3. Else if tail == Assistant with only ToolCall items: pop it
    // 4. Else: pop trailing User(prompt) only (existing semantics)
    // 5. Run validate_no_orphan_tool_calls as a postcondition
    // Returns which shape was stripped (for logging / cache-token handling)
}
```

Plus, in the terminal error arm (and optionally at the F3 gate failure of Path B):

1. `strip_failed_tool_call_tail(...)` — surgical removal instead of full compaction.
2. **Cache-token invalidation:** zero `TokenState`/`TokenTracker` cache fields (mirror
   `reset_after_compaction`) and recompute `input_tokens` from the trimmed messages (mirror
   `recalculate_token_tracker`), so the next turn's pre-prompt estimate and hook seed are honest.
3. **Persistence:** add a `MessageStore`/manifest primitive to drop the tail `MessageRef`s (or the
   marker-skip variant) and call it from the agent-loop error handler where the session id is in
   scope (`agent_loop.rs:1483-1496` — the `if let Err(e) = result` arm).
4. **Gate fix (F3):** relax `has_compactable_turns` to also count a trailing tool-result as
   compactable content — or, better, route "prompt too long + tail is a failed tool pair" directly
   to the surgical strip (cheaper than full DAG compaction for the dominant case).
5. Keep the error surfaced to the user; the session remains interactive (like the EXT-016 image
   recovery `break`), so the user's next message starts from a clean stack.

Testing strategy (per repo standards): pure-function tests for `strip_failed_tool_call_tail` over the
five tail shapes + system-reminder protection + orphan postcondition (proptest for random stacks is a
good fit); an integration test in `rust/cli/tests/` (stub provider scripted to fail after a tool
result) asserting the next turn's outgoing history does not contain the stripped pair; a persistence
test asserting the manifest tail after the strip (temp-dir based, `rust/test-helpers`).

---

## 8. Open Questions

1. **Surgical strip vs full compaction:** should a failed *huge tool result* that is >~10% of the
   context trigger full DAG compaction instead of just pair-removal? (Pair removal alone may leave a
   context that is still over threshold — in which case the existing hook/Path B catches it next call.)
   Proposal: strip first; let the existing threshold machinery handle the rest.
2. **Which unrecoverable errors qualify?** "prompt is too long" and `context_length_exceeded` are
   clear. Should image-content 400s (EXT-016) reuse the same helper? (Today it pops User then
   sanitizes; it could strip a failed image-carrying tool result too.)
3. **Persist-strip semantics for fork/merge:** removing `MessageRef`s changes manifest length;
   `MessageRef` indexes are used by `CompactionState.compacted_before_index`
   (`persistence/manifest.rs:95`). The strip must either (a) not shift compacted-prefix indexes
   (only ever trim the *tail* after `compacted_before_index`) or (b) shift the index. Confirm tail-only
   trimming is always safe (it should be: the compacted prefix is at the front).
4. **Interrupt path (F4):** should Esc mid-tool also run the strip? Defer to a follow-up.
5. **`session.turns`:** vestigial (never populated in the CLI path — only cleared). The removal
   helper should not bother with it; confirm safe to leave.

## 9. File Inventory (primary)

- `rust/cli/src/interactive/stream_loop.rs` — error cascade, terminal arm (2082-2115), tail push (488)
- `rust/cli/src/interactive/error_classifiers.rs` — cascade classifiers
- `rust/cli/src/interactive/recovery_compaction.rs` — `begin_compaction_recovery` (existing pop at 282)
- `rust/cli/src/interactive/stream_handlers.rs` — `handle_tool_call` (buffer) / `handle_tool_result` (pair push)
- `rust/cli/src/interactive_helpers.rs` — `execute_compaction`, `recalculate_token_tracker`,
  `validate_no_orphan_tool_calls`, `inject_synthetic_tool_results_for_orphans`, `reconcile_session_messages`
- `rust/core/src/compaction_hook.rs` — `TokenState`
- `rust/core/src/compaction/model.rs` — `TokenTracker`, `reset_after_compaction`
- `rust/providers/src/caching_client.rs` — Anthropic cache_control request rewriting
- `rust/agent-loop/src/persist.rs` + `background_output.rs` — per-message persistence timing
- `rust/core/src/persistence/manifest.rs` / `messages.rs` — manifest `MessageRef` list, append-only store
- `rust/sessions/src/handle_impl.rs` — resume/restore (envelope replay)
