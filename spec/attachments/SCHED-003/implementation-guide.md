# SCHED-003: Core Scheduler Engine — Implementation Guide

## Overview

Implement the tokio scheduler service inside `SessionManager`. This is a single `tokio::spawn` task with a 30-second interval timer that evaluates all schedules from `spec/schedules.json` against the current time using a Rust cron crate. It starts alongside `SessionManager::instance()` and stops when fspec exits.

## Architecture

### Where It Lives

The scheduler is a tokio task spawned within `SessionManager`. The `SessionManager` struct (in `codelet/napi/src/session_manager.rs`) is a singleton via `OnceLock`:

```rust
pub struct SessionManager {
    sessions: RwLock<IndexMap<Uuid, Arc<BackgroundSession>>>,
    chain_of_command: ChainOfCommand,
    active_session_id: RwLock<Option<Uuid>>,
    // NEW: scheduler handle for graceful shutdown
    scheduler_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
}
```

### Timer Loop Pattern

Follow the same pattern as the reaper in `codelet/napi/src/unified_exec/reaper.rs`, but at a 30-second cadence:

```rust
pub fn spawn_scheduler(project_path: String) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = evaluate_schedules(&project_path).await {
                // Log error but don't crash the scheduler
                eprintln!("Scheduler error: {}", e);
            }
        }
    })
}
```

The reaper reference:
- Located at `codelet/napi/src/unified_exec/reaper.rs`
- Uses `tokio::time::sleep(Duration::from_secs(2))` in a loop (we use `interval` instead for more precise cadence)
- Checks a global store each tick and exits when its target is gone

### Cron Evaluation

Use a lightweight Rust crate for cron evaluation. Options:

1. **`cron`** crate — Most popular, supports standard 7-field cron expressions
2. **`croner`** crate — Lightweight, supports 5-field standard + timezone-aware scheduling

The evaluation logic each tick:

```rust
async fn evaluate_schedules(project_path: &str) -> Result<(), SchedulerError> {
    let schedules_path = format!("{}/spec/schedules.json", project_path);
    let file_content = tokio::fs::read_to_string(&schedules_path).await?;
    let schedules: SchedulesFile = serde_json::from_str(&file_content)?;
    
    let now = chrono::Utc::now();
    
    for (name, schedule) in &schedules.schedules {
        if schedule.status != "active" {
            continue; // Skip paused schedules
        }
        
        if should_trigger(schedule, now) {
            trigger_job(name, schedule, project_path).await?;
        }
    }
    
    Ok(())
}
```

### `should_trigger()` Logic

```
1. Parse cron expression + timezone
2. Calculate the PREVIOUS trigger time before `now` in the schedule's timezone
3. If `last_run_at` is None OR `last_run_at` < previous_trigger_time:
   → This schedule needs to fire
4. Before firing, check overlap policy (delegated to SCHED-006)
5. Before firing, check MAX_SESSIONS (delegated to SCHED-006)
```

### Timezone Handling

Use `chrono-tz` crate for IANA timezone conversion:

```rust
use chrono_tz::Tz;

let tz: Tz = schedule.timezone.parse()?;
let now_in_tz = chrono::Utc::now().with_timezone(&tz);
```

## Integration Points

### Starting the Scheduler

The scheduler should start when a session is created in a project that has `spec/schedules.json`. Options:

**Option A: Start on first session creation**
In `create_session_with_id()` or `create_initial_session()`, check if the project has schedules and spawn the scheduler if not already running.

**Option B: Start explicitly from TypeScript**
Add a NAPI function `start_scheduler(project_path: String)` that TypeScript calls during app initialization.

Option A is preferred — it's self-contained within the Rust layer.

### Triggering Jobs

The scheduler doesn't execute jobs directly. Instead, it calls into the job execution layer:

```rust
async fn trigger_job(name: &str, schedule: &ScheduleEntry, project_path: &str) -> Result<()> {
    match schedule.job_type {
        JobType::Agent { .. } => {
            // SCHED-004: Spawn agent subordinate
            trigger_agent_job(name, schedule, project_path).await
        }
        JobType::Shell { .. } => {
            // SCHED-005: Execute shell command
            trigger_shell_job(name, schedule, project_path).await
        }
    }
}
```

### Updating Last-Run Timestamp

After a job completes (or is skipped), update `spec/schedules.json`:

```rust
async fn update_last_run(
    schedules_path: &str,
    name: &str,
    status: &str,
) -> Result<()> {
    // Read → modify → write (consider file locking with flock/advisory locks)
    let content = tokio::fs::read_to_string(schedules_path).await?;
    let mut schedules: SchedulesFile = serde_json::from_str(&content)?;
    
    if let Some(entry) = schedules.schedules.get_mut(name) {
        entry.last_run_at = Some(chrono::Utc::now().to_rfc3339());
        entry.last_run_status = Some(status.to_string());
    }
    
    let json = serde_json::to_string_pretty(&schedules)?;
    tokio::fs::write(schedules_path, json).await?;
    Ok(())
}
```

**Important**: The TypeScript layer uses `LockedFileManager` with `proper-lockfile` for file locking. The Rust layer needs to coordinate. Options:
1. Use `flock`/`fcntl` advisory locks in Rust
2. Route all writes through TypeScript via NAPI
3. Use a simple atomic write (write to .tmp, rename) — acceptable if Rust is the only writer for timestamps

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `codelet/napi/src/scheduler/mod.rs` | Create | Scheduler module |
| `codelet/napi/src/scheduler/engine.rs` | Create | Timer loop + cron evaluation |
| `codelet/napi/src/scheduler/types.rs` | Create | Rust types for schedules.json |
| `codelet/napi/src/session_manager.rs` | Modify | Add scheduler spawn on startup |
| `Cargo.toml` (codelet-napi) | Modify | Add cron + chrono-tz dependencies |

## Crate Dependencies

```toml
[dependencies]
cron = "0.12"       # or croner = "2.0"
chrono-tz = "0.8"   # IANA timezone support
```

## Key Constraints

- The scheduler MUST NOT crash if `spec/schedules.json` is missing or malformed — log and skip
- The scheduler MUST NOT block the tokio runtime — all operations must be async
- File reads happen every 30 seconds — this picks up schedule additions/removals without restart
- The 30-second interval means triggers may fire up to 30 seconds late — acceptable for cron-scale scheduling
- The scheduler runs for the lifetime of the fspec process — no separate daemon
