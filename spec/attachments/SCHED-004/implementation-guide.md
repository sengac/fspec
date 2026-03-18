# SCHED-004: Agent Job Execution — Implementation Guide

## Overview

When the scheduler engine (SCHED-003) determines an agent-type schedule should fire, spawn a full subordinate session via AgentManager with the configured role, initial prompt, full tool access, and the user's default model/provider.

## How AgentManager Spawning Works Today

The existing subordinate session spawning flow (from `codelet/napi/src/agent_manager_handler.rs`):

### Handler Registration

Each session registers an `AgentManagerHandler` in `agent_loop()` (session_manager.rs:4349):

```rust
let agent_manager_handler = crate::agent_manager_handler::create_handler(
    project, spawner_model_string
);
codelet_tools::set_agent_manager_handler(session.id, Some(agent_manager_handler));
```

### `handle_spawn()` Flow (agent_manager_handler.rs:71)

1. Generate a new `Uuid` for the subordinate
2. **Create a persistence manifest** — ensures session history is saved and searchable via SessionSearch
3. Call `session_manager.create_session_with_id()` via `tokio::task::block_in_place` + `rt.block_on()`
4. Register the ChainOfCommand relationship: `session_manager.add_supervisor(subordinate_id, spawner_id)`
5. Set role on the subordinate if provided
6. Return `AgentManagerResult::Spawned { session_id }`

### What `create_session_with_id()` Does

- Checks `sessions.len() >= MAX_SESSIONS` — returns error if full
- Creates a `BackgroundSession` with a new `agent_loop` tokio task
- The `BackgroundSession` struct has ~30 fields including input/output channels, token counters, role overlay, interrupt mechanism, etc.
- The session inherits the spawner's model string

## What the Scheduler Needs to Do Differently

The scheduler is NOT an agent session — it's a tokio task. It needs to spawn subordinates **without** going through `AgentManagerTool`'s handler pattern (which requires a session_id for the "spawner").

### Option A: Direct SessionManager API (Recommended)

Add a new method to `SessionManager`:

```rust
impl SessionManager {
    pub async fn spawn_scheduled_session(
        &self,
        name: &str,           // Schedule name (used as session name)
        project: &str,        // Project path
        role: &str,           // Agent role
        prompt: &str,         // Initial prompt
        model: &str,          // Model string (from user defaults)
    ) -> Result<Uuid, SessionError> {
        let id = Uuid::new_v4();
        
        // 1. Create persistence manifest
        // 2. Create session (reuse create_session_with_id internals)
        // 3. Set role
        // 4. Mark as schedule-triggered (new flag on BackgroundSession)
        // 5. Send initial prompt via input_tx
        
        Ok(id)
    }
}
```

### Schedule-Triggered Indicator

Add a flag to `BackgroundSession` so the TUI can show a clock icon:

```rust
pub struct BackgroundSession {
    // ... existing fields ...
    /// Whether this session was spawned by the scheduler
    pub schedule_triggered: AtomicBool,
    /// Name of the schedule that triggered this session (if any)
    pub schedule_name: RwLock<Option<String>>,
}
```

The TUI session list (TypeScript side) reads this flag and renders a 🕐 icon.

### Session Naming

Schedule-triggered sessions should have identifiable names:

```
"[scheduled] nightly-review — 2026-03-18 02:00"
```

This makes them distinguishable in:
- TUI session list
- SessionSearch results
- `AgentManager(action='list')` output

### Model Resolution

The schedule doesn't store a model — it uses the user's default at execution time. The scheduler needs to resolve this:

```rust
// Get the user's current default model
// This is set during app initialization from config
fn get_default_model() -> String {
    // Read from the active session's model, or from config
    // The model string looks like "anthropic/claude-opus-4-6"
}
```

### Full Tool Access

Scheduled agent sessions get the same tools as any other session — Read, Write, Edit, Bash, Grep, Glob, Ls, AstGrep, AstGrepRefactor, Fspec, Bridge, WebSearch, ConnectMCP, SessionSearch, DeepSearch, AgentManager, etc.

This is automatic — `create_session_with_id()` registers all tools as part of the agent builder.

## TUI Integration

### Session List Icon

In the TypeScript TUI, the session list component needs to check the `schedule_triggered` flag:

```typescript
// In session list rendering
const icon = session.scheduleTriggered ? '🕐' : '💬';
const label = `${icon} ${session.name}`;
```

### NAPI Bridge

Expose the new `BackgroundSession` fields via NAPI:

```rust
#[napi]
pub fn session_is_scheduled(session_id: String) -> bool {
    // ...
}

#[napi]
pub fn session_schedule_name(session_id: String) -> Option<String> {
    // ...
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `codelet/napi/src/scheduler/agent_job.rs` | Create | Agent job spawning logic |
| `codelet/napi/src/session_manager.rs` | Modify | Add `spawn_scheduled_session()`, add schedule flags to `BackgroundSession` |
| `codelet/napi/src/lib.rs` | Modify | Add NAPI exports for schedule metadata |
| `src/tui/components/SessionList.tsx` | Modify | Add clock icon for scheduled sessions |

## Key Constraints

- Scheduled sessions count toward MAX_SESSIONS (10) — if full, defer (handled by SCHED-006)
- The session must have full persistence — session history saved to disk, searchable via SessionSearch
- The initial prompt is sent as the first user message after session creation
- The agent loop runs to natural completion (stop point) — no forced termination
- After completion, update `lastRunAt` and `lastRunStatus` in spec/schedules.json
