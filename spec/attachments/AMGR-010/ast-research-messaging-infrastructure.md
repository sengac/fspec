# AMGR-010 AST Research — Agent Messaging Infrastructure

## Research Scope
Analyzed the existing messaging infrastructure in `codelet/napi/src/session_manager.rs` and `codelet/tools/src/agent_manager/` to understand what exists and what needs to be added for the `message` action.

## Existing Infrastructure (Already Built)

### IncomingMessage Type (line 265)
```rust
pub struct IncomingMessage {
    pub source_session_id: String,
    pub role_name: String,
    pub message: String,
    pub images: Option<Vec<BridgeImageData>>,
}
```

### BackgroundSession Channels (line 601)
- `incoming_message_tx: mpsc::Sender<IncomingMessage>` — capacity 16
- `incoming_message_rx: Mutex<mpsc::Receiver<IncomingMessage>>` — consumed in agent_loop

### receive_incoming_message() (line 1129)
```rust
pub fn receive_incoming_message(&self, msg: IncomingMessage) -> Result<(), ...> {
    self.incoming_message_tx.try_send(msg) // non-blocking
}
```

### agent_loop tokio::select! (line 3835+)
Branch 2 already handles `incoming_message_rx.recv()` — messages are formatted via `format_incoming_message()` as `[SUPERVISOR: role | Session: id] message` before being sent to LLM.

## AgentManagerTool — Current Actions (types.rs)

```rust
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentManagerAction {
    Spawn { role: Option<String> },
    List,
    GetStatus { session_id: String },
    Close { session_id: String },
}

pub enum AgentManagerResult {
    Spawned { session_id: String },
    Listed { sessions: Vec<SessionEntry> },
    Status { ... },
    Closed { session_id: String },
    Error { code: String, message: String },
}
```

## What Needs to Be Added

### types.rs
1. Add `Message { session_id: String, message: String }` variant to `AgentManagerAction`
2. Add `MessageDelivered { session_id: String }` variant to `AgentManagerResult`

### agent_manager_handler.rs
3. Add `Message` match arm in `create_handler()` closure:
   - Parse target UUID from session_id string
   - Look up target session from SessionManager
   - Get sender's role from calling session (Option<String>)
   - Call `target.receive_incoming_message(IncomingMessage { ... })`
   - Handle `TrySendError::Full` → delivery_failed error
   - Handle session not found → session_not_found error

### Serde Validation
4. Serde tagged dispatch already handles parameter validation — missing fields cause deserialization errors which map to `invalid_parameter` at the tool level

## Key Finding
The messaging infrastructure is **already complete** — incoming_message channels, agent_loop select branch, message formatting. AMGR-010 only needs to:
1. Add the `Message` variant to the action enum
2. Add the handler logic to bridge the AgentManager tool call to `receive_incoming_message()`

Estimated complexity: **5 points** — straightforward extension of existing handler pattern.
