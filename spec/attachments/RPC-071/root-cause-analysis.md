# RPC-071 — Root Cause Analysis

> AgentView leaks raw Rust `Debug`-printed `StreamChunk` variants into the user-visible scrollback because `chunk_to_lines` has a catch-all `format!("{other:?}")` arm.

---

## 1. Observed Symptom

**Date observed:** 2026-05-27
**Branch:** `codelet-integration`
**Binary:** Rust `fspec` (built from `codelet/fspec`)
**Reproduction:** Open a DONE work unit's Work Agent, type a message, press Enter.

Rendered scrollback (verbatim, from screenshot `Screenshot 2026-05-27 at 8.46.38 am.png`):

```
user> please review this card
UserInput { text: "please review this card" }
SessionStateChange { state: Running }
SessionStateChange { state: Idle }
```

**Expected scrollback (parity with TS Ink frontend):**

```
user> please review this card
```

The other three lines should not be present at all — they are pure state-mutation
chunks that drive the footer / status pill / dialogs, never the conversation
scrollback.

---

## 2. Faulty Code

### 2.1 Primary defect

`codelet/fspec-tui/src/store/agent_view/session_context.rs:101-111`

```rust
/// Convert a `StreamChunk` into pre-rendered scrollback lines.
fn chunk_to_lines(chunk: &StreamChunk) -> Vec<Line<'static>> {
    let body: String = match chunk {
        StreamChunk::Text { text, .. }            => format!("assistant> {text}"),
        StreamChunk::Thinking { thinking, .. }    => format!("(thinking) {thinking}"),
        StreamChunk::UserNotification { message, ..} => format!("[notice] {message}"),
        StreamChunk::Error { error }              => format!("[error] {error}"),
        StreamChunk::Done                         => "[done]".to_string(),
        other                                     => format!("{other:?}"),  // ← BUG
    };
    vec![Line::from(Span::raw(body))]
}
```

The catch-all `other => format!("{other:?}")` arm uses Rust's built-in `Debug`
formatter on the entire `StreamChunk` enum variant. That's why the user sees
literal Rust syntax like `UserInput { text: "..." }` and
`SessionStateChange { state: Running }` in their conversation.

### 2.2 Compounding defect

`codelet/fspec-tui/src/app/dispatch.rs:31` calls `record_chunk` for **every**
incoming chunk, and `record_chunk` unconditionally pushes whatever
`chunk_to_lines` returns into scrollback:

```rust
// codelet/fspec-tui/src/store/agent_view/session_context.rs:70-75
pub fn record_chunk(&mut self, chunk: &StreamChunk) {
    let seq = self.scrollback_next_seq;
    self.scrollback_next_seq = self.scrollback_next_seq.saturating_add(1);
    let lines = chunk_to_lines(chunk);          // ← always 1+ lines
    self.scrollback.push(RenderedChunk { seq, lines });
}
```

This means the seven RPC-045 state-only chunks (`SessionStateChange`,
`IsolationStateChange`, `DebugStateChange`, `FooterStateUpdate`,
`FspecCommandRequest`, `SupervisorPendingInjection`, `CompactionComplete`) get
**double-handled**:

1. `record_chunk` runs first and Debug-dumps them into scrollback (wrong).
2. `App::handle_stream_chunk_state_updates` runs after and mutates store state
   (correct).

The TS frontend doesn't have this problem because its renderer is explicitly
opt-in — only `UserInput`, `Text`, `Thinking`, `ToolCall`, `ToolResult`,
`UserNotification`, `Done`, `Error` produce conversation messages. Everything
else updates state silently.

---

## 3. Why RPC-045 Missed This

**RPC-045's acceptance** (`spec/work-units.json`, RPC-045 description) said:

> AgentView: subscribe to chunks + status broadcasts; **handle every new
> StreamChunk variant**

The card landed `handle_stream_chunk_state_updates` in `dispatch_rpc045.rs`,
which correctly branches on the seven new state variants and writes the
corresponding store state. But the card never updated `chunk_to_lines` — the
catch-all `format!("{other:?}")` arm was left in place from before RPC-045
introduced any of the new variants.

The unit tests in `session_context.rs` cover only `Text` and the chunk-count
invariant, so the regression was undetectable at unit-test scope. The
cross-transport parity tests at `codelet/fspec-tui/tests/rpc026_*` exercise
the dispatch path but assert on store state, not on rendered scrollback.

---

## 4. TS Ink Parity Reference

The TypeScript Ink AgentView is the canonical reference for what should and
shouldn't appear in scrollback.

### 4.1 `UserInput` → friendly `user> {text}` line

`src/tui/utils/chunkProcessor.ts:227-232`

```typescript
for (const chunk of chunks) {
  if (chunk.type === 'UserInput' && chunk.text) {
    messages.push({
      type: 'user-input',
      content: chunk.text,
    });
  } else if (chunk.type === 'IncomingMessage' && chunk.text) {
    // ...
  }
}
```

`src/tui/components/AgentView.tsx:291` and `:3501` mirror the same pattern in
the realtime chunk stream — `UserInput` produces a `user-input` message, NEVER
a Debug dump.

### 4.2 State chunks → store mutation, no scrollback line

The TS frontend dispatches the state chunks (`SessionStateChange`,
`IsolationStateChange`, etc.) into its session store, which drives:

- The status pill in the header
- The footer's CWD / branch display
- The pause / HITL dialogs
- The isolation banner

None of these chunks appear as a conversation line. The Rust side already
performs the equivalent store mutations in `handle_stream_chunk_state_updates`,
so the only missing piece is suppressing the (wrong) scrollback push.

---

## 5. Variant-by-Variant Audit

`StreamChunk` (defined in `codelet/rpc-types/src/lib.rs:1000-1110`) has 23
variants. This is the complete contract `chunk_to_lines` MUST honour after the
fix.

| Variant | Disposition | Rendered as |
|---|---|---|
| `Text { text, .. }` | **Show** | `assistant> {text}` |
| `Thinking { thinking, .. }` | **Show** | `(thinking) {thinking}` |
| `ToolCall { tool_call, .. }` | **Show** (delegated to richer renderer) | TBD (parity with TS `tool-call` message) |
| `ToolResult { tool_result, .. }` | **Show** (delegated) | TBD (parity with TS `tool-result` message) |
| `ToolProgress { tool_progress, .. }` | **Show** (delegated) | TBD (parity with TS `tool-progress` message) |
| `SessionStateChange { state }` | **Suppress** | (none — store update only) |
| `UserNotification { message, severity }` | **Show** | `[notice] {message}` |
| `Interrupted { queued_inputs }` | **Show** | `[interrupted] queued: {queued_inputs.len()}` |
| `TokenUpdate { tokens }` | **Suppress** | (none — store update only) |
| `ContextFillUpdate { context_fill }` | **Suppress** | (none — store update only) |
| `Done` | **Show** | `[done]` |
| `Error { error }` | **Show** | `[error] {error}` |
| `UserInput { text }` | **Show** | `user> {text}` |
| `IncomingMessage { text, images }` | **Show** | `supervisor> {text}` (parity with TS `IncomingMessage` arm) |
| `SupervisorPendingInjection { .. }` | **Suppress** | (none — store update only) |
| `CompactionComplete { compaction_result }` | **Suppress** | already emitted via `EmitSessionNotice` |
| `FspecCommandRequest { fspec_request }` | **Suppress** | (none — store update only) |
| `FspecCommandResult { fspec_result }` | **Suppress** | (none — store update only) |
| `WorkUnitsUpdate { work_units }` | **Suppress** | (none — BoardView consumes this) |
| `IsolationStateChange { .. }` | **Suppress** | (none — store update only) |
| `FooterStateUpdate { .. }` | **Suppress** | (none — store update only) |
| `DebugStateChange { enabled }` | **Suppress** | (none — store update only) |

**Ratio: 8 visible / 15 silent. The current code shows 5 of the 8 visible
ones and Debug-dumps all 15 silent ones.**

The three `ToolCall` / `ToolResult` / `ToolProgress` variants currently fall
through the catch-all too — they have always been wrong, but the new RPC-045
state variants made the regression user-visible. A separate richer renderer
(parity with the TS Ink tool message components) is out of scope for this
card and will be tracked as a follow-up.

---

## 6. Why the Session Also Produces No Assistant Output

Note that the same screenshot shows the session going `Running → Idle`
**without** any assistant `Text` or `Done` chunks. That's a separate
defect — the agent loop is a no-op in the fspec binary because
`NoopSessionManagerHooks` discards the input channel. That's tracked as
**RPC-072**, not as part of this card. **RPC-071 fixes only the rendering
contract.**

After RPC-071 lands, the user will still see only `user> please review this
card` and nothing else — but at least the screen won't be polluted with
Rust internals. RPC-072 then makes the agent actually reply.

---

## 7. Test Strategy

See `test-plan.md` in this attachment directory for the full plan. Summary:

- A new `session_context_chunk_to_lines.rs` test seeds every `StreamChunk`
  variant and asserts the rendered scrollback contains exactly the eight
  visible lines in the right order, with the right prefixes.
- A regression integration test in `codelet/fspec-tui/tests/` opens an
  AgentView via the same dispatch path the production binary uses, feeds in
  the four chunks from the screenshot, and asserts the rendered buffer
  contains only `user> please review this card`.
- The catch-all arm is replaced with `debug_assert!(false, "...")` so any
  future `StreamChunk` variant additions panic during `cargo test` instead
  of silently leaking into scrollback.
