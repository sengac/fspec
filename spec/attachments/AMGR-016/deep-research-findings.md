# AMGR-016: Deep Research Findings — Root Cause Analysis

## Research Methodology

Three parallel subordinate agents with DeepSearch investigated:
1. **Agent 1** — Session lifecycle state machine (running/idle transitions)
2. **Agent 2** — DeepSearch sub-agent spawning and result injection
3. **Agent 3** — Error handling, timeouts, and watchdog mechanisms

*Note: Agents 2 and 3 timed out on `await_idle(600s)` — ironically demonstrating the exact bug under investigation. All three eventually completed.*

---

## Finding 1: Session Status Has No "Errored" State

**File**: `codelet/napi/src/session_manager.rs` (line 97–109)

```rust
#[repr(u8)]
pub enum SessionStatus {
    Idle = 0,
    Running = 1,
    Interrupted = 2,
    Paused = 3,       // waiting for user input
    Compacting = 4,   // context compaction in progress
}
```

**Impact**: When an agent encounters a fatal error, it can only transition to `Idle` — there is no `Errored` or `Stalled` variant. The spawner has no way to distinguish "completed successfully" from "crashed after error" from "stuck forever".

Storage: `AtomicU8` per `BackgroundSession`, reads `Ordering::Acquire`, writes `Ordering::AcqRel`. Status changes broadcast via `supervisor_broadcast` channel (used by `await_idle`).

---

## Finding 2: The Outer Agent Loop Always Sets Idle — In Theory

**File**: `codelet/napi/src/session_manager.rs` (lines 5248–5266)

```rust
if let Err(e) = result {
    session.handle_output(StreamChunk::error(e.to_string()));
    session.set_status(SessionStatus::Idle);  // line 5260
    session.handle_output(StreamChunk::done());
} else {
    session.set_status(SessionStatus::Idle);  // line 5265
}
```

Both success and error paths unconditionally set `Idle`. **However**, this code only runs AFTER the stream loop returns. If the stream loop never returns (i.e., `stream.next().await` blocks forever), this cleanup code never executes.

---

## Finding 3: No Timeout on Stream Consumption — THE ROOT CAUSE

**File**: `codelet/cli/src/interactive/stream_loop.rs`

The inner streaming loop calls `stream.next().await` with **no timeout wrapper**:

- No `tokio::time::timeout` wraps `stream.next()`
- No per-chunk idle timeout
- No per-turn generation timeout
- No per-generation wall-clock timeout
- The `api_start_time` variable is **diagnostic-only** (used for latency reporting)

If the LLM API returns the HTTP response headers (so no HTTP-level timeout fires) but then stalls on the SSE body stream, `stream.next().await` blocks indefinitely until:
- OS TCP timeout (5–30+ minutes depending on platform)
- User presses Esc (`interrupt_notify`) — **not available for subordinate agents with no TUI**

**This is the most likely root cause for the AMGR-016 incident.** Agent `179f28e7` received its DeepSearch result (turn 10), the agent loop called the LLM for the next response, and the SSE stream stalled silently. The `stream.next().await` blocked forever with no timeout, keeping the agent in `Running` state permanently.

---

## Finding 4: DeepSearch Sub-Agent Cannot Get Stuck (Tool Layer Is Safe)

**File**: `codelet/napi/src/deep_search_handler.rs`

DeepSearch itself has robust error handling:

1. **Error → String conversion**: Rig's tool dispatch converts all errors to string results (never crashes parent):
   ```rust
   // codelet/patches/rig-core/src/agent/prompt_request/streaming.rs:493-500
   let tool_result = match agent.tool_server_handle.call_tool(&tool_name, &args).await {
       Ok(thing) => thing,
       Err(e) => e.to_string()  // Error becomes a string tool result!
   };
   ```

2. **RAII drop guards** (`SessionSearchCleanup`, `DeepSearchCleanup`, `GraphSearchCleanup`) guarantee handler cleanup even on panic

3. **Recursion limit** (`max_recursion_depth=2`) prevents infinite nesting

4. **Depth limit** (`max_depth=50`) prevents runaway tool loops

**However**: No wall-clock timeout wraps the entire sub-agent execution. If the sub-agent's own LLM call stalls, the parent blocks indefinitely on `.await`. This confirms the same underlying vulnerability exists at the DeepSearch level too.

---

## Finding 5: Error Recovery Is Comprehensive But Only For Errors That Arrive

**File**: `codelet/cli/src/interactive/error_classifiers.rs`

Five error classifiers handle different failure types:

| Priority | Error Type | Recovery | Retries |
|----------|-----------|----------|---------|
| 1 | Compaction cancellation | Expected; break | N/A |
| 2 | Prompt too long | Pop message, compact | 1 |
| 3 | Image content error | Sanitize images | 1 |
| 4 | Truncated tool call | Recovery prompt | 2 (MAX_TRUNCATION_RETRIES) |
| 5 | Transient network | Exponential backoff | 3 (1s→2s→4s) |
| 6 | Terminal error | Return Err | 0 |

All exhausted retries either `return Err(...)` (caught by agent_loop → sets Idle) or gracefully complete. **The recovery system is robust for errors that actually arrive as `Some(Err(e))` from the stream.**

The gap: if the stream produces no items at all (not `Some(Err)`, not `None`, just blocks), no classifier runs.

---

## Finding 6: Existing Watchdog Only Covers Compaction, Not Streaming

**File**: `codelet/napi/src/session_manager.rs` (lines 5135–5209)

The compaction convergence watchdog (CMPCT-020) detects when an agent fails to call `inject_summary` after compaction. But it runs **post-stream** — it waits for the stream loop to return first. It cannot detect a stalled stream because the stream loop must complete before the watchdog evaluates.

---

## Finding 7: `await_idle` Implementation

**File**: `codelet/napi/src/agent_manager_handler.rs` (line 665)

`await_idle` is event-driven via Tokio broadcast channels:
1. Subscribes to each target session's `supervisor_broadcast` channel
2. Uses `tokio::select!` with an optional `tokio::time::sleep(timeout)` branch
3. Returns `"idle"`, `"timed_out"`, `"destroyed"`, or `"interrupted"`

It cannot return `"errored"` or `"stalled"` because those states don't exist in `SessionStatus`.

---

## Root Cause Conclusion

The bug has a single root cause with two manifestations:

### Primary: No streaming idle timeout
`stream.next().await` in `stream_loop.rs` has no timeout. If the SSE stream stalls after headers are received (connection alive, no data), the agent blocks forever.

### Secondary: No stalled state detection
Even if we added a timeout, there's no mechanism to:
- Distinguish "actively generating" from "stuck waiting for tokens"
- Report a stalled/errored state to the spawner
- Auto-recover (abort turn, retry, or mark errored)

---

## Recommended Fix Architecture

### Layer 1: Streaming Idle Timeout (Critical)
Wrap `stream.next()` in `tokio::time::timeout(idle_timeout)`:
```rust
// Pseudo-code for stream_loop.rs inner loop
match tokio::time::timeout(Duration::from_secs(120), stream.next()).await {
    Ok(Some(Ok(item))) => { /* process normally, reset timer */ }
    Ok(Some(Err(e))) => { /* existing error classifier cascade */ }
    Ok(None) => { /* stream ended normally */ break; }
    Err(_elapsed) => { /* STALLED — no tokens for 120s */ 
        // Emit error, break, let agent_loop set Idle
    }
}
```

### Layer 2: Errored/Stalled Status (Important)
Add to `SessionStatus`:
```rust
pub enum SessionStatus {
    Idle = 0,
    Running = 1,
    Interrupted = 2,
    Paused = 3,
    Compacting = 4,
    Errored = 5,    // NEW: terminal error occurred
}
```

Update `await_idle` to return `"errored"` status and `get_status` to expose it.

### Layer 3: Generation Heartbeat (Nice-to-have)
Track `last_token_time: AtomicU64` per session, update on each streaming chunk. A background monitor task could detect stalls proactively without modifying the stream loop.

### Layer 4: DeepSearch Wall-Clock Timeout (Important)
Wrap the entire sub-agent execution in a timeout:
```rust
match tokio::time::timeout(Duration::from_secs(300), build_and_run_agent(...)).await {
    Ok(result) => result,
    Err(_) => Err("DeepSearch sub-agent timed out after 300s".to_string()),
}
```
