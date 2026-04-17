# CMPCT-030 — Replace Tautological Compaction Tests with Real Integration Coverage

**Parent:** CMPCT-022
**Bug:** Test coverage gap

## The Current State of Tests

### `codelet/cli/tests/gemini_continuation_compaction_test.rs`

This entire file (187 lines) is **tautological**. Example scenario:

```rust
#[test]
fn test_graceful_handling_when_compaction_triggered_during_continuation() {
    // @step Given a Gemini model session is in continuation mode
    let mut token_tracker = TokenTracker::new();
    token_tracker.input_tokens = 150_000;
    
    // Simulate what the fixed code does:
    // 1. Check if error is compaction-related
    let error_str = "PromptCancelled";
    let is_compaction_cancel = error_str.contains("PromptCancelled");
    assert!(is_compaction_cancel, "Should detect compaction cancellation");
    
    // ...
    let compaction_needed = true; // This is what the implementation sets
    assert!(compaction_needed, "Compaction should be signaled");
}
```

The test literally asserts that `"PromptCancelled".contains("PromptCancelled")` and that `true == true`. It does not invoke any production code from `stream_loop.rs` or `gemini_continuation.rs`. The `integration_behavior_documentation` module at the bottom openly admits this:

> "Actual behavior is verified by the unit tests above" *(which verify nothing)*

### `codelet/cli/tests/prompt_too_long_recovery_test.rs`

Better — tests the pure `is_prompt_too_long_error()` function against many input strings. But all tests are pure-function decision tests. Not one exercises the stream-loop error handler end-to-end.

### `codelet/core/src/compaction_hook.rs` tests

Good unit tests for the hook's threshold decision logic. But none wire up the hook into a real streaming pipeline; they call `check_compaction()` directly.

### `codelet/cli/tests/emergency_threshold_compaction_test.rs`

Tests `execute_compaction()` as a standalone call. Doesn't exercise the error→recovery pipeline.

## What's Missing

### 1. Full end-to-end cycle
A test that:
- Creates a mock `impl Stream<Item = Result<MultiTurnStreamItem, anyhow::Error>>`
- Yields `[Text, Text, Err(PromptCancelled { chat_history })]`
- Feeds through `run_agent_stream_internal`
- Verifies:
  - `session.messages` has the correct final content
  - `session.token_tracker.cumulative_billed_*` is updated
  - `execute_compaction` was called exactly once
  - The retry stream was started with "Continue"
  - The retry stream completed successfully

### 2. PromptCancelled at each of the 6 rig cancel sites
Parameterize over the 6 sites:
- After `on_completion_call` (before API call, no tokens)
- After `on_text_delta` (tokens emitted)
- After `on_tool_call` (tool call emitted, not executed)
- After `on_tool_result` (tool executed)
- After `on_tool_call_delta` (tool args streaming)
- After `on_stream_completion_response_finish` (turn complete)

Each should trigger recovery and produce a consistent final state.

### 3. Last-user-message pop verification
Test `stream_loop.rs:1190-1196`:
```rust
// Seed
session.messages = vec![
    Message::System { ... },
    Message::User { content: "first prompt" },
    Message::Assistant { content: "response" },
    Message::User { content: "second prompt" },  // ← this must be popped
];
// Force a prompt-too-long error
// Assert session.messages.last() is Assistant, not User("second prompt")
```

### 4. compaction_needed=false + PromptCancelled (BUG 4)
```rust
// Directly yield PromptCancelled without setting the flag
// Verify recovery STILL runs (after CMPCT-026 fix)
```

### 5. Nested/wrapped PromptCancelled (BUG 3)
```rust
// Wrap PromptError::PromptCancelled in .context("upstream") + StreamingError::Prompt
// Verify is_compaction_cancelled still returns true (after CMPCT-025 fix)
```

### 6. Partial-text preservation (BUG 2)
```rust
// Yield [Text("important analysis"), Err(PromptCancelled)]
// Verify session.messages.last() == Assistant { content: "important analysis" }
// (after CMPCT-024 fix)
```

### 7. Cascading compaction (BUG 5)
```rust
// Primary stream: PromptCancelled
// Compaction runs → retry stream: "prompt is too long"
// Verify a second compaction attempt runs (after CMPCT-027 fix)
```

### 8. Circuit breaker
```rust
// Primary stream: PromptCancelled
// Retry 1: PromptCancelled
// Retry 2: PromptCancelled
// Retry 3: PromptCancelled
// Verify Err("Compaction retry budget exhausted")
```

### 9. Pre-prompt vs post-cancel equivalence
```rust
// Path A: compaction runs BEFORE stream starts (pre-prompt)
// Path C: compaction runs AFTER stream starts (hook cancel)
// Both given identical initial state
// Verify session.messages after both paths complete is bit-identical
```

### 10. Mid-tool-call cancel (BUG 8)
```rust
// Yield [ToolCall, ToolResult, Err(PromptCancelled)]
// Verify session.messages contains the ToolCall and matching ToolResult
// (after CMPCT-029 fix)
```

## Test Infrastructure Needed

### A mock `impl Stream` that yields a programmable sequence

```rust
use futures::stream::{self, Stream};
use rig::agent::MultiTurnStreamItem;

fn mock_stream(
    items: Vec<Result<MultiTurnStreamItem<MockResponse>, anyhow::Error>>,
) -> impl Stream<Item = Result<MultiTurnStreamItem<MockResponse>, anyhow::Error>> + Unpin {
    stream::iter(items)
}
```

### A mock `RigAgent` 

Looking at `run_agent_stream_internal`, it takes `agent: &RigAgent<M>`. We need a test-mode agent whose `prompt_streaming_with_history_and_hook` returns our programmable stream. Options:
- **Option 1**: Introduce a `trait AgentStreamer` abstraction that both `RigAgent` and `MockAgent` implement. Change the signature of `run_agent_stream_internal` to take `&impl AgentStreamer`. (Large refactor.)
- **Option 2**: Extract the pure-state-machine logic from `run_agent_stream_internal` into a smaller function that takes a stream directly: `fn process_stream(stream: impl Stream, session: &mut Session, ...) -> Result<StreamOutcome>`. Test THAT function. (Recommended.)
- **Option 3**: Use `mockall` to mock the whole `RigAgent`. (May be brittle.)

### A mock `StreamOutput`

`stream_loop.rs` takes `output: &O where O: StreamOutput`. Create a test-mode `StreamOutput` that records every `emit_*` call for later assertion:

```rust
struct CapturingOutput {
    events: Mutex<Vec<StreamEvent>>,
}
impl StreamOutput for CapturingOutput { ... }
```

## Implementation Steps

1. **Extract `process_stream` function** — pull the main `loop { match stream.next().await { ... } }` out of `run_agent_stream_internal` into a separately-testable function that takes the stream by generic parameter.
2. **Build `CapturingOutput`** — test helper that records events.
3. **Build `mock_multi_turn_stream_items`** — helpers to construct the stream items: `text_chunk()`, `tool_call()`, `tool_result()`, `usage()`, `final_response()`, `prompt_cancelled_error()`.
4. **Delete `gemini_continuation_compaction_test.rs`** (or replace its contents entirely).
5. **Write 10 new scenarios** listed above.

## Acceptance Criteria

1. `codelet/cli/tests/gemini_continuation_compaction_test.rs` no longer contains tautological assertions like `"PromptCancelled".contains("PromptCancelled")`.
2. At least one test invokes `process_stream` (or equivalent) with a mock stream that yields `PromptCancelled` and verifies the full session state after recovery.
3. Parameterized tests cover all 6 rig cancel sites.
4. Each fix in CMPCT-023 through CMPCT-029 has at least one regression test that would fail without the fix.
5. Test runtime remains under 30s for the full suite.

## Files to Modify

- `codelet/cli/tests/gemini_continuation_compaction_test.rs` — rewrite
- `codelet/cli/src/interactive/stream_loop.rs` — extract `process_stream`
- `codelet/cli/tests/helpers/` (new directory) — `CapturingOutput`, mock stream builders
- `codelet/cli/tests/` — new files:
  - `prompt_cancelled_end_to_end_test.rs`
  - `prompt_cancelled_at_each_hook_site_test.rs`
  - `compaction_cascade_test.rs`
  - `compaction_circuit_breaker_test.rs`
  - `prompt_cancelled_tool_call_preservation_test.rs`

## Testing This Card

By definition, this card IS the tests. Acceptance is: 
- Every other card (CMPCT-023 through CMPCT-029) has a regression test that exercises its fix.
- Running the full test suite with no fixes applied shows the new tests FAILING for the right reasons.
- Running with all fixes applied shows them all passing.
