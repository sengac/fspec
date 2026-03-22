# PROV-044: Circuit Breaker / API Failure Tracker — Implementation Guide

## Problem

No failure tracking across stream loop invocations. If the API returns errors on 3 consecutive user turns, fspec keeps trying at full speed. Each turn independently encounters the same error with no cross-turn memory or backoff.

## VTCode Reference

### ApiFailureTracker (`vtcode-core/src/core/agent/state.rs` lines 111–151)

```rust
pub struct ApiFailureTracker {
    pub consecutive_failures: u32,
    pub last_failure: Option<std::time::Instant>,
}

impl ApiFailureTracker {
    pub fn new() -> Self {
        Self { consecutive_failures: 0, last_failure: None }
    }

    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.last_failure = Some(std::time::Instant::now());
    }

    pub fn reset(&mut self) {
        self.consecutive_failures = 0;
        self.last_failure = None;
    }

    pub fn should_circuit_break(&self) -> bool {
        self.consecutive_failures >= 3
    }

    pub fn backoff_duration(&self) -> Duration {
        let base_ms = 1000;
        let max_ms = 30000;
        let backoff_ms = base_ms * 2_u64.pow(self.consecutive_failures.saturating_sub(1));
        Duration::from_millis(backoff_ms.min(max_ms))
    }
}
```

### Tracker on AgentRunner (`vtcode-core/src/core/agent/runner.rs` lines 109–115)

```rust
// Field on AgentRunner
failure_tracker: Mutex<ApiFailureTracker>,
streaming_failures: Mutex<u8>,
streaming_last_failure: Mutex<Option<std::time::Instant>>,
```

### Usage before API call (`vtcode-core/src/core/agent/runner/provider_response.rs` lines 211–219)

```rust
// Check circuit breaker before fallback
if self.failure_tracker.lock().should_circuit_break() {
    let backoff = self.failure_tracker.lock().backoff_duration();
    warn!(
        "Circuit breaker active after {} consecutive failures. Waiting {:?} before retry.",
        self.failure_tracker.lock().consecutive_failures,
        backoff
    );
    tokio::time::sleep(backoff).await;
}
```

### Recording failures (lines 237–238)
```rust
// On API error
self.failure_tracker.lock().record_failure();
```

### Reset on success (line 282)
```rust
// On successful response
self.failure_tracker.lock().reset();
```

## Proposed Implementation for fspec

### 1. Add ApiFailureTracker to Session (`codelet/cli/src/session/mod.rs`)

```rust
/// PROV-044: Tracks consecutive API failures for circuit breaker behavior
#[derive(Debug)]
pub struct ApiFailureTracker {
    pub consecutive_failures: u32,
    pub last_failure: Option<std::time::Instant>,
}

impl ApiFailureTracker {
    pub fn new() -> Self {
        Self { consecutive_failures: 0, last_failure: None }
    }

    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.last_failure = Some(std::time::Instant::now());
    }

    pub fn reset(&mut self) {
        self.consecutive_failures = 0;
        self.last_failure = None;
    }

    pub fn should_circuit_break(&self) -> bool {
        self.consecutive_failures >= 3
    }

    pub fn backoff_duration(&self) -> std::time::Duration {
        use std::time::Duration;
        let base_ms: u64 = 1000;
        let max_ms: u64 = 30_000;
        let backoff_ms = base_ms.saturating_mul(
            2u64.saturating_pow(self.consecutive_failures.saturating_sub(1))
        );
        Duration::from_millis(backoff_ms.min(max_ms))
    }
}
```

Add field to Session:
```rust
pub struct Session {
    // ... existing fields ...
    pub api_failure_tracker: ApiFailureTracker,
}
```

### 2. Apply in stream_loop.rs

**Before creating stream (~line 828, before `agent.prompt_streaming_with_history_and_hook`):**
```rust
// PROV-044: Check circuit breaker before API call
if session.api_failure_tracker.should_circuit_break() {
    let backoff = session.api_failure_tracker.backoff_duration();
    warn!(
        "PROV-044: Circuit breaker active after {} consecutive failures. Waiting {:?}",
        session.api_failure_tracker.consecutive_failures, backoff
    );
    output.emit_status(&format!(
        "API has failed {} times consecutively. Waiting {:.0}s before retry...",
        session.api_failure_tracker.consecutive_failures,
        backoff.as_secs_f64()
    ));
    tokio::time::sleep(backoff).await;
}
```

**On error (the `Some(Err(e))` branch, ~line 1755):**
```rust
// Record failure for circuit breaker
session.api_failure_tracker.record_failure();
```

**On successful FinalResponse (~line 1095):**
```rust
// Reset circuit breaker on success
session.api_failure_tracker.reset();
```

### 3. Tests

```rust
#[test]
fn circuit_breaker_triggers_after_three_failures() {
    let mut tracker = ApiFailureTracker::new();
    assert!(!tracker.should_circuit_break());

    tracker.record_failure();
    tracker.record_failure();
    assert!(!tracker.should_circuit_break());

    tracker.record_failure();
    assert!(tracker.should_circuit_break());
}

#[test]
fn circuit_breaker_resets_on_success() {
    let mut tracker = ApiFailureTracker::new();
    tracker.record_failure();
    tracker.record_failure();
    tracker.record_failure();
    assert!(tracker.should_circuit_break());

    tracker.reset();
    assert!(!tracker.should_circuit_break());
    assert_eq!(tracker.consecutive_failures, 0);
}

#[test]
fn backoff_duration_grows_exponentially() {
    let mut tracker = ApiFailureTracker::new();
    tracker.record_failure(); // 1
    assert_eq!(tracker.backoff_duration(), Duration::from_millis(1000));

    tracker.record_failure(); // 2
    assert_eq!(tracker.backoff_duration(), Duration::from_millis(2000));

    tracker.record_failure(); // 3
    assert_eq!(tracker.backoff_duration(), Duration::from_millis(4000));
}

#[test]
fn backoff_caps_at_30_seconds() {
    let mut tracker = ApiFailureTracker::new();
    for _ in 0..20 {
        tracker.record_failure();
    }
    assert_eq!(tracker.backoff_duration(), Duration::from_millis(30_000));
}
```

## Dependency

Depends on PROV-043 (RetryPolicy struct) — can share the delay calculation logic.

## Estimated Effort: 3 story points
