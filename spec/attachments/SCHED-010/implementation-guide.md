# SCHED-010: Bridge Notifications & Error Handling — Implementation Guide

## Overview

Add a `StreamChunk::ScheduleEvent` variant for schedule lifecycle events. Bridge notifications are sent on job failure or completion if a bridge (e.g., Telegram) is connected. Also handle blocklist failures — if a scheduled job hits a blocked tool, fail immediately.

## New StreamChunk Variant

### Adding `ScheduleEvent`

Following the pattern documented in the codebase (4 places to modify):

#### 1. Add variant to `StreamChunk` enum (`codelet/napi/src/types.rs`)

```rust
#[napi(string_enum = "type")]
pub enum StreamChunk {
    // ... existing 20 variants ...
    
    /// Schedule lifecycle event (trigger, completion, failure, skip, defer)
    ScheduleEvent {
        #[napi(js_name = "eventType")]
        event_type: String,           // "triggered" | "completed" | "failed" | "skipped" | "deferred" | "queued"
        #[napi(js_name = "scheduleName")]
        schedule_name: String,
        #[napi(js_name = "jobType")]
        job_type: String,             // "agent" | "shell"
        #[napi(js_name = "message")]
        message: Option<String>,      // Human-readable description
        #[napi(js_name = "exitCode")]
        exit_code: Option<i32>,       // Shell jobs only
        #[napi(js_name = "stderr")]
        stderr: Option<String>,       // Shell jobs only, on failure
        #[napi(js_name = "sessionId")]
        session_id: Option<String>,   // Agent jobs: the spawned session ID
    },
}
```

#### 2. Add constructor method

```rust
impl StreamChunk {
    pub fn schedule_event(
        event_type: String,
        schedule_name: String,
        job_type: String,
        message: Option<String>,
        exit_code: Option<i32>,
        stderr: Option<String>,
        session_id: Option<String>,
    ) -> Self {
        Self::ScheduleEvent {
            event_type, schedule_name, job_type, message,
            exit_code, stderr, session_id,
        }
    }
}
```

#### 3. Add JSON serialization arm in `to_json_value()`

```rust
Self::ScheduleEvent { event_type, schedule_name, job_type, message, exit_code, stderr, session_id } => json!({
    "type": "scheduleEvent",
    "eventType": event_type,
    "scheduleName": schedule_name,
    "jobType": job_type,
    "message": message,
    "exitCode": exit_code,
    "stderr": stderr,
    "sessionId": session_id,
}),
```

#### 4. Handle in correlation tracking methods

`ScheduleEvent` does NOT participate in correlation tracking (it's a system event, not conversation content), so add it to the pass-through arms in `with_correlation_id()` and `with_observed_correlation_ids()`.

## Event Emission Points

### When to Emit ScheduleEvents

| Event | When | Key Fields |
|-------|------|------------|
| `triggered` | Job starts executing | schedule_name, job_type, session_id (agent only) |
| `completed` | Job finishes successfully | schedule_name, job_type, exit_code=0 (shell) |
| `failed` | Job fails | schedule_name, job_type, exit_code (shell), stderr (shell), message (error detail) |
| `skipped` | Overlap policy=skip, previous still running | schedule_name, message |
| `deferred` | MAX_SESSIONS reached | schedule_name, message (e.g., "10/10 sessions") |
| `queued` | Overlap policy=queue, previous still running | schedule_name |

### Emission in Scheduler Engine

```rust
// In the scheduler engine (SCHED-003)
async fn trigger_agent_job(name: &str, schedule: &ScheduleEntry, project_path: &str) {
    // Emit triggered event
    emit_schedule_event(StreamChunk::schedule_event(
        "triggered".to_string(),
        name.to_string(),
        "agent".to_string(),
        Some(format!("Spawning agent session for '{}'", name)),
        None, None,
        Some(session_id.to_string()),
    ));
    
    // ... spawn session ...
    
    // On completion (detected by session status change)
    emit_schedule_event(StreamChunk::schedule_event(
        "completed".to_string(),
        name.to_string(),
        "agent".to_string(),
        Some(format!("Agent session '{}' completed", name)),
        None, None,
        Some(session_id.to_string()),
    ));
}
```

### How to Emit

Schedule events need to reach the bridge. Two options:

**Option A: Emit via a session's handle_output (Recommended)**

If the scheduled job spawns a session, emit through that session's `handle_output()`. The event flows through the normal broadcast path to all connected bridges.

**Option B: Direct broadcast**

Create a global schedule event broadcast channel that the bridge relay subscribes to independently. More complex but works for events that don't have an associated session (like "skipped").

For events without a session (skipped, deferred), use the **primary/active session's** broadcast channel, or create a dedicated system event channel.

## Bridge Relay Integration

### Outbound Processing

In `bridge_relay.rs`'s `process_outbound_chunk()`, the `ScheduleEvent` variant flows through the default path (no special filtering needed — unlike `FspecCommandRequest` which is filtered out).

The bridge consumer (e.g., Telegram bot) receives:

```json
{
  "type": "chunk",
  "session_id": "...",
  "data": {
    "type": "scheduleEvent",
    "eventType": "failed",
    "scheduleName": "daily-sync",
    "jobType": "shell",
    "message": "Shell command failed",
    "exitCode": 1,
    "stderr": "npm ERR! missing script: sync",
    "sessionId": null
  }
}
```

### Telegram Formatting

The Telegram bridge (external) formats schedule events as:

```
🔴 Schedule Failed: daily-sync
Type: shell
Exit Code: 1
Stderr: npm ERR! missing script: sync
```

```
✅ Schedule Completed: nightly-review
Type: agent
Session: abc123
```

This is handled in the Telegram bot code (external to this repo), but the `ScheduleEvent` JSON schema must be documented for bridge implementors.

## Blocklist Failure Handling

### How Blocklists Work Today

The blocklist system (BLOCK-002) is checked at the tool dispatch layer. When a tool call is blocked:

```rust
// In the tool dispatch path
if is_tool_blocked(tool_name, &session.blocklist) {
    return Err(ToolError::Blocked {
        tool: tool_name,
        message: format!("Tool '{}' is blocked by session blocklist", tool_name),
    });
}
```

### For Scheduled Sessions

Scheduled sessions inherit the same blocklist mechanism. When a blocked tool error occurs:

1. The `ToolError::Blocked` is returned to the agent loop
2. The agent loop treats it as a tool error and includes it in the conversation
3. **For scheduled sessions specifically**: detect that this is a schedule-triggered session and emit a `failed` ScheduleEvent

```rust
// In the agent loop error handling for scheduled sessions
if session.schedule_triggered.load(Ordering::Relaxed) {
    if matches!(tool_error, ToolError::Blocked { .. }) {
        emit_schedule_event(StreamChunk::schedule_event(
            "failed".to_string(),
            session.schedule_name.read().await.clone().unwrap_or_default(),
            "agent".to_string(),
            Some(format!("Blocked tool: {}", tool_error)),
            None, None,
            Some(session.id.to_string()),
        ));
        // Terminate the agent loop for this scheduled session
        break;
    }
}
```

**Key decision**: When a blocked tool is hit, the scheduled session **fails immediately** (breaks out of agent loop), rather than letting the agent try to work around it. This matches the SCHED-001 rule.

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `codelet/napi/src/types.rs` | Modify | Add `ScheduleEvent` variant to `StreamChunk` |
| `codelet/napi/src/scheduler/events.rs` | Create | Event emission helpers |
| `codelet/napi/src/session_manager.rs` | Modify | Blocklist failure detection for scheduled sessions |
| `codelet/napi/src/bridge_relay.rs` | Verify | Ensure ScheduleEvent passes through outbound (should work by default) |

## Key Constraints

- ScheduleEvent must reach ALL connected bridges — not just the session's supervisor broadcast
- Blocked tool = immediate job failure + ScheduleEvent emission
- Shell failures include exit_code and stderr in the event
- Agent completions include the session_id in the event for cross-referencing with SessionSearch
- Events must be serializable to JSON via `to_json_value()` for bridge relay
