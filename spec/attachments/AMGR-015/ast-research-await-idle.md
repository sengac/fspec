# AMGR-015 Research: AgentManager `await_idle` Action

## Research Summary

Five parallel DeepSearch agents investigated the codebase to answer how to add an efficient
`await_idle` action to the AgentManager tool. This document synthesizes their findings.

---

## 1. Current AgentManager Architecture

### Module Layout
```
codelet/tools/src/agent_manager/
├── mod.rs       — Tool struct, Rig Tool impl, JSON schema, call() entry point
├── types.rs     — Action enum, args, context references, result types
├── handler.rs   — Per-session handler registry (static HashMap<Uuid, Handler>)
└── tests.rs     — Comprehensive test suite (~2000+ lines)
```

### Current Actions
The `AgentManagerAction` enum (serde-tagged union on `"action"` field):

| Action | Purpose | Async? |
|--------|---------|--------|
| `Spawn` | Create subordinate session | No (sync closure) |
| `List` | List all sessions with relationships | No |
| `GetStatus` | One-shot status snapshot for a session | No |
| `Close` | Terminate subordinate (spawner only) | No |
| `Message` | Send message with optional context | No |
| `SetRole` | Set/clear system prompt overlay | No |

**Key observation:** ALL current actions are synchronous. The handler type is:
```rust
pub type AgentManagerHandler =
    Arc<dyn Fn(AgentManagerAction, Uuid) -> AgentManagerResult + Send + Sync>;
```

The `call()` method in `mod.rs` calls `execute_agent_manager()` which is synchronous — it looks
up the handler in a static `RwLock<HashMap<Uuid, Handler>>` and invokes it directly.

### Handler Registration
The NAPI layer (`session_manager.rs`) registers a closure per session:
```rust
set_agent_manager_handler(session_id, Some(Arc::new(move |action, calling_id| {
    // Direct access to SessionManager — no TypeScript round-trip
    match action {
        Spawn { .. } => handle_spawn(...),
        List => handle_list(...),
        // ...
    }
})));
```

---

## 2. Session State Machine

### SessionStatus (5 states)
```rust
#[repr(u8)]
pub enum SessionStatus {
    Idle = 0,        // Default — waiting for input
    Running = 1,     // Processing a prompt
    Interrupted = 2, // User pressed Esc
    Paused = 3,      // Waiting for HITL response
    Compacting = 4,  // Context compaction in progress
}
```

Stored as `AtomicU8` — lock-free, read with `Ordering::Acquire`.

### State Transitions to Idle
- **Stream Done + no compaction** → `set_status(Idle)` (via `should_idle_on_done()` guard)
- **CompactionComplete** → `set_status(Idle)`
- **CompactionFailed** → `set_status(Idle)`

### The `set_status()` Method (Critical for await_idle)
Every status change triggers THREE side-effects:
1. **Atomic swap** (`Ordering::AcqRel`)
2. **StreamChunk emission** — sends `StreamChunk::SessionStateChange { state }` via `handle_output()`
3. **Metadata broadcast** — calls `broadcast_metadata_update()` for relay clients

The StreamChunk goes through `handle_output()` which:
- Pushes to output buffer
- **Sends to `supervisor_broadcast` channel** (tokio `broadcast::Sender<StreamChunk>`, capacity 256)
- Forwards to `GLOBAL_CHUNK_CALLBACK` for TypeScript

---

## 3. Notification Infrastructure (What We Can Leverage)

### Supervisor Broadcast Channel (PRIMARY MECHANISM)
Each `BackgroundSession` has:
```rust
supervisor_broadcast: broadcast::Sender<StreamChunk>,
```
- Capacity: 256 chunks
- Subscribe via: `session.subscribe_to_stream() → broadcast::Receiver<StreamChunk>`
- **Every `set_status()` call emits `SessionStateChange` through this channel**
- Late subscribers start from current position (no replay needed — we only care about future transitions)

**This is the ideal mechanism for await_idle.** Subscribe to the broadcast, filter for
`SessionStateChange(Idle)`, and resolve. Zero polling.

### Other Channels (Not needed for this feature)
| Channel | Purpose | Relevant? |
|---------|---------|-----------|
| `incoming_message_tx/rx` | Inter-session messages | No |
| `interrupt_notify: Notify` | Esc interrupt wake-up | Yes — for cancellation |
| `pause_response_tx/rx` | HITL blocking | No |
| `GLOBAL_CHUNK_CALLBACK` | Rust→TypeScript | No |

### ChainOfCommand Graph
Tracks supervisor↔subordinate relationships. Methods:
- `get_subordinates(supervisor_id) → Vec<Uuid>`
- `get_supervisors(subordinate_id) → Vec<Uuid>`
- `add_supervisor(sub, sup)` with BFS cycle detection

Not directly needed for `await_idle`, but could be used for validation (e.g., "can only await
sessions you supervise" — though currently message sending has no such restriction).

---

## 4. Existing Wait/Poll Patterns Analysis

### Pattern: Loop Store Idle Check (Scheduler)
```rust
// loop_store.rs — Skip-when-busy pattern
let idle_check: IdleCheckFn = Arc::new(move |_| {
    Box::pin(async move { s.get_status() == SessionStatus::Idle })
});
```
- Fixed interval poll (configurable, ≥1s)
- Skip-when-busy (doesn't queue, just retries next interval)
- **ANTI-PATTERN for await_idle** — wasteful polling, no notification

### Pattern: Bridge Reconnection (Exponential Backoff)
```rust
// bridge_relay.rs — Classic backoff
let mut delay = Duration::from_secs(1);
loop {
    match connect().await { ... }
    sleep(delay).await;
    delay = min(delay * 2, Duration::from_secs(30));
}
```
- Not applicable — this is error recovery, not state waiting

### Pattern: Test Fixtures (Deadline Poll)
```rust
// bridge_test_fixtures.rs — Best existing "wait" pattern
pub async fn wait_for_messages(&self, count: usize, timeout_secs: u64) {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    while Instant::now() < deadline {
        if messages.len() >= count { return messages; }
        sleep(Duration::from_millis(50)).await;
    }
}
```
- 50ms poll with deadline — functional but suboptimal
- **We can do better** with broadcast subscription

### Key Finding: NO Agent-Level Wait Exists
There is **no mechanism** for a supervisor to efficiently wait for a subordinate to become idle.
`get_status` is purely a one-shot snapshot. The LLM must poll across multiple tool calls.

---

## 5. Proposed Architecture

### 5.1. New Action Variant
```rust
// types.rs
pub enum AgentManagerAction {
    // ... existing variants ...
    AwaitIdle {
        session_id: SessionIdParam,  // String or Vec<String>
        timeout: Option<u64>,        // Seconds, default 300
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SessionIdParam {
    Single(String),
    Multiple(Vec<String>),
}
```

### 5.2. New Result Variant
```rust
// types.rs
pub enum AgentManagerResult {
    // ... existing variants ...
    AwaitResult {
        results: Vec<AwaitSessionResult>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwaitSessionResult {
    pub session_id: String,
    pub status: AwaitOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AwaitOutcome {
    Idle,        // Session reached idle state
    TimedOut,    // Deadline expired before idle
    Destroyed,   // Session was destroyed during wait
    Interrupted, // Calling session was interrupted (Esc)
}
```

### 5.3. Async Handler Path

The current handler type is synchronous:
```rust
type AgentManagerHandler = Arc<dyn Fn(Action, Uuid) -> Result + Send + Sync>;
```

**Option B (recommended):** Add a parallel async handler:
```rust
pub type AgentManagerAsyncHandler =
    Arc<dyn Fn(AgentManagerAction, Uuid) -> Pin<Box<dyn Future<Output = AgentManagerResult> + Send>>
        + Send + Sync>;

static AGENT_MANAGER_ASYNC_HANDLERS: Lazy<RwLock<HashMap<Uuid, AgentManagerAsyncHandler>>> = ...;
```

In `call()`:
```rust
async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    // Pre-tool hook check...
    
    let result = match &args.action {
        AgentManagerAction::AwaitIdle { .. } => {
            execute_agent_manager_async(self.session_id, args.action).await
        }
        _ => execute_agent_manager(self.session_id, args.action),
    };
    
    serde_json::to_string_pretty(&result).map_err(...)
}
```

### 5.4. Wait Implementation (in NAPI handler)

```rust
// agent_manager_handler.rs (pseudocode)
async fn handle_await_idle(
    session_manager: &SessionManager,
    session_ids: Vec<Uuid>,
    timeout_secs: u64,
    calling_session: &BackgroundSession,
) -> AgentManagerResult {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let interrupt = calling_session.interrupt_notify.clone();
    
    // Phase 1: Validate all sessions exist, resolve already-idle ones
    let mut results: Vec<AwaitSessionResult> = Vec::new();
    let mut pending: Vec<(Uuid, broadcast::Receiver<StreamChunk>)> = Vec::new();
    
    for id in session_ids {
        match session_manager.get_session(&id) {
            None => return AgentManagerResult::Error { ... session_not_found ... },
            Some(session) => {
                if session.get_status() == SessionStatus::Idle {
                    results.push(AwaitSessionResult { session_id: id.to_string(), status: Idle });
                } else {
                    pending.push((id, session.subscribe_to_stream()));
                }
            }
        }
    }
    
    if pending.is_empty() {
        return AgentManagerResult::AwaitResult { results };
    }
    
    // Phase 2: Wait for pending sessions
    let mut join_set = JoinSet::new();
    for (id, mut rx) in pending {
        let session_mgr = session_manager.clone();
        join_set.spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(StreamChunk::SessionStateChange { state }) if state == "idle" => {
                        return (id, AwaitOutcome::Idle);
                    }
                    Err(RecvError::Closed) => {
                        return (id, AwaitOutcome::Destroyed);
                    }
                    Err(RecvError::Lagged(n)) => continue, // Skip lagged, keep listening
                    _ => continue, // Other chunk types — ignore
                }
            }
        });
    }
    
    // Phase 3: Collect results with timeout and interrupt
    let remaining = deadline.duration_since(Instant::now());
    tokio::select! {
        _ = tokio::time::sleep(remaining) => {
            // Timeout — mark all remaining as timed_out
            join_set.abort_all();
            // ... collect results ...
        }
        _ = interrupt.notified() => {
            // User interrupted — mark all remaining as interrupted
            join_set.abort_all();
            // ... collect results ...
        }
        // Or individual tasks complete via join_set.join_next()
    }
    
    AgentManagerResult::AwaitResult { results }
}
```

### 5.5. JSON Schema Update (mod.rs)

Add to the tool definition:
```json
{
  "action": {
    "enum": ["spawn", "list", "get_status", "close", "message", "set_role", "await_idle"]
  },
  "session_id": {
    "oneOf": [
      { "type": "string" },
      { "type": "array", "items": { "type": "string" } }
    ],
    "description": "Target session ID(s). For await_idle: one or more sessions to wait for."
  },
  "timeout": {
    "type": ["integer", "null"],
    "description": "Maximum wait time in seconds (default: 300). Only for await_idle."
  }
}
```

---

## 6. Files Requiring Modification

| File | Changes |
|------|---------|
| `codelet/tools/src/agent_manager/types.rs` | Add `AwaitIdle` variant, `SessionIdParam`, `AwaitSessionResult`, `AwaitOutcome` |
| `codelet/tools/src/agent_manager/mod.rs` | Update JSON schema, add async dispatch path in `call()` |
| `codelet/tools/src/agent_manager/handler.rs` | Add async handler registry + `execute_agent_manager_async()` |
| `codelet/napi/src/agent_manager_handler.rs` | Implement `handle_await_idle()` with broadcast subscription |
| `codelet/napi/src/session_manager.rs` | Register async handler alongside sync handler, expose `subscribe_to_stream()` to handler |
| `codelet/tools/src/agent_manager/tests.rs` | Add comprehensive tests for all await_idle scenarios |

---

## 7. Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Handler type change breaks existing actions | Medium | Option B (separate async handler) avoids changing sync path |
| Broadcast channel lagging (256 capacity) | Low | `Lagged` error just means we retry — no data loss for state detection |
| Deadlock if awaiting self | Low | Self-await would timeout — session can't be both awaiting and idle |
| Session destroyed after validate but before subscribe | Low | Race window is tiny; broadcast `Closed` error handles it |
| Interrupt_notify is single-use (`notify_one`) | Medium | May need `notify_waiters()` or a shared `AtomicBool` check in the select |

---

## 8. Example Tool Call (LLM Perspective)

```json
{
  "action": "await_idle",
  "session_id": ["abc-123", "def-456", "ghi-789"],
  "timeout": 120
}
```

Response:
```json
{
  "results": [
    { "session_id": "abc-123", "status": "idle" },
    { "session_id": "def-456", "status": "idle" },
    { "session_id": "ghi-789", "status": "timed_out" }
  ]
}
```

---

## 9. Why This Approach (Not Polling)

The current alternative for an LLM supervisor wanting to wait:
```
Loop:
  1. Call AgentManager(action='get_status', session_id='abc')   → "running"
  2. Call Bash(command='sleep 5')                                → wait
  3. Repeat from 1
```

**Problems with polling:**
- Each poll is a full LLM tool-call round-trip (~1-5 seconds each)
- Sleep via Bash is imprecise and blocks the calling session's tool execution
- Wastes tokens — each poll generates input/output tokens
- No way to efficiently await multiple sessions simultaneously
- Creates visual noise in conversation (dozens of get_status calls)

**`await_idle` is better:**
- Single tool call blocks efficiently using OS-level async notification
- Zero CPU usage while waiting (broadcast recv is a futex/epoll wait)
- Handles multiple sessions in one call
- Returns structured results showing which finished and which timed out
- Interruptible (Esc cancellation)
- Clean conversation — one call, one result
