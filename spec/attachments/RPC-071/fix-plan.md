# RPC-071 — Fix Plan

> Concrete, file-by-file changes required to satisfy the acceptance criteria.

---

## 1. Strategy

**Approach: Option B — make `chunk_to_lines` return `Option<Vec<Line>>` and
remove the catch-all arm.**

Alternatives considered:

| Option | Description | Verdict |
|---|---|---|
| **A** | Whitelist visible variants, keep catch-all that returns empty `Vec`. | Rejected — silent failure mode. A new `StreamChunk` variant could be added in `codelet-rpc-types` and never get a scrollback rendering, with zero compile-time signal. The whole reason RPC-045 introduced this bug was a silent catch-all. |
| **B** | Return `Option<Vec<Line>>`, exhaustively enumerate every variant (no catch-all). | **Chosen** — compiler enforces completeness. New variants fail `cargo build` until explicitly classified as visible / silent. |
| **C** | Split `chunk_to_lines` into `chunk_to_lines(...) -> Vec` and a separate `is_visible_chunk(...) -> bool` predicate. | Rejected — two functions to keep in sync, easy to skew. |

---

## 2. Files to Change

### 2.1 `codelet/fspec-tui/src/store/agent_view/session_context.rs`

**Change 1: Rewrite `chunk_to_lines` to return `Option<Vec<Line<'static>>>`
and exhaustively match every variant. No catch-all `other =>` arm.**

Before:

```rust
fn chunk_to_lines(chunk: &StreamChunk) -> Vec<Line<'static>> {
    let body: String = match chunk {
        StreamChunk::Text { text, .. } => format!("assistant> {text}"),
        StreamChunk::Thinking { thinking, .. } => format!("(thinking) {thinking}"),
        StreamChunk::UserNotification { message, .. } => format!("[notice] {message}"),
        StreamChunk::Error { error } => format!("[error] {error}"),
        StreamChunk::Done => "[done]".to_string(),
        other => format!("{other:?}"),
    };
    vec![Line::from(Span::raw(body))]
}
```

After:

```rust
/// Convert a `StreamChunk` into pre-rendered scrollback lines.
///
/// Returns `None` for pure state-mutation chunks (RPC-045 family +
/// token / context-fill / fspec-tool family) that must NEVER appear
/// in scrollback. See `spec/attachments/RPC-071/ts-parity-reference.md`
/// for the authoritative variant table.
///
/// The match arm is exhaustive — adding a new `StreamChunk` variant
/// to `codelet-rpc-types` will fail to compile until classified here.
fn chunk_to_lines(chunk: &StreamChunk) -> Option<Vec<Line<'static>>> {
    let body: String = match chunk {
        // Conversation lines.
        StreamChunk::UserInput { text }                 => format!("user> {text}"),
        StreamChunk::Text { text, .. }                  => format!("assistant> {text}"),
        StreamChunk::Thinking { thinking, .. }          => format!("(thinking) {thinking}"),
        StreamChunk::IncomingMessage { text, .. }       => format!("supervisor> {text}"),
        StreamChunk::UserNotification { message, .. }   => format!("[notice] {message}"),
        StreamChunk::Error { error }                    => format!("[error] {error}"),
        StreamChunk::Interrupted { queued_inputs }      =>
            format!("[interrupted] {} queued", queued_inputs.len()),
        StreamChunk::Done                               => "[done]".to_string(),

        // State-only chunks — consumed elsewhere, MUST NOT appear in scrollback.
        StreamChunk::SessionStateChange { .. }
        | StreamChunk::IsolationStateChange { .. }
        | StreamChunk::DebugStateChange { .. }
        | StreamChunk::FooterStateUpdate { .. }
        | StreamChunk::FspecCommandRequest { .. }
        | StreamChunk::FspecCommandResult { .. }
        | StreamChunk::WorkUnitsUpdate { .. }
        | StreamChunk::SupervisorPendingInjection { .. }
        | StreamChunk::CompactionComplete { .. }
        | StreamChunk::TokenUpdate { .. }
        | StreamChunk::ContextFillUpdate { .. } => return None,

        // Tool variants — deferred to a richer renderer (RPC-073 follow-up).
        // Suppressed until then so they don't pollute scrollback.
        StreamChunk::ToolCall { .. }
        | StreamChunk::ToolResult { .. }
        | StreamChunk::ToolProgress { .. } => return None,
    };
    Some(vec![Line::from(Span::raw(body))])
}
```

**Change 2: Make `record_chunk` skip the push when `chunk_to_lines` returns
`None`. Also avoid bumping `scrollback_next_seq` on suppressed chunks so the
seq is a true monotonic cursor over visible chunks only.**

Before:

```rust
pub fn record_chunk(&mut self, chunk: &StreamChunk) {
    let seq = self.scrollback_next_seq;
    self.scrollback_next_seq = self.scrollback_next_seq.saturating_add(1);
    let lines = chunk_to_lines(chunk);
    self.scrollback.push(RenderedChunk { seq, lines });
}
```

After:

```rust
/// Append a chunk's rendered lines to this session's scrollback.
/// No-op for state-only chunks per the chunk_to_lines contract.
pub fn record_chunk(&mut self, chunk: &StreamChunk) {
    let Some(lines) = chunk_to_lines(chunk) else { return; };
    let seq = self.scrollback_next_seq;
    self.scrollback_next_seq = self.scrollback_next_seq.saturating_add(1);
    self.scrollback.push(RenderedChunk { seq, lines });
}
```

### 2.2 `codelet/fspec-tui/src/store/agent_view/session_context.rs` (tests)

Add the new test module entries described in `test-plan.md`.

### 2.3 `codelet/fspec-tui/tests/`

Add a new integration test file `chunk_rendering_parity_rpc071.rs` per
`test-plan.md`.

### 2.4 Feature file

Create `spec/features/agentview-chunk-rendering.feature` with the scenarios
generated from this card's Example Map.

---

## 3. Why `record_chunk_appends_and_bumps_seq` Test Will Need Updating

The existing unit test in `session_context.rs:130-136` asserts:

```rust
ctx.record_chunk(&StreamChunk::text("hi".to_string()));
ctx.record_chunk(&StreamChunk::text("there".to_string()));
assert_eq!(ctx.scrollback.chunk_count(), 2);
assert_eq!(ctx.scrollback_next_seq, 2);
```

Still passes after the fix — `Text` chunks are visible, both push, both bump
seq. No change required.

But a **new** test must assert that `record_chunk` with a state-only chunk
leaves `chunk_count` and `scrollback_next_seq` unchanged.

---

## 4. Non-Goals

- **Rich tool-call rendering** — `ToolCall`, `ToolResult`, `ToolProgress` are
  suppressed for now. A follow-up card (RPC-073, to be created if the
  reviewer asks) will introduce a proper tool-message renderer that mirrors
  the TS Ink `<ToolCallMessage>` component.
- **Streaming `Text` merge** — the TS frontend merges consecutive streaming
  `Text` chunks into a single conversation message. The Rust scrollback
  currently emits one line per chunk. That's a known limitation tracked by
  RPC-019 / RPC-029 fallout and is out of scope.
- **Markdown rendering** — out of scope, will be a future card.

---

## 5. Acceptance Walkthrough

When the user types "please review this card" after the fix:

1. `send_input` calls `handle_output(StreamChunk::user_input(text))`.
2. The chunks broadcast emits `UserInput { text: "please review this card" }`.
3. `App::dispatch` routes it through `Action::ChunkReceived`.
4. `record_chunk` calls `chunk_to_lines` → `Some(["user> please review this card"])`.
5. The line is pushed onto scrollback. `seq` becomes 1.
6. Subsequently `SessionStateChange { state: Running }` arrives.
7. `record_chunk` calls `chunk_to_lines` → `None`. No push, no seq bump.
8. `handle_stream_chunk_state_updates` runs and updates the status pill.
9. Same for `SessionStateChange { state: Idle }`.

User sees:

```
user> please review this card
```

— exactly one line, matching the TS Ink frontend byte-for-byte.

---

## 6. Estimated Effort

**2 story points** (Fibonacci):

- ~30 min: rewrite `chunk_to_lines` + `record_chunk` + update unit tests.
- ~30 min: write the integration regression test (`chunk_rendering_parity_rpc071.rs`).
- ~30 min: feature file + Example Mapping + scenario generation.
- ~15 min: `cargo test --workspace` + manual binary smoke test.

Low risk because the change is localised to two functions and the contract
is fully exhaustively-checked by the compiler.
