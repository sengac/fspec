# SCHED-007: Catch-Up on Restart — Implementation Guide

## Overview

On fspec startup, the scheduler loads `spec/schedules.json` and compares each schedule's `lastRunAt` timestamp against its cron expression to detect missed triggers. If a trigger was missed while fspec was closed, fire at most ONE catch-up run (the most recent missed trigger). Never replay multiple missed triggers.

## Algorithm

```
For each schedule in schedules.json where status == "active":
  1. Parse the cron expression + timezone
  2. Calculate: when was the LAST time this cron should have fired? (previous_trigger)
  3. If lastRunAt is null → this schedule has never run, fire catch-up
  4. If lastRunAt < previous_trigger → the most recent trigger was missed, fire catch-up
  5. If lastRunAt >= previous_trigger → no miss, nothing to do
```

### Pseudocode

```rust
use chrono::{Utc, DateTime};
use cron::Schedule;

async fn check_catch_up(
    schedule: &ScheduleEntry,
    now: DateTime<Utc>,
) -> Option<CatchUpAction> {
    let cron_schedule = Schedule::from_str(&schedule.cron)?;
    let tz: chrono_tz::Tz = schedule.timezone.parse()?;
    
    // Find the most recent trigger time BEFORE now
    let now_in_tz = now.with_timezone(&tz);
    let previous_trigger = cron_schedule
        .before(&now_in_tz)
        .next()?  // Most recent past trigger
        .with_timezone(&Utc);
    
    match &schedule.last_run_at {
        None => {
            // Never run — catch up
            Some(CatchUpAction::Fire { schedule_name: schedule.name.clone() })
        }
        Some(last_run) => {
            let last_run_dt = DateTime::parse_from_rfc3339(last_run)?;
            if last_run_dt < previous_trigger {
                // Missed — catch up (ONCE, not for every missed trigger)
                Some(CatchUpAction::Fire { schedule_name: schedule.name.clone() })
            } else {
                None // No miss
            }
        }
    }
}
```

### Critical: "At Most Once" Guarantee

The catch-up logic fires AT MOST ONCE per schedule, regardless of how many triggers were missed. This is ensured by:

1. Only checking the MOST RECENT trigger time (not iterating all missed triggers)
2. Updating `lastRunAt` to `now` immediately when the catch-up fires (before the job actually completes)
3. The next scheduler tick sees `lastRunAt >= previous_trigger` and skips

### Example

```
Schedule: "0 2 * * *" (daily at 2 AM Brisbane)
fspec closed: Mon 10 PM
fspec restarted: Thu 8 AM

Missed triggers: Tue 2 AM, Wed 2 AM, Thu 2 AM
Most recent previous trigger: Thu 2 AM
lastRunAt: Mon 2 AM

Mon 2 AM < Thu 2 AM → missed!
Action: Fire ONE catch-up run, update lastRunAt to "Thu 8 AM" (now)
```

## Integration with Scheduler Startup

The catch-up check runs ONCE at scheduler initialization, before the regular 30-second tick loop begins:

```rust
pub async fn start_scheduler(project_path: String) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Phase 1: Catch-up check (runs once)
        if let Err(e) = run_catch_up_checks(&project_path).await {
            eprintln!("Catch-up check failed: {}", e);
        }
        
        // Phase 2: Regular schedule evaluation loop
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = evaluate_schedules(&project_path).await {
                eprintln!("Scheduler error: {}", e);
            }
        }
    })
}
```

### Interaction with Overlap & Session Limits

Catch-up jobs still respect:
- **Overlap policy** — if somehow a previous run is still active (unlikely on restart, but possible if sessions are restored), apply the overlap policy
- **MAX_SESSIONS** — if 10 sessions are already active, defer the catch-up job

### Ordering

If multiple schedules need catch-up, they're processed sequentially in file order. Each catch-up respects MAX_SESSIONS, so not all may fire immediately.

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| `lastRunAt` is null (never run) | Fire catch-up |
| `lastRunAt` is in the future (clock skew) | Skip — no miss detected |
| fspec was closed for 30 days, cron is `0 2 * * *` | Fire ONE catch-up, not 30 |
| fspec was closed for 5 minutes, cron is `0 2 * * *` | No catch-up — trigger wasn't missed |
| Schedule is paused | Skip — catch-up only for active schedules |
| Schedule was added while fspec was closed | `lastRunAt` is null → fire catch-up if a trigger was due |

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `codelet/napi/src/scheduler/catch_up.rs` | Create | Catch-up detection and execution |
| `codelet/napi/src/scheduler/engine.rs` | Modify | Call catch-up on startup before tick loop |

## Key Constraints

- At most ONE catch-up per schedule — never replay multiple missed triggers
- Catch-up runs at fspec startup, before the regular tick loop
- `lastRunAt` is updated to `now` when catch-up fires (prevents double-firing on subsequent tick)
- Paused schedules are skipped during catch-up
- Catch-up jobs respect overlap policy and MAX_SESSIONS
