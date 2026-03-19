# SCHED-013: Investigation Findings — Loop Tick Resolution Bug

## Problem Statement

When a user runs `/loop 5s check status`, they expect the prompt to fire every 5 seconds. Instead, it fires every ~30 seconds.

## Root Cause

The scheduler engine (`codelet/napi/src/scheduler/engine.rs`) has a **single hard-coded 30-second tick interval** on line 32:

```rust
let mut interval = tokio::time::interval(Duration::from_secs(30));
```

The `/loop` entries are evaluated as **Step 8** of this 30-second cron tick cycle (`evaluate_and_fire_loops()` called on line 196 of `evaluate_and_run()`). The entire data path works correctly — parsing, storage, and due-detection all honour sub-second intervals — but the engine only **polls** every 30 seconds, quantizing all loop execution to 30s minimum.

The `loopCommandParser.ts` header (lines 6-7) documents this as intentional:
> *"The scheduler engine ticks every 30 seconds, so that is the effective minimum resolution."*

## What Works Correctly

| Component | File | Verdict |
|-----------|------|---------|
| Parser: `/loop 5s` → `intervalSeconds: 5` | `src/tui/utils/loopCommandParser.ts` | ✅ Correct |
| LoopStore: stores `interval_seconds: 5` | `codelet/napi/src/scheduler/loop_store.rs` | ✅ Correct |
| `is_due()`: reports due after 5 seconds | `loop_store.rs:28-36` | ✅ Correct |
| Service: confirms "every 5 seconds" to user | `src/tui/services/loop-service.ts:141-146` | ✅ Correct (but misleading) |
| Engine: evaluates loops every 30s tick | `engine.rs:32, 196` | ❌ Bug — quantizes to 30s |

## Options Considered

### Option A: Separate fast-tick polling loop for loops
- Add a second `tokio::time::interval` at ~1s for loop evaluation only
- Requires adaptive interval (GCD of registered intervals) or fixed fast tick
- Still couples all entries through shared polling; wastes cycles when no entries need firing
- Violates SRP: tick loop manages timing for all entries collectively

### Option B: Reduce main tick from 30s to 5s
- Simplest change (one line) but wastes CPU evaluating cron expressions 6x more often
- Cron only needs minute-level granularity at best
- Still quantizes loops to 5s — doesn't solve 1s or 2s intervals
- Violates SRP: cron doesn't benefit from faster tick

### Option C: Per-entry spawned task (RECOMMENDED)
Each `/loop` entry gets its own `tokio::spawn` task with `tokio::time::sleep(interval)`:

**SOLID Analysis:**
- **S (Single Responsibility)**: Each task owns exactly one entry's timing and execution
- **O (Open/Closed)**: Adding/removing loops doesn't touch the engine. `evaluate_and_fire_loops()` is deleted
- **L (Liskov)**: N/A — no inheritance hierarchy
- **I (Interface Segregation)**: Engine no longer depends on loop-specific interfaces
- **D (Dependency Inversion)**: Loop tasks depend on session abstraction, not the scheduler engine

**DRY/Composable Benefits:**
- Zero polling waste — each task sleeps for exactly its interval
- Exact timing — 5s means 5s, 7s means 7s, no quantization
- Clean lifecycle — cancel = abort the JoinHandle
- Fully independent units — loops are composable, not coupled
- `LoopStore` transforms from passive data store → active task manager

**Concerns (all manageable):**
1. Task proliferation — loops are ephemeral/session-scoped, single-digit count
2. Per-session concurrency — use atomic flag or check-before-send to avoid flooding
3. Session-idle check — same pattern as current code, just inside the task loop

## Reference Implementation Check

- **VTCode** (`/tmp/VTCode`): No cron/scheduler system at all. Their `LoopDetector` is an anti-loop safety mechanism for detecting repetitive agent tool calls — completely unrelated.
- **OpenCode** (`/tmp/opencode`): Frontend-only packages, no scheduling infrastructure.

No prior art to draw from — this is original to fspec.

## Affected Files

### Must Change
- `codelet/napi/src/scheduler/loop_store.rs` — Store `JoinHandle` per entry, spawn task on register, abort on cancel
- `codelet/napi/src/scheduler/engine.rs` — Remove `evaluate_and_fire_loops()` call from `evaluate_and_run()`, remove the function entirely
- `src/tui/utils/loopCommandParser.ts` — Update header comment (remove "30 second minimum resolution" note)

### May Change
- `codelet/napi/src/scheduler/mod.rs` — Ensure loop_store has access to session_manager for send_input
- Existing loop_store tests — Adapt for task-based model

### No Change Needed
- `src/tui/services/loop-service.ts` — Already passes correct interval to NAPI
- `src/tui/utils/loopCommandParser.ts` — Parser already handles sub-second correctly
- `codelet/napi/src/scheduler/engine.rs` (cron parts) — 30s tick stays for cron, untouched
