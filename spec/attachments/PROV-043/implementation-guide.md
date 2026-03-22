# PROV-043: Structured Retry with Exponential Backoff — Implementation Guide

## Problem

PROV-040 (truncation recovery) and PROV-041 (thinking exhaustion recovery) both create new streams immediately with zero delay between retries. If the API is rate-limited or overloaded, immediate retry worsens the problem.

**Current code (stream_loop.rs ~line 1701, thinking exhaustion retry):**
```rust
// Creates new stream immediately — no delay
stream = agent
    .prompt_streaming_with_history_and_hook(&recovery_msg, &mut session.messages, retry_hook)
    .await;
```

**Same pattern at ~line 1887 for truncation retry.**

## VTCode Reference

**File:** `vtcode-core/src/retry.rs` lines 12–190

### RetryPolicy struct (lines 12–20)
```rust
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub jitter: f64,
}
```

### Delay calculation (lines 52–69)
```rust
pub fn delay_for_attempt(&self, attempt_index: u32) -> Duration {
    let multiplier = self.multiplier.powi(attempt_index as i32);
    let base_delay = Duration::from_secs_f64(
        self.initial_delay.as_secs_f64() * multiplier
    ).min(self.max_delay);

    if self.jitter <= 0.0 {
        return base_delay;
    }

    let max_jitter_ms = (base_delay.as_millis() as f64 * self.jitter)
        .round().clamp(0.0, u64::MAX as f64) as u64;
    if max_jitter_ms == 0 {
        return base_delay;
    }
    let offset = (u64::from(attempt_index) * 31) % (max_jitter_ms + 1);
    base_delay.saturating_add(Duration::from_millis(offset))
}
```

### Default policy (lines 186–190)
```rust
impl Default for RetryPolicy {
    fn default() -> Self {
        Self::from_retries(2, Duration::from_secs(1), Duration::from_secs(60), 2.0)
    }
}
```
→ 3 total attempts, delays: 1s → 2s → 4s, capped at 60s

### Decision-making (lines 72–95)
```rust
pub fn decision_for_category(
    &self, category: ErrorCategory, attempt_index: u32, retry_after: Option<Duration>,
) -> RetryDecision {
    let has_remaining_attempts = attempt_index.saturating_add(1) < self.max_attempts;
    if !category.is_retryable() || !has_remaining_attempts {
        return RetryDecision { category, retryable: false, delay: None, retry_after };
    }
    let delay = retry_after.unwrap_or_else(|| self.delay_for_attempt(attempt_index));
    RetryDecision { category, retryable: true, delay: Some(delay), retry_after }
}
```

### Usage in agent runner (vtcode-core/src/core/agent/runner/retry.rs lines 88–108)
```rust
let backoff_duration = decision.delay.expect("retryable decisions need delay");
metrics.record_retry_attempt();
self.runner_println(format_args!(
    "{} Task failed (attempt {}/{}), retrying in {}s...",
    style("[Warning]").red().bold(),
    attempt + 1, policy.max_attempts, backoff_duration.as_secs()
));
sleep(backoff_duration).await;
```

## Proposed Implementation for fspec

### 1. Create RetryPolicy in codelet-core or a new module

```rust
// codelet/core/src/retry_policy.rs (new file)
use std::time::Duration;

pub struct StreamRetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
}

impl StreamRetryPolicy {
    pub fn delay_for_attempt(&self, attempt_index: u32) -> Duration {
        let multiplier = self.multiplier.powi(attempt_index as i32);
        let delay = Duration::from_secs_f64(
            self.initial_delay.as_secs_f64() * multiplier
        );
        delay.min(self.max_delay)
    }
}

impl Default for StreamRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
        }
    }
}
```

### 2. Apply in stream_loop.rs

**At thinking exhaustion retry (~line 1700):**
```rust
// Before creating new stream, add delay
let retry_delay = StreamRetryPolicy::default()
    .delay_for_attempt(thinking_exhaustion_retry_count - 1);
info!("PROV-043: Waiting {:?} before thinking exhaustion retry", retry_delay);
output.emit_status(&format!(
    "Waiting {:.0}s before retry...",
    retry_delay.as_secs_f64()
));
tokio::time::sleep(retry_delay).await;

// Then create new stream as before
stream = agent.prompt_streaming_with_history_and_hook(...).await;
```

**Same pattern at truncation retry (~line 1887).**

### 3. Tests

```rust
#[test]
fn retry_policy_default_delays() {
    let policy = StreamRetryPolicy::default();
    assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(2));
    assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(4));
    assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(8));
}

#[test]
fn retry_policy_caps_at_max() {
    let policy = StreamRetryPolicy {
        max_delay: Duration::from_secs(5),
        ..Default::default()
    };
    assert_eq!(policy.delay_for_attempt(10), Duration::from_secs(5));
}
```

## Estimated Effort: 3 story points
