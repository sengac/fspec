# AST Research: RIG-014 Streaming Loop Detector Integration Points

**Work unit:** RIG-014
**Date:** 2026-08-19
**Method:** AstGrep + Grep over `rust/agent-loop/src` and `rust/cli/src/interactive`

## Goal

Identify the exact code sites where a streaming loop detector must be wired in, and the existing recovery/interrupt machinery it should reuse.

## Findings

### 1. Stream output sink (where deltas are consumed)

`rust/agent-loop/src/background_output.rs:100` — `impl codelet_cli::interactive::StreamOutput for BackgroundOutput`:

```rust
fn emit(&self, event: codelet_cli::interactive::StreamEvent) {
    let chunk = match event {
        StreamEvent::Text(ref text) => { /* accumulate + StreamChunk::text */ }
        StreamEvent::Thinking(ref thinking) => { /* accumulate + StreamChunk::thinking */ }
        StreamEvent::ToolCall(ref tc) => { ... }
        StreamEvent::ToolResult(ref tr) => { ... }
        StreamEvent::Status(status) => { StreamChunk::user_notification(...) }
        ...
    };
}
```

This is the per-session sink that receives EVERY thinking and text delta. It is the natural home for per-(turn, channel) detector state: one `StreamLoopDetector` for the Thinking channel, one for the Text channel, both reset at turn start. `BackgroundOutput` is constructed per-turn at `rust/agent-loop/src/agent_loop.rs:506` (`BackgroundOutput::with_provider(...)`), so detector state naturally scopes to a turn.

### 2. The single streaming choke point (where abort must fire)

`rust/cli/src/interactive/stream_loop.rs` — `run_agent_stream_with_images` (line 189) and the shared `run_agent_stream` (line ~239). ALL providers stream through this one function (dispatched from `rust/agent-loop/src/agent_loop.rs:1013` and the `run_with_provider!` macro arms). It already:

- Checks `is_interrupted: Arc<AtomicBool>` at the top of each iteration (line 734) and on the interrupt path emits `output.emit_interrupted(&queued)` (line 741).
- Uses `interrupt_notify: Arc<Notify>` for immediate wake-up mid-tool-execution via `tokio::select!` (lines 810-815).
- `interrupt()` on the session sets `is_interrupted` (Release) and notifies.

**Integration implication:** the loop detector's ABORT action should drive the EXISTING interrupt machinery — set `is_interrupted` + `interrupt_notify` — rather than inventing a new cancellation path. This reuses the battle-tested cancel/emit-interrupted/stop-reason-propagation code. The detector itself lives in `codelet-agent-loop` (pure, no I/O); the abort callback it hands to the stream loop is a closure that flips the interrupt flag.

### 3. Existing recovery-note precedent to mirror

`rust/cli/src/interactive/recovery_thinking.rs` (PROV-041) is the direct analog for the "corrective note on next turn" requirement:

- `build_thinking_exhaustion_recovery_message(...)` builds a message stating the previous response was interrupted and instructing the model to change behavior, preserving captured reasoning as context.
- The stream loop injects such recovery messages and `continue`s the loop for a retry.

RIG-014's corrective note ("previous response was cut off due to repetitive output; continue with a fresh approach, do not repeat your earlier reasoning") should follow this same builder-function pattern: a pure, public, testable `build_loop_abort_recovery_message(signal, onset_excerpt) -> String`.

### 4. Persistence path (truncation + marker note)

`rust/agent-loop/src/background_output.rs` accumulates `AssistantContent` blocks in a `Mutex<Vec<AssistantContent>>` and persists via `persist_assistant_message()` (called on ToolResult and Done). The truncation requirement (keep pre-onset content, drop degenerate tail, append marker note) is satisfied by stopping accumulation once the detector aborts and appending a marker `AssistantContent::Text` block.

### 5. Crate placement

- **Detector core** → new module `rust/agent-loop/src/stream_loop_detector.rs` in `codelet-agent-loop` (pure, sync, no I/O — unit + proptest friendly). Exports `StreamLoopDetector`, `LoopSignal`, `LoopDetectorConfig`, and the escalation-policy type.
- **Wiring** → `background_output.rs` (feed deltas, hold per-channel state) + `stream_loop.rs` (abort callback flips interrupt flag) + `recovery_thinking.rs`-style builder for the corrective note.

## Conclusion

The design is fully supported by existing machinery:
- Detection state → `BackgroundOutput` (per-turn, per-channel).
- Abort → existing `is_interrupted`/`interrupt_notify` interrupt path.
- Corrective note → mirror `recovery_thinking.rs` builder pattern.
- Truncation → stop accumulation + append marker block.

No new cancellation or persistence infrastructure is required.
