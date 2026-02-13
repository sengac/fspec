# BRIDGE-001 Implementation Notes (REVISED)

## ⚠️ ARCHITECTURE CORRECTION

**The original BRIDGE-001 implementation was WRONG!**

The Bridge should be a **Rig Tool** (like WebSearch, Read, Bash, Fspec) that the **AI agent calls** to manage connections - NOT NAPI bindings called from TypeScript.

---

## Correct Architecture

### Bridge is a Rig Tool

```rust
// codelet/tools/src/bridge.rs (NOT codelet/napi/src/bridge.rs!)

impl Tool for BridgeTool {
    const NAME: &'static str = "Bridge";
    
    type Error = ToolError;
    type Args = BridgeRequest;
    type Output = BridgeResult;
    
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        match args.action {
            BridgeAction::Connect { url } => { /* ... */ }
            BridgeAction::Disconnect { url } => { /* ... */ }
            BridgeAction::List => { /* ... */ }
        }
    }
}
```

### Agent Usage

The agent calls the Bridge tool directly:

```xml
<tool_use>
<name>Bridge</name>
<parameters>
{"action": {"type": "connect", "url": "ws://localhost:8080/telegram"}}
</parameters>
</tool_use>
```

Tool Result:
```
Connected to ws://localhost:8080/telegram. Session output will be relayed to this endpoint.
```

---

## Key Differences from Old Implementation

| Aspect | Old (Wrong) | Correct |
|--------|-------------|---------|
| Location | `codelet/napi/src/bridge.rs` | `codelet/tools/src/bridge.rs` |
| Interface | NAPI bindings (TypeScript calls) | Rig Tool (Agent calls) |
| Control | TUI/commands | AI agent decides |
| Pattern | Like SessionManager | Like WebSearchTool |

---

## How BRIDGE-002 Connects

1. **Agent** calls `Bridge(action: 'connect', url: 'ws://localhost:8080/telegram')`
2. **Bridge Tool** spawns WebSocket client connection to endpoint
3. **Bridge Tool** subscribes to session's `watcher_broadcast` channel
4. **Telegram Endpoint** (BRIDGE-002) receives StreamChunks as JSON
5. **Telegram Endpoint** formats and sends to Telegram
6. **Telegram Endpoint** receives Telegram messages, sends back to Bridge
7. **Bridge Tool** injects messages into session via `watcher_input_tx`

---

## Files to DELETE (Old Wrong Implementation)

- `codelet/napi/src/bridge.rs` - NAPI bindings approach (WRONG)
- `codelet/napi/tests/bridge_core_test.rs` - Tests for wrong approach

## Files to CREATE (Correct Implementation)

- `codelet/tools/src/bridge.rs` - Rig Tool implementation
- `codelet/tools/src/facade/bridge_wrapper.rs` - FacadeWrapper for session context
- `codelet/tools/tests/bridge_test.rs` - Tests for Rig Tool

---

## Message Formats (Unchanged)

**Outbound (Session → Endpoint):**
```json
{
  "type": "chunk",
  "session_id": "uuid-here",
  "data": {
    "type": "text|thinking|tool_call|tool_result|done|error",
    ...payload
  }
}
```

**Inbound (Endpoint → Session):**
```json
{
  "type": "input",
  "session_id": "uuid-here",
  "message": "user input text"
}
```

---

## Session Context Injection

The Bridge tool needs session context (like FspecTool needs project_root):

```rust
pub struct BridgeToolFacadeWrapper {
    inner: BridgeTool,
    session_id: Uuid,
    stream_rx_factory: Arc<dyn Fn() -> broadcast::Receiver<StreamChunk> + Send + Sync>,
    input_tx: mpsc::Sender<WatcherInput>,
}
```

This is passed when creating the tool for an agent session.

---

## Dependencies

```toml
# In codelet/tools/Cargo.toml
tokio-tungstenite  # WebSocket client
futures            # Async stream handling  
serde/serde_json   # JSON serialization
url                # URL parsing
uuid               # Session IDs
```
