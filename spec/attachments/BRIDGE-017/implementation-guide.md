# BRIDGE-017: Command Handling Pipeline in bridge_relay.rs

## Objective

When a bridge endpoint sends an InboundMessage with `type: "command"`, `bridge_relay.rs` must:
1. Emit a `FspecCommandRequest` StreamChunk into the session
2. Intercept `FspecCommandResult` StreamChunks in the outbound broadcast loop (NOT forward them as regular chunks)
3. Send a `commandResponse` OutboundMessage with the matching `request_id` back through the bridge WebSocket

This reuses the exact same `FspecCommandRequest → GlobalSessionStreamManager → fspecCallback → FspecCommandResult` pipeline that the LLM's Fspec tool uses.

---

## Architecture Overview

```
Bridge Endpoint (TS)
    │
    ▼ InboundMessage {type:"command", request_id:"R1", command:"board", args_json:"{}"}
bridge_relay.rs
    │
    ├─► Stores R1 in pending_commands HashMap
    │
    ├─► Calls command_emitter(FspecCommandRequest) callback
    │       │
    │       ▼ session_manager.rs (the callback)
    │       │   emits StreamChunk::FspecCommandRequest into session
    │       │   │
    │       │   ▼ GlobalSessionStreamManager (TypeScript)
    │       │       calls fspecCallback()
    │       │       calls sessionSendFspecResult()
    │       │       │
    │       │       ▼ session broadcasts FspecCommandResult StreamChunk
    │       │
    ◄───────┘ outbound loop receives FspecCommandResult from broadcast
    │
    ├─► Looks up R1 from pending_commands by tool_call_id
    │
    ├─► Formats commandResponse OutboundMessage {request_id:"R1", ...}
    │
    ▼ Sends to bridge WS (relay endpoint receives it)
Bridge Endpoint (TS)
```

---

## Key Insight: The tool_call_id Correlation

The existing `FspecCommandRequest`/`FspecCommandResult` pipeline uses `tool_call_id` for correlation. In the LLM path, session_manager.rs generates a UUID for `tool_call_id`. For bridge commands, we do the same — generate a UUID tool_call_id and store a mapping: `tool_call_id → request_id`. When `FspecCommandResult` arrives with that `tool_call_id`, we look up the original `request_id`.

---

## Files to Modify

### 1. `codelet/tools/src/bridge_relay.rs`

#### New type: CommandEmitter callback

```rust
/// BRIDGE-017: Callback to emit FspecCommandRequest into the session
/// Takes (command, args_json, project_root, tool_call_id) and emits the StreamChunk.
/// Returns immediately — the result comes back asynchronously via the broadcast channel.
pub type CommandEmitter = Arc<dyn Fn(String, String, String, String) + Send + Sync>;
```

#### Extend `spawn_relay_task` signature

Add `command_emitter: Option<CommandEmitter>` parameter:

```rust
pub async fn spawn_relay_task(
    session_id: Uuid,
    url: String,
    stream_rx: broadcast::Receiver<serde_json::Value>,
    input_injector: InputInjector,
    control_handler: Option<ControlHandler>,
    command_emitter: Option<CommandEmitter>,  // NEW
) -> Result<tokio::task::JoinHandle<()>, ToolError> {
```

Thread it through `relay_loop` → `connect_and_relay`.

#### Extend `handle_inbound_message`

Add `command_emitter: Option<CommandEmitter>` parameter. Handle the new message type:

```rust
MSG_TYPE_COMMAND => {
    let request_id = inbound.request_id.unwrap_or_default();
    let command = inbound.command.unwrap_or_default();
    let args_json = inbound.args_json.unwrap_or_else(|| "{}".to_string());
    
    if let Some(emitter) = command_emitter {
        let tool_call_id = Uuid::new_v4().to_string();
        // Store request_id → tool_call_id mapping (done by caller via returned value)
        tracing::info!("Bridge command: {} (request_id: {}, tool_call_id: {})", 
            command, request_id, tool_call_id);
        
        // The project_root needs to come from somewhere — use CWD
        let project_root = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());
        
        emitter(command, args_json, project_root, tool_call_id);
        Ok(())
    } else {
        tracing::warn!("Received command but no command emitter configured");
        Ok(())
    }
}
```

**IMPORTANT**: `handle_inbound_message` needs to return the `(request_id, tool_call_id)` pair so the caller can store it in the pending commands map. Refactor the return type or pass a pending_commands reference.

Better approach: Pass a `pending_commands: Arc<Mutex<HashMap<String, String>>>` to `handle_inbound_message` and have it insert the mapping directly:

```rust
pub async fn handle_inbound_message(
    text: &str,
    session_id: Uuid,
    input_injector: InputInjector,
    control_handler: Option<ControlHandler>,
    command_emitter: Option<CommandEmitter>,       // NEW
    pending_commands: Option<Arc<Mutex<HashMap<String, String>>>>,  // NEW: tool_call_id → request_id
) -> Result<(), String> {
```

#### Modify the outbound broadcast loop in `connect_and_relay`

Currently (lines 252-276), ALL chunks are forwarded as "chunk" OutboundMessages. Must inspect the JSON for `fspecCommandResult` type and intercept:

```rust
chunk_result = stream_rx.recv() => {
    match chunk_result {
        Ok(chunk_json) => {
            // BRIDGE-017: Check if this is a FspecCommandResult
            let chunk_type = chunk_json.get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            
            if chunk_type == "fspecCommandResult" {
                // Intercept — do NOT forward as regular chunk
                if let Some(ref pending) = pending_commands_clone {
                    let fspec_result = chunk_json.get("fspecResult");
                    let tool_call_id = fspec_result
                        .and_then(|r| r.get("toolCallId"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    
                    let request_id = {
                        let mut map = pending.lock().unwrap();
                        map.remove(tool_call_id)
                    };
                    
                    if let Some(req_id) = request_id {
                        let success = fspec_result
                            .and_then(|r| r.get("success"))
                            .and_then(|s| s.as_bool())
                            .unwrap_or(false);
                        let data = fspec_result
                            .and_then(|r| r.get("data"))
                            .and_then(|d| d.as_str())
                            .unwrap_or("");
                        let error = fspec_result
                            .and_then(|r| r.get("error"))
                            .and_then(|e| e.as_str());
                        
                        // Parse the data as JSON if possible, else use string
                        let result_value = serde_json::from_str(data)
                            .unwrap_or_else(|_| serde_json::Value::String(data.to_string()));
                        
                        let outbound = OutboundMessage {
                            msg_type: "commandResponse".to_string(),
                            session_id: session_id.to_string(),
                            data: serde_json::json!({
                                "command": "", // TODO: we don't have the original command name here
                                "success": success,
                                "result": result_value,
                                "error": error,
                            }),
                            request_id: Some(req_id),
                        };
                        
                        // Send commandResponse
                        let msg_json = serde_json::to_string(&outbound)
                            .map_err(|e| format!("serialize error: {e}"))?;
                        ws_write.send(Message::Text(msg_json.into())).await
                            .map_err(|e| format!("Send failed: {e}"))?;
                    }
                }
                continue; // Do NOT forward as chunk
            }
            
            // Regular chunk — forward as before
            let outbound = OutboundMessage {
                msg_type: "chunk".to_string(),
                session_id: session_id.to_string(),
                data: chunk_json,
                request_id: None,
            };
            // ... existing send logic ...
        }
    }
}
```

**Note about the command name**: To include the original command name in the commandResponse, store `(request_id, command_name)` in the pending map instead of just `request_id`. Use `HashMap<String, (String, String)>` where key is tool_call_id and value is `(request_id, command_name)`.

### 2. `codelet/tools/src/bridge_handler.rs`

Extend `BridgeSessionContext` to include the command emitter:

```rust
pub struct BridgeSessionContext {
    pub broadcast_rx_factory: BroadcastReceiverFactory,
    pub input_injector: InputInjector,
    pub control_handler: Option<ControlHandler>,
    pub command_emitter: Option<CommandEmitter>,  // NEW
}
```

Update `set_bridge_session_context` signature accordingly.

### 3. `codelet/napi/src/session_manager.rs`

In the bridge setup section (~line 5230-5385), create the `command_emitter` callback:

```rust
// BRIDGE-017: Create command emitter for bridge relay
let session_for_command = session.clone();
let command_emitter: codelet_tools::CommandEmitter = Arc::new(move |command, args_json, project_root, tool_call_id| {
    // Check global chunk callback is registered
    if GLOBAL_CHUNK_CALLBACK.get().is_none() {
        tracing::warn!("Cannot emit FspecCommandRequest - no global chunk callback");
        return;
    }
    
    let fspec_request = crate::types::FspecRequest {
        command,
        args_json,
        project_root,
        tool_call_id,
    };
    
    session_for_command.handle_output(StreamChunk::fspec_command_request(fspec_request));
});
```

Pass it to `set_bridge_session_context`:

```rust
codelet_tools::set_bridge_session_context(
    session_id_for_bridge,
    broadcast_rx_factory,
    input_injector,
    Some(control_handler),
    Some(command_emitter),  // NEW
);
```

Also update `handle_bridge_action` in `bridge_handler.rs` where `spawn_relay_task` is called — pass the `command_emitter` from the context.

### 4. `codelet/tools/src/lib.rs`

Export the new `CommandEmitter` type:

```rust
pub use bridge_relay::{CommandEmitter, ...};
```

---

## Key Design Decisions

1. **Pending commands map**: `Arc<Mutex<HashMap<String, (String, String)>>>` where key=tool_call_id, value=(request_id, command_name). Created per-connection in `connect_and_relay`.

2. **FspecCommandResult interception**: Done by inspecting `chunk_json["type"]` as a string. The JSON format from `StreamChunk::to_json_value()` produces `"type": "fspecCommandResult"` (camelCase).

3. **Timeout**: NOT handled by bridge_relay.rs. The existing FspecCommandRequest→FspecCommandResult pipeline handles timeouts (session_manager.rs waits with `wait_for_fspec_response()` which blocks the agent loop). For bridge commands, the command_emitter is fire-and-forget — it emits the request and returns immediately. The result flows back asynchronously through the broadcast channel.

4. **project_root**: Obtained from `std::env::current_dir()` in the command handler. This is correct because the fspec process's CWD is always the project root.

---

## Tests to Write

### Unit tests in `bridge_relay.rs`:

```rust
#[test]
fn test_inbound_command_message_parse() {
    let json = r#"{"type":"command","session_id":"s1","message":"","request_id":"r1","command":"board","args_json":"{}"}"#;
    let msg: InboundMessage = serde_json::from_str(json).unwrap();
    assert_eq!(msg.msg_type, "command");
    assert_eq!(msg.request_id, Some("r1".to_string()));
    assert_eq!(msg.command, Some("board".to_string()));
}

#[tokio::test]
async fn test_handle_command_emits_fspec_request() {
    use std::sync::atomic::{AtomicBool, Ordering};
    
    let session_id = Uuid::new_v4();
    let emitter_called = Arc::new(AtomicBool::new(false));
    let emitter_called_clone = emitter_called.clone();
    
    let command_emitter: CommandEmitter = Arc::new(move |cmd, _args, _root, _tcid| {
        assert_eq!(cmd, "board");
        emitter_called_clone.store(true, Ordering::SeqCst);
    });
    
    let input_injector: InputInjector = Arc::new(|_| {});
    let pending = Arc::new(Mutex::new(HashMap::new()));
    
    let json = format!(
        r#"{{"type":"command","session_id":"{}","message":"","request_id":"r1","command":"board","args_json":"{{}}"}}"#,
        session_id
    );
    
    let result = handle_inbound_message(
        &json, session_id, input_injector, None,
        Some(command_emitter), Some(pending)
    ).await;
    
    assert!(result.is_ok());
    assert!(emitter_called.load(Ordering::SeqCst));
}
```

### Outbound interception test (harder — needs broadcast channel):

Test that when a FspecCommandResult JSON with a known tool_call_id appears on the broadcast channel, it gets intercepted and turned into a commandResponse rather than being forwarded as a chunk.

---

## Verification

1. `cargo test -p codelet-tools` — all tests pass
2. `cargo test -p codelet-napi` — all tests pass  
3. `cargo build` — compiles cleanly
4. Manual test with fake relay: send a command message, verify commandResponse comes back

---

## Estimate: 5 points

Complex changes across 4 files in 2 crates. The outbound interception logic in the broadcast loop is the hardest part. Needs careful correlation tracking and JSON inspection.
