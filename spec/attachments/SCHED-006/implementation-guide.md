# SCHED-006: Overlap & Session Limit Management — Implementation Guide

## Overview

Implement two concurrent-execution controls for the scheduler:
1. **Overlap policies** (skip/queue) — what happens when a schedule triggers but its previous run is still active
2. **Session limit management** — what happens when all 10 session slots (MAX_SESSIONS) are full

## Overlap Policies

### Detection: Is Previous Run Still Active?

The scheduler needs to track which schedules currently have active sessions. Options:

**Option A: Track in scheduler state (Recommended)**

Maintain an in-memory map of schedule_name → active_session_id:

```rust
pub struct SchedulerState {
    /// Currently running sessions per schedule
    active_runs: RwLock<HashMap<String, Uuid>>,
    /// Queued jobs waiting for their previous run to complete
    queued_jobs: RwLock<VecDeque<QueuedJob>>,
}

struct QueuedJob {
    schedule_name: String,
    schedule_entry: ScheduleEntry,
    queued_at: chrono::DateTime<chrono::Utc>,
}
```

**Option B: Check SessionManager**

Query `SessionManager::instance().sessions` for sessions with matching `schedule_name`. More accurate but requires iterating all sessions each tick.

Option A is preferred — O(1) lookup per schedule per tick.

### Skip Policy

```rust
if active_runs.contains_key(&schedule.name) {
    match schedule.overlap_policy {
        OverlapPolicy::Skip => {
            // Log skip, don't fire
            record_skip_event(&schedule.name).await;
            return Ok(());
        }
        // ...
    }
}
```

### Queue Policy

```rust
OverlapPolicy::Queue => {
    // Add to queue, will be processed when the active run completes
    queued_jobs.push_back(QueuedJob {
        schedule_name: schedule.name.clone(),
        schedule_entry: schedule.clone(),
        queued_at: chrono::Utc::now(),
    });
    record_queue_event(&schedule.name).await;
    return Ok(());
}
```

### Queue Drain

When a scheduled session completes (detected by the scheduler on the next tick, or via a completion callback):

```rust
async fn on_job_complete(name: &str, scheduler_state: &SchedulerState) {
    // Remove from active runs
    scheduler_state.active_runs.write().await.remove(name);
    
    // Check if there's a queued job for this schedule
    let mut queue = scheduler_state.queued_jobs.write().await;
    if let Some(pos) = queue.iter().position(|j| j.schedule_name == name) {
        let queued = queue.remove(pos).unwrap();
        // Spawn the queued job (may still need to check MAX_SESSIONS)
        drop(queue); // Release lock before spawning
        trigger_job(&queued.schedule_name, &queued.schedule_entry, project_path).await;
    }
}
```

### Completion Detection

How does the scheduler know a session has completed? Options:

1. **Poll on tick** — Each 30-second tick, check if `active_runs` session IDs are still in `SessionManager.sessions`. Simple but up to 30s delay.
2. **Callback/watcher** — Register a session destruction callback. More responsive but more complex.
3. **Check session status** — `BackgroundSession.status` is an `AtomicU8` with values Idle/Running/Interrupted/Paused/Compacting. An idle session that was previously running has completed.

Option 1 (poll on tick) is simplest and consistent with the scheduler's existing 30-second cadence. A session that completed 25 seconds ago will be detected on the next tick.

## Session Limit Management

### MAX_SESSIONS Constant

```rust
const MAX_SESSIONS: usize = 10;
```

This is enforced in `create_session_with_id()` which returns an error if `sessions.len() >= MAX_SESSIONS`.

### Deferral Logic

Before spawning any scheduled job (agent or shell), check the session count:

```rust
async fn try_spawn_job(schedule: &ScheduleEntry, project_path: &str) -> Result<SpawnResult> {
    let session_manager = SessionManager::instance();
    let session_count = session_manager.sessions.read().await.len();
    
    if session_count >= MAX_SESSIONS {
        // Defer — add to a separate deferral queue
        record_deferral_event(&schedule.name, session_count).await;
        return Ok(SpawnResult::Deferred);
    }
    
    // Proceed with spawn
    // ...
}
```

### Deferral Queue

Deferred jobs are similar to queued jobs but wait for a session slot rather than a specific schedule's previous run:

```rust
pub struct SchedulerState {
    active_runs: RwLock<HashMap<String, Uuid>>,
    queued_jobs: RwLock<VecDeque<QueuedJob>>,     // Waiting for overlap
    deferred_jobs: RwLock<VecDeque<QueuedJob>>,   // Waiting for session slot
}
```

On each tick, after normal schedule evaluation, check if deferred jobs can now run:

```rust
async fn process_deferred_jobs(state: &SchedulerState, project_path: &str) {
    let session_count = SessionManager::instance().sessions.read().await.len();
    if session_count >= MAX_SESSIONS {
        return; // Still full
    }
    
    let mut deferred = state.deferred_jobs.write().await;
    if let Some(job) = deferred.pop_front() {
        drop(deferred); // Release lock
        trigger_job(&job.schedule_name, &job.schedule_entry, project_path).await;
    }
}
```

## Event Recording

Both skip and deferral events should be recorded in session history for SessionSearch:

```rust
async fn record_skip_event(name: &str) {
    // Emit to the scheduler's own logging mechanism
    // Could be a dedicated "scheduler" session or just file-based logging
    log::info!("Skipped {}: previous run still active", name);
}

async fn record_deferral_event(name: &str, session_count: usize) {
    log::info!("Deferred {}: session limit reached ({}/{})", name, session_count, MAX_SESSIONS);
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `codelet/napi/src/scheduler/state.rs` | Create | `SchedulerState` with active_runs, queues |
| `codelet/napi/src/scheduler/overlap.rs` | Create | Overlap policy evaluation |
| `codelet/napi/src/scheduler/engine.rs` | Modify | Integrate overlap + session limit checks |

## Key Constraints

- Overlap check happens BEFORE session limit check — a skip policy prevents even attempting to spawn
- Queue is FIFO — first-queued runs first
- Deferred jobs are re-evaluated every 30 seconds (on each scheduler tick)
- Only ONE queued/deferred job is spawned per tick — prevents burst spawning
- Skip and deferral events must be recorded for observability (SessionSearch or StreamChunk events)
