# BUG-220: Interruption provenance is lost

## Scope and confidence

Researched on 2026-10-21 at `7e60de8e`. Source confirms that user cancellation and internal loop cancellation converge on a reasonless interrupt primitive and event. This is an observability/classification defect, not evidence that every interruption is erroneous. No new regression was executed.

## Evidence and source chain

- `rust/agent-loop/src/background_output.rs:265` stages a loop-abort corrective note, then calls `session.interrupt()` at line 269.
- `rust/sessions/src/background_session.rs:1697` sets the interrupt atomic, advances the epoch, requests Bash abortion and wakes the stream. It does not carry a typed initiating cause through that primitive.
- `rust/cli/src/interactive/stream_loop.rs:768` tears down the stream and emits the interruption event.
- `rust/rpc-types/src/lib.rs:1495` defines `StreamChunk::Interrupted` with queued inputs but no interruption reason.
- `rust/cli/src/interactive/interruption.rs:8` distinguishes accepted completion from interruption for cancellation classification, but not an internal loop abort from user action.
- `rust/fspec-tui/src/store/agent_view/record_chunk.rs:89` renders the generic `Interrupted` notice.

The 02:59:36 UTC loop abort in session `321a7519-b009-497d-98ee-5acc966a52a4` is a concrete internally initiated interruption. Its subsequent automatic retry is at `rust/agent-loop/src/agent_loop.rs:1562`.

The shared synthetic-input branch can also log `Compaction watchdog` for a loop-abort retry. Diagnostic labels should name the actual recovery cause, not whichever path originally introduced the shared branch.

## Impact

Operators cannot tell an internal safety action from Escape, queued-input interruption or other cancellation. Tool cancellation and persisted status may imply user action when no user interrupt occurred. This contributes to the apparent randomness in the Codex screenshot, independently of BUG-219's reconnection banner.

## Proposed behavioral scope

- Preserve an explicit cause from the initiating action through session control, stream teardown, tool cancellation, wire serialization and TUI rendering.
- Include user cancellation, loop-detector abort and other existing internal causes discovered while mapping callers; do not guess a cause from a pending note after teardown.
- Define precedence when a user interrupt races an internal abort. An actual user stop must not be overridden by automatic retry.
- Preserve accepted done/goal completion as completion rather than an error-like user interruption.
- Keep compatibility with clients receiving older reasonless events.

## Specifications and regression work

Review `spec/features/agent-loop-interrupt-cascade.feature`, `spec/features/immediate-esc-interrupt-during-tool-execution.feature`, `spec/features/codex-interruption-outcome-recording.feature`, and `spec/features/streaming-loop-detection.feature`.

Add integration coverage for a real internal detector abort and for user Escape while a tool runs. Assert the cause survives persistence and both embedded/WebSocket transport paths. Include concurrent causes, stale epochs, accepted completion and the bounded retry path. Existing cancellation cleanup must remain intact.

Related cards: BUG-216 (false evidence), BUG-219 (wrong notification), BUG-214 (compaction cancellation). Do not merge their distinct correctness requirements into a cosmetic message-only change.
