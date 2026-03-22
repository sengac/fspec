# PROV-048: Refactoring Guide — Streaming Health + Session Sub-Component Pattern

## Refactoring Opportunity

PROV-048 adds streaming failure tracking. Combined with PROV-044's circuit breaker and PROV-041's thinking state, Session would have **three separate tracking concerns** bolted on as raw fields. This card establishes the **sub-component pattern** for Session.

## Current Anti-Pattern: Flat Field Explosion

Without refactoring, Session would become:

```rust
pub struct Session {
    provider_manager: ProviderManager,
    pub messages: Vec<Message>,
    pub turns: Vec<ConversationTurn>,
    pub token_tracker: TokenTracker,
    pub annotations: HashMap<...>,
    pub thinking_exhaustion_cross_turn_count: u32,     // PROV-041
    pub session_thinking_level: ThinkingLevel,         // PROV-041
    pub api_failure_tracker: ApiHealthTracker,          // PROV-044
    pub streaming_failure_count: u32,                   // PROV-048
    pub last_streaming_failure: Option<Instant>,        // PROV-048
}
// 10 fields, 8 public — a data bag, not an object
```

## The Sub-Component Pattern

Instead, group related tracking into cohesive sub-components behind Session methods:

```rust
pub struct Session {
    provider_manager: ProviderManager,
    pub messages: Vec<Message>,          // TODO: encapsulate in ConversationState
    pub turns: Vec<ConversationTurn>,    // TODO: encapsulate in ConversationState
    pub token_tracker: TokenTracker,     // TODO: encapsulate
    pub annotations: HashMap<...>,       // TODO: encapsulate in ConversationState
    thinking: ThinkingState,             // PROV-044 extracts this
    api_health: ApiHealthTracker,        // PROV-044 adds this
    streaming_health: StreamingHealth,   // PROV-048 adds this
}
```

The `thinking`, `api_health`, and `streaming_health` fields are **private** — accessed only through Session methods.

## New Module: `session/streaming_health.rs` (~70 lines)

```rust
use std::time::{Duration, Instant};

const FAILURE_COOLDOWN: Duration = Duration::from_secs(60);
const WARNING_THRESHOLD: u32 = 3;

/// Tracks streaming failure patterns across turns within a session.
/// Provides diagnostics and cooldown-based reset.
pub struct StreamingHealth {
    failure_count: u32,
    last_failure: Option<Instant>,
}

impl StreamingHealth {
    pub fn new() -> Self {
        Self { failure_count: 0, last_failure: None }
    }

    /// Record a streaming failure. Returns diagnostic message if threshold exceeded.
    pub fn record_failure(&mut self) -> Option<String> {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());

        if self.failure_count >= WARNING_THRESHOLD {
            Some(format!(
                "Streaming has failed {} times this session. Consider switching providers (/openai, /gemini) or checking network connectivity.",
                self.failure_count
            ))
        } else {
            None
        }
    }

    /// Record a successful stream completion. Resets the counter.
    pub fn record_success(&mut self) {
        self.failure_count = 0;
    }

    /// Check if enough time has passed since last failure to reset.
    /// Called before each API call to give the connection another chance.
    pub fn maybe_reset_cooldown(&mut self) {
        if let Some(last) = self.last_failure {
            if last.elapsed() > FAILURE_COOLDOWN {
                self.failure_count = 0;
                self.last_failure = None;
            }
        }
    }

    pub fn failure_count(&self) -> u32 { self.failure_count }

    pub fn reset(&mut self) { *self = Self::new(); }
}
```

## Session Methods (Facade)

```rust
impl Session {
    /// Record a streaming failure. Returns a diagnostic warning if failures exceed threshold.
    pub fn record_streaming_failure(&mut self) -> Option<String> {
        self.streaming_health.record_failure()
    }

    /// Record a successful streaming completion.
    pub fn record_streaming_success(&mut self) {
        self.streaming_health.record_success();
        self.api_health.record_success(); // PROV-044: success resets both
    }

    /// Pre-flight check: apply cooldowns and determine if API call should proceed.
    pub fn pre_flight_health_check(&mut self) -> HealthCheckResult {
        self.streaming_health.maybe_reset_cooldown();
        self.api_health.reset_if_cooled_down();

        if let Some(delay) = self.api_health.suggested_delay() {
            HealthCheckResult::DelayRequired(delay)
        } else {
            HealthCheckResult::Proceed
        }
    }

    /// Full reset on provider switch.
    pub fn switch_provider(&mut self, name: &str) -> Result<()> {
        self.provider_manager.switch_provider(name)?;
        self.messages.clear();
        self.turns.clear();
        self.token_tracker = TokenTracker::default();
        self.annotations.clear();
        self.thinking.reset();
        self.api_health = ApiHealthTracker::new();
        self.streaming_health.reset();  // NEW
        Ok(())
    }
}

pub enum HealthCheckResult {
    Proceed,
    DelayRequired(Duration),
}
```

## Integration in stream_loop.rs

### Before stream creation (~line 828)

```rust
// PROV-048 + PROV-044: Pre-flight health check
match session.pre_flight_health_check() {
    HealthCheckResult::DelayRequired(delay) => {
        output.emit_status(&format!("Waiting {:.1}s before API call...", delay.as_secs_f64()));
        tokio::time::sleep(delay).await;
    }
    HealthCheckResult::Proceed => {}
}
```

### On FinalResponse (success)

```rust
session.record_streaming_success(); // resets both streaming_health and api_health
```

### On Error

```rust
StreamOutcome::Error(kind) => {
    if let Some(warning) = session.record_streaming_failure() {
        output.emit_status(&warning);
    }
    // ... existing error handling
}
```

## Why This Pattern Matters for Future Cards

The sub-component pattern established here makes future additions trivial:

| Future Need | Just Add |
|------------|----------|
| Request latency tracking | `LatencyTracker` sub-component |
| Token spend budgeting | `SpendTracker` sub-component |
| Model quality scoring | `QualityTracker` sub-component |

Each is a private field with public Session methods. No `pub` field leakage. No caller knowledge of internals.

## Estimated Impact

- **Lines added to stream_loop.rs**: ~15 (health check + success/failure recording)
- **New module**: `session/streaming_health.rs` (~70 lines)
- **Session refactoring**: Establishes the sub-component + facade pattern
- **switch_provider bug**: Fully fixed (all sub-components reset)
