# AST Research: Interrupt Mechanism in NAPI

## Files Analyzed

### 1. codelet/napi/src/session.rs

**Current Structure:**
```rust
pub struct CodeletSession {
    inner: Arc<Mutex<codelet_cli::session::Session>>,
    is_interrupted: Arc<AtomicBool>,  // Current interrupt flag
}
```

**interrupt() method (line 66-68):**
```rust
pub fn interrupt(&self) {
    self.is_interrupted.store(true, Release);
}
```

**prompt() method (line 253-320):**
- Resets interrupt flag at start
- Calls `run_agent_stream` with `is_interrupted` Arc

### 2. codelet/cli/src/interactive/stream_loop.rs

**run_agent_stream (line 73-95):**
- NAPI entry point
- Passes `is_interrupted` to internal function
- No event stream (None) for NAPI mode

**run_agent_stream_internal (line 102-553):**
- Main stream loop
- Checks `is_interrupted` at top of loop (line 246)
- NAPI mode: Direct `stream.next().await` (line 286) - **BLOCKING**
- CLI mode: `tokio::select!` with keyboard events (line 268-282) - **NON-BLOCKING**

## Problem Identified

NAPI mode (line 284-287):
```rust
_ => {
    Some(stream.next().await)  // BLOCKS until chunk arrives!
}
```

The interrupt check at line 246 only runs after `stream.next()` returns.

## Solution Analysis

Add `tokio::sync::Notify` to enable immediate wake-up:

1. **CodeletSession changes:**
   - Add `interrupt_notify: Arc<Notify>` field
   - In `interrupt()`: call `notify_waiters()` after setting flag

2. **run_agent_stream changes:**
   - Add `interrupt_notify: Arc<Notify>` parameter
   - Use `tokio::select!` with `notify.notified()` for NAPI mode

3. **Compatibility:**
   - CLI mode unchanged (uses keyboard events)
   - NAPI mode gets immediate interrupt via Notify
