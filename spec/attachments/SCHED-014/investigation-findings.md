# SCHED-014: Fair Dispatch for Overlapping Loop Entries — Investigation

## Problem Statement

When multiple `/loop` entries target the same session, a high-frequency loop can **starve** lower-frequency loops. There is no coordination between loop tasks — each independently races to fire when the session becomes idle.

---

## Current Architecture

### Per-Entry Spawned Tasks (SCHED-013)

Each `/loop` entry spawns its own independent tokio task:

```rust
let handle = tokio::spawn(async move {
    loop {
        tokio::time::sleep(interval).await;
        // check expiry → self-terminate
        // check idle → skip if busy
        on_fire(prompt.clone());
    }
});
```

- **No coordination** between loop tasks on the same session
- **No shared queue** — each task independently sleeps, wakes, checks idle, fires
- **No round-robin** — wakeup order depends on tokio's timer wheel (registration time + interval phase)

### The TOCTOU Race

The idle check is a **point-in-time check, not a reservation**:

1. Loop A checks `idle_check` → `true` (Idle)
2. Loop B checks `idle_check` → `true` (Idle) — still Idle because A hasn't called `send_input` yet
3. Loop A calls `on_fire` → `send_input` → status becomes Running
4. Loop B calls `on_fire` → `send_input` → succeeds because `try_send` pushes into the 32-slot mpsc channel

**Result**: Both prompts land in the input channel back-to-back. The agent loop processes them sequentially. The first loop to wake up always gets served first.

### Starvation Scenario

Loop A (5s interval) and Loop B (30s interval) on the same session:

| Time | Event | Result |
|------|-------|--------|
| t=5s | Loop A wakes, session idle | **A fires** → session busy |
| t=10s | Loop A wakes, session busy | A skipped |
| t=12s | Session becomes idle | — |
| t=15s | Loop A wakes, session idle | **A fires** → session busy |
| t=20s | Loop A wakes, session busy | A skipped |
| t=22s | Session becomes idle | — |
| t=25s | Loop A wakes, session idle | **A fires** → session busy |
| t=30s | Loop B wakes, session busy | **B skipped** ← starved |
| ... | Pattern continues | B keeps losing the race |

Loop B can be starved indefinitely because Loop A wakes up 6x more often.

### What the Cron Engine Does (for comparison)

The cron scheduler (`state.rs`) has a proven overlap system:

- **`OverlapAction::Skip`** — drop the trigger if already running
- **`OverlapAction::Queue`** — enqueue ONE pending job; fire when previous completes
- **Replace semantics** — `queue.retain(|j| j.schedule_name != name)` — at most one queued instance per schedule
- **Completion-driven draining** — `sweep_completed` → `drain_queued_for`
- **FIFO fairness** for deferred jobs via `VecDeque::pop_front()`

Loop entries use **none** of this infrastructure.

---

## Proposed Solutions

### Option A: Centralized Per-Session Dispatcher (Recommended)

Instead of each loop task independently calling `on_fire`, introduce a **per-session dispatch queue**:

```rust
struct SessionLoopDispatcher {
    /// Pending prompts: (loop_id, prompt) with replace semantics per loop_id
    pending: RwLock<VecDeque<(String, String)>>,
    /// Track which loop_id last fired (for round-robin)
    last_fired: RwLock<Option<String>>,
}
```

**How it works:**

1. Loop tasks **don't call `on_fire` directly**. Instead, they `enqueue` their prompt into the per-session dispatcher.
2. The dispatcher has a single tokio task that:
   - Waits for `SessionStatus::Idle` (via `tokio::sync::Notify` or polling)
   - Picks the next loop from the queue using **round-robin** (skip past `last_fired`)
   - Calls `send_input` for that one prompt
   - Waits for the session to become Idle again
3. **Replace semantics**: if loop A's timer fires twice before it gets served, only the latest prompt is kept — preventing queue backup.

**Why this is recommended:**
- Eliminates the TOCTOU race entirely (single dispatcher owns the idle→fire transition)
- Directly maps to the cron engine's proven queue/drain pattern
- Requires minimal changes to `LoopStore` (loop tasks `enqueue` instead of `on_fire`)
- Round-robin naturally prevents any single high-frequency loop from monopolizing the session

### Option B: Per-Entry Overlap Policy (Lighter Touch)

Add an overlap policy to `LoopEntry` itself:

```rust
pub struct LoopEntry {
    // ... existing fields ...
    pub overlap_policy: OverlapPolicy,  // skip | queue
}
```

- **Skip (current behavior, made explicit)**: If session is busy, skip this tick
- **Queue**: If session is busy, enqueue ONE pending execution. When session becomes idle, fire the queued prompt before any new timer-driven prompt

This reuses the `OverlapAction` enum from `state.rs`.

**Pros**: Minimal scope, reuses existing concepts.
**Cons**: Doesn't solve fairness *between* loops — just makes individual loops less lossy.

### Option C: Weighted Fair Queuing (Sophisticated)

Priority based on starvation ratio:

```rust
struct FairLoopScheduler {
    /// Priority = time_since_last_fire / interval_seconds
    /// Higher ratio = more "starved" = higher priority
    entries: RwLock<BTreeMap<OrderedFloat<f64>, String>>,
}
```

When the session becomes idle, pick the loop with the highest starvation ratio. A 30-minute loop that hasn't fired in 30 minutes (ratio 1.0) beats a 5-second loop that fired 5 seconds ago (ratio ~0.17).

**Pros**: Mathematically fair, prevents starvation regardless of interval ratios.
**Cons**: More complex, likely over-engineered for typical usage (2-3 loops per session).

---

## Comparison Table

| Aspect | Current State | Cron Engine | Option A | Option B | Option C |
|--------|--------------|-------------|----------|----------|----------|
| Overlap detection | None (TOCTOU race) | `check_overlap()` with `active_runs` | Centralized dispatcher | Per-entry policy | Centralized scheduler |
| Queueing | None (skip or race) | `VecDeque` with replace | Per-session `VecDeque` with replace per loop_id | One pending per entry | Priority queue |
| Fairness | First-to-wake wins | FIFO for deferred | Round-robin | None (individual only) | Starvation-ratio |
| Backpressure | 32-slot mpsc silently drops | Explicit skip/queue/defer | Dispatcher controls admission | Per-entry decision | Scheduler controls admission |
| Completion signal | `set_status(Idle)` | `sweep_completed()` | Subscribe to Idle via `Notify` | Poll on next wake | Subscribe to Idle |
| Complexity | Trivial | Moderate | Moderate | Low | High |

---

## Recommendation

**Option A (Centralized Per-Session Dispatcher)** is the right approach because it:

1. Eliminates the TOCTOU race entirely
2. Mirrors the cron engine's proven architecture
3. Provides natural round-robin fairness
4. Has moderate complexity with high payoff
5. Can be extended to Option C's starvation-ratio priority later if needed
