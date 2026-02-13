# AST Research: Bridge Rig Tool Architecture

## Research Summary

This research documents how to implement the **Bridge Tool** as a proper Rig Tool (like WebSearch, Read, Bash) that the AI agent calls to manage WebSocket connections to external endpoints.

## Key Insight: THIS IS A RIG TOOL, NOT NAPI BINDINGS!

The Bridge is a **tool the AI agent calls**, not a background service controlled via TUI commands.

### Comparison with WebSearch Tool

| Aspect | WebSearch | Bridge |
|--------|-----------|--------|
| Tool Location | `codelet/tools/src/web_search.rs` | `codelet/tools/src/bridge.rs` |
| Actions | search, open_page, find_in_page, capture_screenshot | connect, disconnect, list |
| Global State | `static BROWSER: Mutex<Option<Arc<ChromeBrowser>>>` | `static BRIDGES: Mutex<HashMap<Uuid, BridgeManager>>` |
| Agent Calls | `WebSearch(action: 'search', query: '...')` | `Bridge(action: 'connect', url: '...')` |

## Tool Definition

```rust
// codelet/tools/src/bridge.rs

use rig::{completion::ToolDefinition, tool::Tool};

#[derive(Clone, Debug)]
pub struct BridgeTool;

#[derive(Debug, Deserialize, Serialize)]
pub struct BridgeRequest {
    pub action: BridgeAction,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeAction {
    Connect { url: String },
    Disconnect { url: String },
    List,
}

impl Tool for BridgeTool {
    const NAME: &'static str = "Bridge";
    
    type Error = ToolError;
    type Args = BridgeRequest;
    type Output = BridgeResult;
    
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "Bridge".to_string(),
            description: "Connect to external WebSocket endpoints to relay session output and receive remote input. Use action 'connect' to establish connection, 'disconnect' to close, 'list' to show active bridges.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "oneOf": [
                            {
                                "type": "object",
                                "properties": {
                                    "type": { "const": "connect" },
                                    "url": { "type": "string", "description": "WebSocket URL (e.g., ws://localhost:8080)" }
                                },
                                "required": ["type", "url"]
                            },
                            {
                                "type": "object", 
                                "properties": {
                                    "type": { "const": "disconnect" },
                                    "url": { "type": "string", "description": "WebSocket URL to disconnect" }
                                },
                                "required": ["type", "url"]
                            },
                            {
                                "type": "object",
                                "properties": {
                                    "type": { "const": "list" }
                                },
                                "required": ["type"]
                            }
                        ]
                    }
                },
                "required": ["action"]
            }),
        }
    }
    
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Implementation here - needs session_id from context
    }
}
```

## Session Context Injection

The Bridge tool needs access to the session's broadcast channel and input channel. This requires injecting session context at runtime, similar to how FspecTool needs project_root.

### Pattern: FacadeWrapper with Session Context

```rust
// In codelet/tools/src/facade/bridge_wrapper.rs

pub struct BridgeToolFacadeWrapper {
    inner: BridgeTool,
    session_id: Uuid,
    stream_rx_factory: Arc<dyn Fn() -> broadcast::Receiver<StreamChunk> + Send + Sync>,
    input_tx: mpsc::Sender<WatcherInput>,
}

impl BridgeToolFacadeWrapper {
    pub fn new(
        session_id: Uuid,
        stream_rx_factory: Arc<dyn Fn() -> broadcast::Receiver<StreamChunk> + Send + Sync>,
        input_tx: mpsc::Sender<WatcherInput>,
    ) -> Self {
        Self {
            inner: BridgeTool,
            session_id,
            stream_rx_factory,
            input_tx,
        }
    }
}
```

## Existing Infrastructure (from WATCH-003, WATCH-006)

### Broadcast Channel

Location: `codelet/napi/src/session_manager.rs:836`

```rust
/// Broadcast channel for watcher sessions to observe stream output (WATCH-003)
watcher_broadcast: broadcast::Sender<StreamChunk>,
```

### Subscribe Method

Location: `codelet/napi/src/session_manager.rs:1120-1122`

```rust
/// Subscribe to the output stream for watcher sessions (WATCH-003)
pub fn subscribe_to_stream(&self) -> broadcast::Receiver<StreamChunk> {
    self.watcher_broadcast.subscribe()
}
```

### Input Injection

Location: `codelet/napi/src/session_manager.rs:841-844`

```rust
/// Channel for receiving watcher input messages (WATCH-006)
watcher_input_tx: mpsc::Sender<WatcherInput>,
watcher_input_rx: Mutex<mpsc::Receiver<WatcherInput>>,
```

## Global Connection State

Like WebSearch manages a global browser instance, Bridge manages global connection state:

```rust
// Global bridge connections per session
lazy_static::lazy_static! {
    static ref BRIDGES: Mutex<HashMap<Uuid, Arc<BridgeManager>>> = Mutex::new(HashMap::new());
}

pub struct BridgeManager {
    session_id: Uuid,
    connections: HashMap<String, BridgeConnection>,
}

pub struct BridgeConnection {
    url: Url,
    state: BridgeConnectionState,
    outbound_buffer: VecDeque<OutboundMessage>,
    buffer_size_bytes: u64,
    // WebSocket task handles
}
```

## Message Formats

### Outbound (Session → Endpoint)

```json
{
    "type": "chunk",
    "session_id": "uuid-here",
    "data": {
        "type": "text",
        "text": "Hello, I can help with that."
    }
}
```

### Inbound (Endpoint → Session)

```json
{
    "type": "input",
    "session_id": "uuid-here", 
    "message": "build the app"
}
```

## StreamChunk Types to Relay

All StreamChunk variants should be relayed - the endpoint decides what to display:

- `Text` - AI text responses
- `Thinking` - Extended thinking/reasoning  
- `ToolCall` - Tool invocations
- `ToolResult` - Tool results
- `ToolProgress` - Streaming tool output
- `Done` - Stream completed
- `Error` - Error occurred
- `UserInput` - User input messages

## Integration Points

1. **Tool Registration**: Add BridgeTool to the tool set in agent session creation
2. **FacadeWrapper**: Create BridgeToolFacadeWrapper that injects session context
3. **Session Cleanup**: Call `bridge_shutdown_all(session_id)` when session ends
4. **lib.rs Export**: Add `pub mod bridge;` to `codelet/tools/src/lib.rs`

## Example Agent Usage

```
Agent: I'll connect to the Telegram bridge endpoint so you can receive my responses on your phone.

<tool_use>
<name>Bridge</name>
<parameters>
{"action": {"type": "connect", "url": "ws://localhost:8080/telegram"}}
</parameters>
</tool_use>

Tool Result: Connected to ws://localhost:8080/telegram. All session output will be relayed to this endpoint.

Agent: Great! The bridge is connected. I'll now proceed with your request...
```

## Files to Create/Modify

### Create:
- `codelet/tools/src/bridge.rs` - Main tool implementation
- `codelet/tools/src/facade/bridge_wrapper.rs` - FacadeWrapper for session context

### Modify:
- `codelet/tools/src/lib.rs` - Add `pub mod bridge;`
- `codelet/tools/src/facade/mod.rs` - Add bridge wrapper export
- Agent session code - Register BridgeTool with session context
