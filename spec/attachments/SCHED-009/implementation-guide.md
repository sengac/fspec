# SCHED-009: Schedule AI Tool — Implementation Guide

## Overview

Implement an AI-callable `Schedule` tool so agents can manage schedules programmatically. Follow the **handler-delegated tool pattern** (Pattern B) used by AgentManager, SessionSearch, DeepSearch, and InjectSummary.

## Tool Pattern: Handler-Delegated

The three-layer architecture:

```
1. Tool Definition (Rust, codelet-tools)    — Schema + dispatch to handler
2. Handler Registry (Rust, codelet-tools)   — Global per-session handler map
3. Handler Implementation (NAPI layer)      — Actual logic, registered on session creation
```

### Layer 1: Tool Definition

Create `codelet/tools/src/schedule/mod.rs`:

```rust
use rig::tool::Tool;

pub struct ScheduleTool {
    session_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ScheduleArgs {
    /// Action: add, list, pause, resume, remove
    pub action: String,
    /// Schedule name (required for add, pause, resume, remove)
    pub name: Option<String>,
    /// Cron expression (required for add)
    pub cron: Option<String>,
    /// IANA timezone (required for add)
    pub timezone: Option<String>,
    /// Job type: "agent" or "shell" (required for add)
    pub job_type: Option<String>,
    /// Agent role (required for add with job_type=agent)
    pub role: Option<String>,
    /// Agent prompt (required for add with job_type=agent)
    pub prompt: Option<String>,
    /// Shell command (required for add with job_type=shell)
    pub command: Option<String>,
    /// Overlap policy: "skip" or "queue" (optional for add, default: skip)
    pub overlap_policy: Option<String>,
}

impl Tool for ScheduleTool {
    const NAME: &'static str = "Schedule";
    type Error = ToolError;
    type Args = ScheduleArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "Schedule".to_string(),
            description: "Manage scheduled jobs...".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ScheduleArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<String, ToolError> {
        let result = execute_schedule_command(self.session_id, args);
        Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
    }
}
```

### Layer 2: Handler Registry

Create `codelet/tools/src/schedule_handler.rs`:

```rust
use once_cell::sync::Lazy;
use std::sync::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

pub type ScheduleHandler = Arc<dyn Fn(ScheduleRequest) -> ScheduleResult + Send + Sync>;

static SCHEDULE_HANDLERS: Lazy<RwLock<HashMap<Uuid, ScheduleHandler>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub fn set_schedule_handler(session_id: Uuid, handler: Option<ScheduleHandler>) {
    let mut handlers = SCHEDULE_HANDLERS.write().unwrap();
    match handler {
        Some(h) => { handlers.insert(session_id, h); }
        None => { handlers.remove(&session_id); }
    }
}

pub fn execute_schedule_command(session_id: Uuid, request: ScheduleRequest) -> ScheduleResult {
    let handlers = SCHEDULE_HANDLERS.read().unwrap();
    match handlers.get(&session_id) {
        Some(handler) => handler(request),
        None => ScheduleResult::error("No schedule handler registered for this session"),
    }
}
```

### Layer 3: Handler Implementation

Register in `agent_loop()` (session_manager.rs), similar to AgentManager handler:

```rust
// In agent_loop setup
let schedule_handler = crate::schedule_handler::create_handler(project.clone());
codelet_tools::set_schedule_handler(session.id, Some(schedule_handler));
```

The handler delegates to the TypeScript layer via the same fspec command bridge pattern, OR calls directly into the Rust schedule persistence:

```rust
pub fn create_handler(project: String) -> ScheduleHandler {
    Arc::new(move |request: ScheduleRequest| -> ScheduleResult {
        // Read/write spec/schedules.json
        // Uses tokio::task::block_in_place + rt.block_on() for async-in-sync
        match request.action.as_str() {
            "add" => handle_add(&project, &request),
            "list" => handle_list(&project),
            "pause" => handle_pause(&project, &request.name.unwrap()),
            "resume" => handle_resume(&project, &request.name.unwrap()),
            "remove" => handle_remove(&project, &request.name.unwrap()),
            _ => ScheduleResult::error(&format!("Unknown action: {}", request.action)),
        }
    })
}
```

## Provider-Specific Facades

Following the facade pattern for multi-provider support:

| Provider | Tool Name | Schema Style |
|----------|-----------|-------------|
| Claude | `Schedule` | `schemars::schema_for!(ScheduleArgs)` |
| Gemini | `schedule_management` | Hand-crafted snake_case schema |
| OpenAI | `schedule` | `schemars::schema_for!(ScheduleArgs)` |
| Z.AI | `manage_schedule` | Hand-crafted with `additionalProperties: false` |

Create facade files:
- `codelet/tools/src/facade/schedule_facade.rs`
- Add `ScheduleToolFacade` trait + provider implementations
- Add `ScheduleToolFacadeWrapper` (or reuse the generic `FacadeToolWrapper`)

## Agent Builder Registration

Add the Schedule tool to each provider's agent builder:

```rust
// In codelet/providers/src/claude.rs
let agent = model.agent("...")
    // ... existing tools ...
    .tool(ScheduleTool::new(session_id))    // or facade-wrapped version
```

## Tool Response Format

```json
{
  "success": true,
  "action": "add",
  "schedule": {
    "name": "daily-tests",
    "cron": "0 6 * * *",
    "timezone": "Australia/Sydney",
    "jobType": "shell",
    "command": "npm test",
    "overlapPolicy": "skip",
    "status": "active"
  }
}
```

```json
{
  "success": true,
  "action": "list",
  "schedules": [
    { "name": "nightly-review", "cron": "0 2 * * *", "timezone": "Australia/Brisbane", "type": "agent", "status": "active", "lastRun": "2026-03-17T16:00:00Z", "nextRun": "2026-03-18T16:00:00Z" },
    { "name": "daily-sync", "cron": "0 9 * * 1-5", "timezone": "UTC", "type": "shell", "status": "active", "lastRun": null, "nextRun": "2026-03-18T09:00:00Z" }
  ]
}
```

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `codelet/tools/src/schedule/mod.rs` | Create | ScheduleTool definition |
| `codelet/tools/src/schedule_handler.rs` | Create | Handler registry |
| `codelet/tools/src/facade/schedule_facade.rs` | Create | Provider-specific facades |
| `codelet/tools/src/lib.rs` | Modify | Export schedule module + handler functions |
| `codelet/napi/src/schedule_handler.rs` | Create | Handler implementation |
| `codelet/napi/src/session_manager.rs` | Modify | Register handler in agent_loop |
| `codelet/providers/src/claude.rs` | Modify | Add ScheduleTool to agent builder |
| `codelet/providers/src/gemini.rs` | Modify | Add schedule facade |
| `codelet/providers/src/openai.rs` | Modify | Add schedule facade |
| `codelet/providers/src/zai.rs` | Modify | Add schedule facade |

## Key Constraints

- The tool must be available in ALL provider agent builders (Claude, Gemini, OpenAI, Z.AI, Codex)
- Validation (cron, timezone, name uniqueness) happens in the handler, error messages returned to the LLM
- The tool shares the same `spec/schedules.json` file as the slash commands — concurrent access via file locking
- Handler cleanup: `set_schedule_handler(session_id, None)` on session destruction
