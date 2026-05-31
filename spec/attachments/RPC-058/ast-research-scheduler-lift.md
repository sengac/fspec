# AST Research — RPC-058 Scheduler Engine Lift

This dossier collects the structural inventory required before specifying the
codelet-core::scheduler lift. The patterns referenced here are the ones that
RPC-054 / RPC-055 / RPC-056 / RPC-057 already established and that RPC-058
must replicate.

## 1. Existing scheduler modules in codelet/napi/src/scheduler/

Pattern queried (rust):
```
pub fn $NAME($$$ARGS) { $$$BODY }
pub async fn $NAME($$$ARGS) { $$$BODY }
pub struct $NAME { $$$FIELDS }
pub trait $NAME { $$$BODY }
```

Findings (line counts via wc -l):
- `codelet/napi/src/scheduler/mod.rs` (24)
- `codelet/napi/src/scheduler/types.rs` (64) — `SchedulesFile`, `ScheduleEntry`, `AgentConfig`, `ShellConfig`, `EvaluationResult`
- `codelet/napi/src/scheduler/engine.rs` (353) — `spawn_scheduler`, `evaluate_and_run`, `evaluate_schedules`, plus the `get_session_count` / `get_live_session_ids` helpers that today reach into `crate::session_bindings::SessionManager::instance()` directly
- `codelet/napi/src/scheduler/state.rs` (152) — `SchedulerState`, `OverlapAction` (pure)
- `codelet/napi/src/scheduler/cron_utils.rs` (86) — `parse_cron`, `parse_timezone`, `should_trigger`, `MAX_SESSIONS` (pure)
- `codelet/napi/src/scheduler/trigger.rs` (175) — `trigger_and_update` (pure)
- `codelet/napi/src/scheduler/agent_job.rs` (102) — `trigger_agent_job_from_entry`, `trigger_agent_job` (reaches into `crate::session_bindings::SessionManager::instance().spawn_scheduled_session(...)`)
- `codelet/napi/src/scheduler/shell_job.rs` (76) — pure
- `codelet/napi/src/scheduler/catch_up.rs` (156) — pure
- `codelet/napi/src/scheduler/job_log.rs` (117) — pure
- `codelet/napi/src/scheduler/loop_store.rs` (280) — OUT OF SCOPE (lifts in RPC-059)

Three NAPI-bound call sites that the lift must replace with `SchedulerHooks`:
1. `engine.rs:208` — `crate::session_bindings::SessionManager::instance().session_count()` → `hooks.get_session_count()`
2. `engine.rs:221` — `crate::session_bindings::SessionManager::instance().live_session_ids()` → `hooks.get_live_session_ids()`
3. `agent_job.rs:67` — `crate::session_bindings::SessionManager::instance().spawn_scheduled_session(...)` → `hooks.spawn_scheduled_session(ScheduleTrigger)`

The `default_model` argument currently threads through `trigger_agent_job(name, project_path, config, default_model)` — under the lift, default_model is sourced from `hooks.default_model()`.

## 2. Existing CRUD surface in codelet/napi/src/schedule_handler.rs

Lives at codelet/napi/src/schedule_handler.rs (374 lines). Function inventory
(grep `^fn \|^pub fn `):
- `pub fn create_handler(project: String) -> ScheduleHandler` — keeps living in NAPI as the LLM tool-call handler
- `fn schedules_path`, `fn lock_dir_path` — path helpers
- `fn with_schedules_lock<F>` — wraps `codelet_common::file_lock::with_file_lock`
- `fn read_schedules_file`, `fn write_schedules_file` — JSON I/O with atomic temp+rename
- `fn validate_cron`, `fn validate_timezone`, `fn validate_add_request` — pure validation
- `fn handle_add`, `fn handle_list`, `fn handle_pause`, `fn handle_resume`, `fn handle_remove` — the five CRUD ops

All of the helpers above except `create_handler` are pure (no NAPI, no
SessionManager). They lift cleanly into `codelet/core/src/scheduler/crud.rs`.
The existing `create_handler` in NAPI becomes a thin adapter that re-uses
the lifted module via `codelet_core::scheduler::crud::{handle_add, handle_list, ...}`.

The signature change required by codelet-rpc-types: today these return
`ScheduleResult` from `codelet_tools::schedule::types`. After the lift they
return `Result<ScheduledJob, String>` (add/pause/resume), `Result<Vec<ScheduledJob>, String>`
(list), `Result<(), String>` (remove) so the RPC trait surface stays homogeneous
with RPC-056/057's `Result<*, String>` convention.

## 3. SessionManagerHandle trait surface to replicate

`codelet/core/src/session_manager_handle.rs` (1683 lines). RPC-058 must extend
the existing pattern at three sites:

### 3.1 Default-impl trait methods (around the blocklist_list spot at line 591)
```rust
fn schedule_add(&self, _job: ScheduledJob) -> Result<ScheduledJob, String> {
    Ok(ScheduledJob::default())
}
fn schedule_list(&self) -> Result<Vec<ScheduledJob>, String> { Ok(Vec::new()) }
fn schedule_pause(&self, _name: &str) -> Result<ScheduledJob, String> {
    Ok(ScheduledJob::default())
}
fn schedule_resume(&self, _name: &str) -> Result<ScheduledJob, String> {
    Ok(ScheduledJob::default())
}
fn schedule_remove(&self, _name: &str) -> Result<(), String> { Ok(()) }
```

### 3.2 StubSessionManagerHandle counter fields (around line 713)
```rust
schedule_add_calls: AtomicU64,
schedule_list_calls: AtomicU64,
schedule_pause_calls: AtomicU64,
schedule_resume_calls: AtomicU64,
schedule_remove_calls: AtomicU64,
schedules: Mutex<Vec<ScheduledJob>>,
```
Plus `pub fn schedule_*_calls(&self) -> u64` accessors and the trait override
that increments the counter and returns a stable Stub payload.

## 4. RPC service surface

### 4.1 codelet-rpc-types (1488 lines)
Add `ScheduledJob` flat struct alongside `BlocklistRuleInfo`, `MergeOutcome`,
`SessionChangesSummary`. Must be `napi(object)` compatible — String/Option<String>
fields only, no nested enum payloads. Mirrors the `ScheduleEntry` shape but
flattens agent/shell config into top-level role/prompt/command.

### 4.2 codelet/rpc/src/lib.rs (1521 lines)
At line 382 (`async fn blocklist_list() -> Vec<BlocklistRuleInfo>;` etc.) add:
```rust
async fn schedule_add(job: ScheduledJob) -> Result<ScheduledJob, String>;
async fn schedule_list() -> Result<Vec<ScheduledJob>, String>;
async fn schedule_pause(name: String) -> Result<ScheduledJob, String>;
async fn schedule_resume(name: String) -> Result<ScheduledJob, String>;
async fn schedule_remove(name: String) -> Result<(), String>;
```
And at line 1431 (`FspecServiceImpl::blocklist_list`) add the five routing
implementations that delegate to `self.inner.session_manager()` with safe
defaults when no handle is attached.

### 4.3 FspecBackend trait + transports
- `codelet/rpc-embedded/src/lib.rs` — `EmbeddedFspecBackend` forwards each method to its tarpc client.
- `codelet/rpc-server/src/lib.rs` — `WebSocketFspecBackend` forwards each method to its tarpc client.

## 5. codelet-sessions handle_impl

`codelet/sessions/src/handle_impl.rs` (1104 lines). RPC-058 must override the
five default-impl trait methods so they delegate to `codelet_core::scheduler::crud`
helpers, resolving repo_path at call time via `std::env::current_dir()`
(matching the blocklist_list pattern in RPC-056).

## 6. fspec-tui app dispatch surface

Existing files:
- `codelet/fspec-tui/src/app/dispatch.rs` (299 LoC — at the 300-LoC ceiling). The catch-all chain at line 285 today reads:
  ```
  if !self.try_dispatch_rpc022(&action)
      && !self.try_dispatch_rpc053(&action)
      && !self.try_dispatch_rpc054(&action)
      && !self.try_dispatch_rpc056(&action)
  {
      let _ = self.try_dispatch_rpc057(&action);
  }
  ```
  RPC-058 extends this chain to also try `try_dispatch_rpc058`.
- `codelet/fspec-tui/src/app/slash_parser.rs` (138 LoC). Today exposes
  `SlashCommandParse::{OpenModelDialog, OpenThinkingDialog, SetThinkingLevel, InvalidThinkingLevel, ClearRole, SetRole, NotASlashCommand}`.
  RPC-058 adds `ScheduleSubcommand(ScheduleSubcommand)`.
- `codelet/fspec-tui/src/app/dispatch_rpc057.rs` (252 LoC) — RPC-058's
  template: spawn tokio task, await backend round-trip, route via
  `Action::EmitSessionNotice`, silent no-op when no Tokio runtime, no-op when no current session.
- Existing dispatch_rpc0XX.rs files: 018, 020, 022, 024, 025, 026, 045,
  046, 050, 051, 052, 053, 054, 055, 056, 057 — RPC-058 follows the same naming.

## 7. ScheduleTrigger (codelet-core, replacement for direct SessionManager call)

Today `engine.rs` and `trigger.rs` pass `(name, project_path, entry, default_model)`
into `agent_job::trigger_agent_job`. Under the lift, the on_trigger hook
receives a structured `ScheduleTrigger { name, project_path, entry, job_type }`
so the engine never depends on SessionManager's public method shape.

## 8. Cross-transport parity tests

Pattern (RPC-056/057): integration test in codelet/tests/ that:
1. Constructs a shared `Arc<StubSessionManagerHandle>`
2. Wires it into both `EmbeddedFspecBackend` and `WebSocketFspecBackend`
3. Calls each of the five new methods via Embedded → asserts stub counter == 1
4. Calls each of the five new methods via WebSocket → asserts stub counter == 2
5. Asserts the byte-identical payloads returned by both transports

## Conclusion

The lift is mechanically isomorphic to RPC-054/055/056/057:
- 10 modules move from codelet/napi/src/scheduler/ → codelet/core/src/scheduler/
- 1 new module created: codelet/core/src/scheduler/crud.rs (lifted from schedule_handler.rs)
- 1 new trait declared: codelet-core::scheduler::SchedulerHooks
- 5 new RPC methods threaded through 6 layers (Handle / Stub / Service / Backend / Embedded / WebSocket)
- 1 new wire type: ScheduledJob
- 3 new fspec-tui app files: schedule_parser.rs, dispatch_rpc058.rs, slash_parser.rs additions
- 1 dispatch.rs catch-all extension

No structural surprises. All discovery done; ready to move to testing phase.
