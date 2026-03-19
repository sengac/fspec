# SCHED-007: AST Research — Catch-Up on Restart

## Key Code Patterns

### spawn_scheduler (engine.rs:22-33)
Current implementation immediately enters the tick loop. Catch-up must be inserted
BEFORE the loop:

```rust
pub fn spawn_scheduler(project_path: String) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        let state = Arc::new(SchedulerState::new());
        info!("Scheduler started for project: {}", project_path);
        // INSERT: run_catch_up(&project_path, &state).await
        loop {
            interval.tick().await;
            if let Err(e) = evaluate_and_run(&project_path, &state).await { ... }
        }
    })
}
```

### find_previous_trigger (engine.rs:302-325)
Already exists — reusable for catch-up detection. Takes a cron + now-in-tz,
returns Option<DateTime<Tz>>.

### evaluate_single_schedule (engine.rs:210-300)
The existing trigger logic already uses lastRunAt vs previous_trigger comparison.
Catch-up logic is essentially the same comparison but:
1. Runs once at startup (not every 30s)
2. Calls trigger_and_update directly for missed schedules
3. Updates lastRunAt immediately to prevent double-fire

### update_last_run (engine.rs:368-385)
Reusable for updating lastRunAt after catch-up fires.
