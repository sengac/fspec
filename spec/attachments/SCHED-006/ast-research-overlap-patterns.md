# SCHED-006: AST Research — Overlap & Session Limit Management

## Key Code Patterns Found

### evaluate_and_run (engine.rs:40)
The main tick function. Currently evaluates all schedules and triggers each hit immediately.
Overlap check needs to be inserted BEFORE `trigger_and_update` call.

```
engine.rs:40 — pub async fn evaluate_and_run(project_path: &str) -> Result<(), anyhow::Error>
```

### trigger_and_update (engine.rs:229)
Handles job execution and timestamp updates. Routes to `trigger_agent_job` or `trigger_shell_job`.
Agent jobs return session IDs that need to be tracked in `active_runs`.

```
engine.rs:229 — async fn trigger_and_update(schedules_path, name, job_type, project_path, entry)
```

### ScheduleEntry (types.rs:17)
Already has `overlap_policy: Option<String>` field — no schema change needed.

### MAX_SESSIONS (session_manager.rs:78)
```rust
const MAX_SESSIONS: usize = 10;
```
Enforced at `create_session_with_id` (line 3181, 3345). Returns error if `sessions.len() >= MAX_SESSIONS`.

### Session storage pattern
Sessions are stored in `SessionManager.sessions: RwLock<HashMap<Uuid, BackgroundSession>>`.
Access via `SessionManager::instance().sessions.read().await` to check if a session ID still exists.

## Design Decisions

1. **SchedulerState** — new struct holding `active_runs`, `queued_jobs`, `deferred_jobs`
   - Constructed once in `spawn_scheduler`, passed to `evaluate_and_run` by reference
   - `active_runs: RwLock<HashMap<String, Uuid>>` — schedule_name → session_id
   - `queued_jobs: RwLock<VecDeque<QueuedJob>>` — overlap=queue waiting for same schedule
   - `deferred_jobs: RwLock<VecDeque<DeferredJob>>` — session limit waiting for any slot

2. **Sweep on tick** — before evaluating schedules, sweep `active_runs` by checking
   `SessionManager::instance()` for session presence. Remove completed entries, drain queues.

3. **Shell jobs bypass session limit** — they use `tokio::process::Command`, not BackgroundSession.
   Only agent jobs need overlap tracking and session limit deferral.
