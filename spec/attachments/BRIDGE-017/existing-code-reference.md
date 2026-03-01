# BRIDGE-017: Existing Code Reference — FspecCommandRequest/Result Pipeline

This attachment provides the exact code snippets from the existing codebase that BRIDGE-017 must integrate with. Read this to understand the established patterns.

---

## 1. How session_manager.rs emits FspecCommandRequest (LLM path)

From `codelet/napi/src/session_manager.rs` ~line 5196-5221:

```rust
// Generate a unique tool call ID for correlation
let tool_call_id = uuid::Uuid::new_v4().to_string();

// Emit FspecCommandRequest chunk for TypeScript to process
let fspec_request = crate::types::FspecRequest {
    command: request.command.clone(),
    args_json: request.args_json.clone(),
    project_root: request.project_root.clone(),
    tool_call_id: tool_call_id.clone(),
};

session_for_fspec.handle_output(StreamChunk::fspec_command_request(fspec_request));

// Block until TypeScript executes and calls sessionSendFspecResult
let fspec_result = session_for_fspec.wait_for_fspec_response();

// Emit FspecCommandResult chunk for UI display
session_for_fspec.handle_output(StreamChunk::fspec_command_result(fspec_result.clone()));
```

**KEY DIFFERENCE for bridge**: The LLM path BLOCKS on `wait_for_fspec_response()`. For bridge commands, we DON'T block — we emit the request and let the result come back asynchronously through the broadcast channel.

However, note that `handle_output` broadcasts to ALL subscribers including the bridge relay's broadcast receiver. So the bridge relay will see BOTH `FspecCommandRequest` AND `FspecCommandResult` chunks on its broadcast channel. It should IGNORE `FspecCommandRequest` chunks (they're for TypeScript) and INTERCEPT `FspecCommandResult` chunks.

---

## 2. StreamChunk JSON format (what the broadcast channel carries)

From `codelet/napi/src/types.rs` ~line 703-720:

```rust
Self::FspecCommandRequest { fspec_request } => json!({
    "type": "fspecCommandRequest",
    "fspecRequest": {
        "command": fspec_request.command,
        "argsJson": fspec_request.args_json,
        "projectRoot": fspec_request.project_root,
        "toolCallId": fspec_request.tool_call_id,
    },
}),
Self::FspecCommandResult { fspec_result } => json!({
    "type": "fspecCommandResult",
    "fspecResult": {
        "success": fspec_result.success,
        "data": fspec_result.data,
        "error": fspec_result.error,
        "systemReminder": fspec_result.system_reminder,
        "toolCallId": fspec_result.tool_call_id,
    },
}),
```

So on the broadcast channel, the JSON will look like:
```json
{
  "type": "fspecCommandResult",
  "fspecResult": {
    "success": true,
    "data": "{\"columns\":{...},\"summary\":\"...\"}",
    "error": null,
    "systemReminder": null,
    "toolCallId": "abc-123-uuid"
  }
}
```

**Matching logic**: `chunk_json["type"] == "fspecCommandResult"` and `chunk_json["fspecResult"]["toolCallId"]` matches a pending tool_call_id.

---

## 3. FspecRequest and FspecResult struct definitions

From `codelet/napi/src/types.rs` ~line 797-828:

```rust
#[napi(object)]
pub struct FspecRequest {
    pub command: String,
    #[napi(js_name = "argsJson")]
    pub args_json: String,
    #[napi(js_name = "projectRoot")]
    pub project_root: String,
    #[napi(js_name = "toolCallId")]
    pub tool_call_id: String,
}

#[napi(object)]
pub struct FspecResult {
    pub success: bool,
    pub data: String,
    pub error: Option<String>,
    #[napi(js_name = "systemReminder")]
    pub system_reminder: Option<String>,
    #[napi(js_name = "toolCallId")]
    pub tool_call_id: String,
}
```

---

## 4. How GlobalSessionStreamManager handles FspecCommandRequest (TypeScript)

From `src/tui/services/globalSessionStreamManager.ts` ~line 311-313:

```typescript
if (chunk.type === 'FspecCommandRequest' && chunk.fspecRequest) {
    void this.handleFspecCommandRequest(sessionId, chunk);
    return; // Not forwarded to session handlers
}
```

And ~line 335-379 (the handler):

```typescript
private async handleFspecCommandRequest(sessionId, chunk) {
    const { command, argsJson, projectRoot, toolCallId } = request;
    
    const resultJson = await this.fspecCallback(command, argsJson, projectRoot);
    const parsed = JSON.parse(resultJson);
    
    napi.sessionSendFspecResult(sessionId, {
        success: parsed.success ?? true,
        data: parsed.data ?? resultJson,
        error: parsed.error ?? undefined,
        systemReminder,
        toolCallId,
    });
}
```

This is the TypeScript side — it receives the FspecCommandRequest, calls fspecCallback, and sends the result back via `sessionSendFspecResult`. This triggers `session.send_fspec_result()` in Rust, which:
1. Unblocks `wait_for_fspec_response()` (for LLM path)  
2. Emits `StreamChunk::FspecCommandResult` via `handle_output` (which broadcasts to bridge relay)

**For bridge commands**: Step 1 is irrelevant (bridge doesn't block). Step 2 is what the bridge relay intercepts.

---

## 5. The broadcast adapter in session_manager.rs

From ~line 5237-5269 — this converts `StreamChunk` to `serde_json::Value`:

```rust
let broadcast_rx_factory: BroadcastReceiverFactory = Arc::new(move || {
    let mut stream_rx = watcher_broadcast_sender.subscribe();
    let (json_tx, json_rx) = tokio::sync::broadcast::channel::<serde_json::Value>(256);
    let json_tx_clone = json_tx.clone();
    tokio::spawn(async move {
        loop {
            match stream_rx.recv().await {
                Ok(chunk) => {
                    let json_value = chunk.to_json_value();
                    let _ = json_tx_clone.send(json_value);
                }
                // ...
            }
        }
    });
    json_rx
});
```

So the bridge relay's `stream_rx` receives `serde_json::Value` (not `StreamChunk`). The JSON inspection approach is correct.

---

## 6. Current BridgeSessionContext

From `codelet/tools/src/bridge_handler.rs` ~line 50-57:

```rust
pub struct BridgeSessionContext {
    pub broadcast_rx_factory: BroadcastReceiverFactory,
    pub input_injector: InputInjector,
    pub control_handler: Option<ControlHandler>,
}
```

And the setup call in session_manager.rs ~line 5360-5365:

```rust
codelet_tools::set_bridge_session_context(
    session_id_for_bridge,
    broadcast_rx_factory,
    input_injector,
    Some(control_handler),
);
```

Both need to be extended with `command_emitter`.

---

## 7. Where spawn_relay_task is called

From `codelet/tools/src/bridge_handler.rs` ~line 201:

```rust
match spawn_relay_task(session_id, url.clone(), broadcast_rx, input_injector, control_handler).await {
```

Must add `command_emitter` parameter here too, pulling it from the context.
