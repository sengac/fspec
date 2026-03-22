# PROV-044: Refactoring Guide — Circuit Breaker + Session Encapsulation

## Refactoring Opportunity

PROV-044 adds a circuit breaker (ApiHealthTracker). Rather than bolting another `pub` field onto Session's already-leaking struct, this card is the vehicle for **Session encapsulation reform**.

## Current Session Problem

```rust
pub struct Session {
    provider_manager: ProviderManager,              // private (good)
    pub messages: Vec<Message>,                     // 72 external accesses
    pub turns: Vec<ConversationTurn>,               // 8 external accesses
    pub token_tracker: TokenTracker,                // 45 external accesses
    pub annotations: HashMap<...>,                  // 5 external accesses
    pub thinking_exhaustion_cross_turn_count: u32,  // 3 external accesses
    pub session_thinking_level: ThinkingLevel,      // 4 external accesses
}
```

Adding `pub api_failure_tracker: ApiHealthTracker` would make it worse. Instead:

## Session Encapsulation Strategy

### Step 1: Extract ThinkingState (data clump)

```rust
// session/thinking_state.rs (~60 lines)
pub struct ThinkingState {
    exhaustion_count: u32,        // was: thinking_exhaustion_cross_turn_count
    level: ThinkingLevel,         // was: session_thinking_level
}

impl ThinkingState {
    pub fn new() -> Self { Self { exhaustion_count: 0, level: ThinkingLevel::High } }
    pub fn record_exhaustion(&mut self) { self.exhaustion_count += 1; }
    pub fn should_downgrade(&self) -> bool { self.exhaustion_count >= CROSS_TURN_THRESHOLD }
    pub fn downgrade(&mut self) -> ThinkingLevel {
        self.level = downgrade_thinking_level(self.level);
        self.exhaustion_count = 0;
        self.level
    }
    pub fn level(&self) -> ThinkingLevel { self.level }
    pub fn reset(&mut self) { *self = Self::new(); }
}
```

### Step 2: Add ApiHealthTracker (new, encapsulated)

```rust
// session/api_health.rs OR interactive/circuit_breaker.rs (~80 lines)
pub struct ApiHealthTracker {
    consecutive_failures: u32,
    last_failure: Option<Instant>,
    cooldown_reset: Duration,       // 60s — after this, reset counter
}

impl ApiHealthTracker {
    pub fn new() -> Self { ... }
    pub fn record_failure(&mut self) { ... }
    pub fn record_success(&mut self) { self.consecutive_failures = 0; }
    pub fn is_circuit_open(&self) -> bool { self.consecutive_failures >= 3 }
    pub fn suggested_delay(&self) -> Option<Duration> {
        if self.consecutive_failures == 0 { return None; }
        // Exponential: 1s, 2s, 4s, 8s... capped at 30s
        let delay = Duration::from_secs(1) * 2u32.pow(self.consecutive_failures.saturating_sub(1));
        Some(delay.min(Duration::from_secs(30)))
    }
    pub fn reset_if_cooled_down(&mut self) {
        if let Some(last) = self.last_failure {
            if last.elapsed() > self.cooldown_reset { self.consecutive_failures = 0; }
        }
    }
}
```

### Step 3: Add Domain Methods to Session

Instead of `session.thinking_exhaustion_cross_turn_count += 1` (raw field mutation), add:

```rust
impl Session {
    /// Record a thinking exhaustion event. Returns the new thinking level if downgraded.
    pub fn record_thinking_exhaustion(&mut self) -> Option<ThinkingLevel> {
        self.thinking.record_exhaustion();
        if self.thinking.should_downgrade() {
            Some(self.thinking.downgrade())
        } else {
            None
        }
    }

    pub fn thinking_level(&self) -> ThinkingLevel { self.thinking.level() }

    /// Record an API failure. Returns suggested delay if circuit is opening.
    pub fn record_api_failure(&mut self) -> Option<Duration> {
        self.api_health.record_failure();
        self.api_health.suggested_delay()
    }

    pub fn record_api_success(&mut self) { self.api_health.record_success(); }

    pub fn should_delay_api_call(&mut self) -> Option<Duration> {
        self.api_health.reset_if_cooled_down();
        if self.api_health.is_circuit_open() {
            self.api_health.suggested_delay()
        } else {
            None
        }
    }
}
```

### Step 4: Fix `switch_provider()` Bug

Current `switch_provider()` resets messages/turns/token_tracker but **forgets** annotations, thinking state, and (now) api_health. With sub-components:

```rust
pub fn switch_provider(&mut self, provider_name: &str) -> Result<()> {
    self.provider_manager.switch_provider(provider_name)?;
    self.messages.clear();
    self.turns.clear();
    self.token_tracker = TokenTracker::default();
    self.annotations.clear();         // BUG FIX: was missing
    self.thinking.reset();            // BUG FIX: was missing
    self.api_health = ApiHealthTracker::new(); // NEW: clean slate
    Ok(())
}
```

## Integration in stream_loop.rs

### Before API call (~line 828)

```rust
// PROV-044: Pre-flight circuit breaker check
if let Some(delay) = session.should_delay_api_call() {
    output.emit_status(&format!(
        "API experiencing failures. Waiting {:.1}s before retry...",
        delay.as_secs_f64()
    ));
    tokio::time::sleep(delay).await;
}
```

### On FinalResponse (success path)

```rust
StreamOutcome::Completed { .. } => {
    session.record_api_success();
    // ...
}
```

### On Error (failure path)

```rust
StreamOutcome::Error(kind) => {
    if let Some(delay) = session.record_api_failure() {
        output.emit_status(&format!("Consecutive API failure. Next attempt delayed by {:.1}s", delay.as_secs_f64()));
    }
    // ... existing recovery logic
}
```

## Why This Is Better Than Adding Another `pub` Field

| Approach | External Accesses | Invariant Enforcement | switch_provider Bug |
|----------|------------------|-----------------------|---------------------|
| `pub api_failure_tracker` | Every caller manages state | None — caller's burden | Will be forgotten again |
| `record_api_failure()` method | Zero — encapsulated | Counter + timestamp atomically updated | Handled in reset() |

## SOLID Alignment

| Principle | How |
|-----------|-----|
| **SRP** | ThinkingState: one job. ApiHealthTracker: one job. Session: orchestrates. |
| **OCP** | Adding streaming health (PROV-048) = new sub-component, no Session struct changes |
| **DIP** | stream_loop depends on Session's methods, not its internal field layout |
| **Tell Don't Ask** | `session.record_api_failure()` vs `session.api_health.consecutive_failures += 1` |

## Estimated Impact

- **Lines removed from stream_loop.rs**: ~30 (thinking state direct manipulation → method calls)
- **New modules**: `thinking_state.rs` (~60), `circuit_breaker.rs` (~80)
- **Session struct**: 7 pub fields → 4 pub fields (thinking + api_health encapsulated)
- **Bug fixed**: `switch_provider()` now resets all state
