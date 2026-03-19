# SCHED-004: Agent Job Execution — AST Research

## Key Patterns Identified

### 1. Scheduler Engine Stub (to be replaced)
**File:** `codelet/napi/src/scheduler/engine.rs:267`
```rust
async fn trigger_agent_job(name: &str) -> Result<(), anyhow::Error> {
    info!("Agent job triggered: {} (stub — SCHED-004)", name);
    Ok(())
}
```
Currently takes only `name` — needs `project_path`, `ScheduleEntry`, and access to `SessionManager`.

### 2. SessionManager Structure
**File:** `codelet/napi/src/session_manager.rs:3061`
```rust
pub struct SessionManager {
    sessions: RwLock<IndexMap<Uuid, Arc<BackgroundSession>>>,
    chain_of_command: ChainOfCommand,
    active_session_id: RwLock<Option<Uuid>>,
    scheduler_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
}
```
**Gap:** No `default_model` field — needs to be added for scheduler model resolution.

### 3. BackgroundSession Structure
**File:** `codelet/napi/src/session_manager.rs:468`
Has ~30 fields. Missing `schedule_triggered` and `schedule_name` fields needed for TUI identification.

### 4. create_session_with_id Signature
**File:** `codelet/napi/src/session_manager.rs:3107`
```rust
pub async fn create_session_with_id(&self, id: &str, model: &str, project: &str, name: &str) -> Result<()>
```
Key parameters: `id` (UUID string), `model` (e.g. "anthropic/claude-sonnet-4"), `project`, `name`.

### 5. Role Setting
**File:** `codelet/napi/src/session_manager.rs:919`
```rust
pub fn set_role(&self, role: String) { ... }
```
Called on `BackgroundSession` to set role overlay.

### 6. Sending Prompt Input
**File:** `codelet/napi/src/session_manager.rs:1181-1182`
```rust
self.input_tx
    .try_send(PromptInput { input, thinking_config })
```
`PromptInput` struct at line 81 — has `input: String` and `thinking_config` fields.

### 7. NAPI Binding Pattern for Session Metadata
**File:** `codelet/napi/src/session_manager.rs:5710`
```rust
pub fn session_set_role(session_id: String, role_name: String) -> Result<()> { ... }
```
Follow this pattern for `session_is_scheduled()` and `session_schedule_name()`.

### 8. SESSION_MANAGER Global
The lazy_static `SESSION_MANAGER` in `lib.rs` provides global access — scheduler can use this to call `spawn_scheduled_session`.

## Implementation Plan

1. **Add `default_model` to `SessionManager`** — `RwLock<Option<String>>`
2. **Add schedule fields to `BackgroundSession`** — `schedule_triggered: AtomicBool`, `schedule_name: RwLock<Option<String>>`
3. **Create `agent_job.rs`** — new module with `trigger_agent_job` implementation
4. **Add `spawn_scheduled_session` to `SessionManager`** — wraps `create_session_with_id` + role + schedule flags + prompt
5. **Wire into `engine.rs`** — replace stub with call to `agent_job::trigger_agent_job`
6. **Add NAPI bindings** — `session_is_scheduled`, `session_schedule_name`
