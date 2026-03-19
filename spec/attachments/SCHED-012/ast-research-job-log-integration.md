# AST Research: Schedule Job Log Integration Points

## trigger_and_update() — main hook point
- **File:** `codelet/napi/src/scheduler/engine.rs:334`
- This is where jobs are triggered and status updated — the primary place to emit "triggered", "completed", and "failed" log entries.

## check_overlap() — skip/queue detection
- **File:** `codelet/napi/src/scheduler/state.rs:49`
- Returns `OverlapAction::Skip` or `OverlapAction::Queue` — log "skipped" and "queued" events in engine.rs where these are matched (lines 116-121).

## defer() — session limit deferral
- **File:** `codelet/napi/src/scheduler/state.rs:83`
- Called when session limit is reached — log "deferred" event in engine.rs at line 129 where `state.defer()` is called.

## enqueue() — overlap queue
- **File:** `codelet/napi/src/scheduler/state.rs:71`
- Called for queue overlap policy — log "queued" event at the call site in engine.rs line 118.

## Scheduler module structure
- `mod.rs` declares: agent_job, catch_up, engine, shell_job, state, types
- New `job_log` module will be added here.

## Integration plan
1. Create `job_log.rs` with `append_log_entry()` and `maybe_rotate()` 
2. Call from `trigger_and_update()` for triggered/completed/failed
3. Call from `scheduler_tick()` match arms for skipped/queued/deferred
4. All calls use `tokio::spawn` to avoid blocking the tick
