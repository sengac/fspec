# RPC-058 — Lift scheduler engine into `codelet-core::scheduler`; `/schedule` subcommand handler

**Parent:** RPC-030 · **Phase:** 7.5 · **Estimate:** 8 pts · **Depends on:** RPC-057

## Goal

Lift the scheduler engine currently embedded in `codelet/napi/src/` (look for cron, tokio interval, `spawn_scheduled_session` at `session_manager.rs` line 3214, `ensure_scheduler_running` line 3861) into `codelet-core::scheduler`. New RPC methods. `/schedule add|list|pause|resume|remove` subcommand handler matching TS `schedule-service.handleScheduleCommand`.

## TS reference

`SLASH_COMMANDS[12]` syntax: `add|list|pause|resume|remove [options]`. The TS service lives in `src/tui/services/schedule-service.ts`. Backed by `codelet/napi/src/scheduler/*` (or similar).

## Lift target

Create `codelet/core/src/scheduler/mod.rs`:

```rust
pub struct SchedulerEngine {
    jobs: RwLock<Vec<ScheduledJob>>,
    runner_handle: RwLock<Option<JoinHandle<()>>>,
    on_trigger: Arc<dyn Fn(ScheduleTrigger) + Send + Sync>,
}

pub struct ScheduledJob {
    pub id: String,
    pub name: String,
    pub cron: String,
    pub prompt: String,
    pub session_id: Option<SessionId>,
    pub paused: bool,
    pub last_triggered: Option<DateTime<Utc>>,
    pub next_trigger: Option<DateTime<Utc>>,
}

impl SchedulerEngine {
    pub fn new(on_trigger: impl Fn(ScheduleTrigger) + Send + Sync + 'static) -> Self;
    pub fn add_job(&self, job: ScheduledJob) -> Result<(), String>;
    pub fn list_jobs(&self) -> Vec<ScheduledJob>;
    pub fn pause(&self, id: &str) -> Result<(), String>;
    pub fn resume(&self, id: &str) -> Result<(), String>;
    pub fn remove(&self, id: &str) -> Result<(), String>;
    pub fn start_runner(&self);
    pub fn stop_runner(&self);
}
```

Lift cron-parsing + interval-tick logic out of NAPI verbatim. Persistence: JSON file at `~/.fspec/schedules.json` (or workspace-local).

## Backend trait additions

```rust
fn schedule_add(&self, name: String, cron: String, prompt: String) -> Result<ScheduledJob, String>;
fn schedule_list(&self) -> Vec<ScheduledJob>;
fn schedule_pause(&self, id: &str) -> Result<(), String>;
fn schedule_resume(&self, id: &str) -> Result<(), String>;
fn schedule_remove(&self, id: &str) -> Result<(), String>;
```

New wire type `ScheduledJob` in `codelet-rpc-types` (mirror the core struct minus the `on_trigger` callback).

## SessionManager integration

When a schedule fires, `SchedulerEngine` calls `on_trigger(trigger)` which calls `SessionManager::spawn_scheduled_session(...)` (the existing async method at line 3214). After the lift, the callback closure in `SessionManager::new()` is:

```rust
let on_trigger = {
    let sm = self.clone(); // Arc<SessionManager>
    move |trigger| {
        tokio::spawn(async move {
            let _ = sm.spawn_scheduled_session(trigger).await;
        });
    }
};
SchedulerEngine::new(on_trigger)
```

## Slash command parser

`/schedule add --name foo --cron "0 * * * *" --prompt "check the build"`
`/schedule list`
`/schedule pause <id>`
`/schedule resume <id>`
`/schedule remove <id>`

Parser lives in `codelet/fspec-tui/src/app/schedule_parser.rs`:

```rust
pub enum ScheduleSubcommand {
    Add { name: String, cron: String, prompt: String },
    List,
    Pause(String),
    Resume(String),
    Remove(String),
}

pub fn parse(args: &str) -> Result<ScheduleSubcommand, String> { /* shlex-style parse */ }
```

Dispatcher routes the parsed subcommand:

```rust
SlashCommandAction::Schedule => {
    // Parsed in the submit-line handler since /schedule takes args.
    // … see slash_parser.rs for shape ...
}
```

(Move the actual dispatch to the submit-line path since the bare `/schedule` opens a list view by default — match TS.)

## Acceptance criteria

1. `codelet/core/src/scheduler/mod.rs` exists with all engine logic. No NAPI dependency.
2. `codelet/napi/src/` scheduler files are deleted or reduced to re-export shims.
3. New RPC methods on `SessionManagerHandle`, `FspecService`, `FspecBackend`.
4. `/schedule list` shows all jobs.
5. `/schedule add --name daily --cron "0 9 * * *" --prompt "morning standup"` creates a job that triggers daily at 9am.
6. Pause / resume / remove work.
7. On trigger, a new session is spawned with the prompt as initial input.
8. Persistence: schedules survive fspec restart.
9. Integration test in `codelet/fspec-tui/tests/scheduler.rs` covers happy path + cron edge cases (next_trigger calculation).

## Risks

- Cron parsing libraries vary in syntax (5-field vs 6-field). Pick one (`cron` crate, 5-field standard) and document.
- The TS scheduler may use a JS cron library with quirks. Compare next-trigger output for several patterns before declaring parity.
- `spawn_scheduled_session` creates a new session each trigger. Confirm no leak with hourly schedules.

## Out of scope

- Schedule output collection / archive (TS has notifications; not in this card).
