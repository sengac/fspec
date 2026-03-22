# PROV-048: Streaming Failure Tracking — Implementation Guide

## Problem

When streaming fails (network interruption, SSE parse error, timeout), fspec reports the error and terminates. There's no tracking of whether streaming is generally reliable for this provider/session. The user gets no diagnostic insight into recurring streaming issues.

## VTCode Reference

### Streaming failure tracking (`vtcode-core/src/core/agent/runner.rs` lines 114–115)

```rust
// Fields on AgentRunner
streaming_failures: Mutex<u8>,
streaming_last_failure: Mutex<Option<std::time::Instant>>,
```

### Constants (`vtcode-core/src/core/agent/runner/constants.rs`)

```rust
pub(super) const MAX_STREAMING_FAILURES: u8 = 2;
pub(super) const STREAMING_COOLDOWN_SECS: u64 = 60; // 1 minute
```

### Recording streaming failures (`vtcode-core/src/core/agent/runner/provider_response.rs` lines 88–114)

Three failure paths all increment the counter:

```rust
// 1. Stream event error (line 89–93)
Err(err) => {
    let mut failures = self.streaming_failures.lock();
    *failures = failures.saturating_add(1);
    *self.streaming_last_failure.lock() = Some(std::time::Instant::now());
    self.failure_tracker.lock().record_failure();
    // ... emit warning ...
}

// 2. Stream creation failed (lines 119–122)
Ok(Err(err)) => {
    let mut failures = self.streaming_failures.lock();
    *failures = failures.saturating_add(1);
    // ...
}

// 3. Stream timeout (lines 137–140)
Err(_) => {
    let mut failures = self.streaming_failures.lock();
    *failures = failures.saturating_add(1);
    // ...
}
```

### Cooldown and reset (`provider_response.rs` lines 37–45)

```rust
// Before each request, check if cooldown has elapsed
if let Some(last_failure) = *self.streaming_last_failure.lock()
    && last_failure.elapsed().as_secs() >= STREAMING_COOLDOWN_SECS
{
    // Reset — give streaming another chance
    *self.streaming_failures.lock() = 0;
    self.streaming_last_failure.lock().take();
}

// Check if streaming should be disabled
streaming_disabled = *self.streaming_failures.lock() >= MAX_STREAMING_FAILURES;
```

### Disabling streaming (`provider_response.rs` lines 158–163)

```rust
} else if streaming_disabled {
    let warning = "Skipping streaming after repeated streaming failures";
    warnings.push(warning.to_string());
    event_recorder.warning(warning);
    // Falls through to non-streaming generate() path
}
```

### Reset on success (`provider_response.rs` lines 166–167, 282–284)

```rust
// After successful streaming response
*self.streaming_failures.lock() = 0;
self.streaming_last_failure.lock().take();

// After successful non-streaming response
self.failure_tracker.lock().reset();
*self.streaming_failures.lock() = self.streaming_failures.lock().saturating_sub(1);
```

## Proposed Implementation for fspec

### 1. Add streaming failure tracking to Session

```rust
// codelet/cli/src/session/mod.rs — add to Session struct

/// PROV-048: Tracks consecutive streaming failures for diagnostic reporting.
/// When streaming fails repeatedly, emits warnings to help diagnose provider issues.
pub streaming_failure_count: u32,
pub last_streaming_failure: Option<std::time::Instant>,
```

### 2. Track failures in stream_loop.rs error branch

```rust
// In the Some(Err(e)) branch (~line 1755)
session.streaming_failure_count += 1;
session.last_streaming_failure = Some(std::time::Instant::now());

if session.streaming_failure_count >= 3 {
    output.emit_status(&format!(
        "Warning: Streaming has failed {} times this session. \
         Consider switching providers or checking network connectivity.",
        session.streaming_failure_count
    ));
}
```

### 3. Reset on success

```rust
// In the FinalResponse handler (~line 1095)
session.streaming_failure_count = 0;
session.last_streaming_failure = None;
```

### 4. Cooldown check at stream start

```rust
// Before creating stream (~line 828)
if let Some(last_fail) = session.last_streaming_failure {
    if last_fail.elapsed() > Duration::from_secs(60) {
        // Cooldown elapsed — reset counter
        session.streaming_failure_count = 0;
        session.last_streaming_failure = None;
    }
}
```

### 5. Tests

```rust
#[test]
fn streaming_failures_accumulate() {
    let mut session = create_test_session();
    assert_eq!(session.streaming_failure_count, 0);

    session.streaming_failure_count += 1;
    session.streaming_failure_count += 1;
    session.streaming_failure_count += 1;
    assert_eq!(session.streaming_failure_count, 3);
}

#[test]
fn streaming_failures_reset_on_success() {
    let mut session = create_test_session();
    session.streaming_failure_count = 5;
    session.last_streaming_failure = Some(Instant::now());

    // Simulate success
    session.streaming_failure_count = 0;
    session.last_streaming_failure = None;

    assert_eq!(session.streaming_failure_count, 0);
    assert!(session.last_streaming_failure.is_none());
}
```

## Note

fspec cannot currently fall back to non-streaming mode because rig's API doesn't expose a separate non-streaming path in the multi-turn agent. However, the **tracking** is still valuable for diagnostics. If/when we implement the custom LLM abstraction (RIG-001 through RIG-010), we can add the actual fallback.

## Estimated Effort: 2 story points
