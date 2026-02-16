# AST Research: Bridge Control Channel

## Research Purpose
Understanding the bridge relay architecture to implement control message handling.

## Files Analyzed

### codelet/tools/src/bridge_relay.rs

Key finding: The `handle_inbound_message` function (lines 269-307) handles incoming WebSocket messages:

```rust
async fn handle_inbound_message(
    text: &str,
    session_id: Uuid,
    input_injector: InputInjector,
) -> Result<(), String> {
    let inbound: InboundMessage = serde_json::from_str(text)
        .map_err(|e| format!("Failed to parse inbound message: {e}"))?;
    
    // Handle based on message type
    match inbound.msg_type.as_str() {
        "input" => {
            // Inject input into session
            input_injector(injected);
            Ok(())
        }
        _ => {
            tracing::warn!("Ignoring unknown message type: {}", inbound.msg_type);
            Ok(())
        }
    }
}
```

**Current limitation:** Only handles `type: "input"` messages. Unknown types are logged and ignored.

### InboundMessage struct (lines 64-73)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub session_id: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageData>>,
}
```

## Implementation Plan

### 1. Create ControlMessage struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlMessage {
    #[serde(rename = "type")]
    pub msg_type: String,  // "control"
    pub session_id: String,
    pub action: String,  // "interrupt" or "clear"
}
```

### 2. Add ControlCallback type

```rust
/// Callback for control actions (interrupt, clear)
pub type ControlCallback = Arc<dyn Fn(ControlAction) + Send + Sync>;

pub enum ControlAction {
    Interrupt,
    Clear,
}
```

### 3. Modify spawn_relay_task to accept control callback

The `spawn_relay_task` function needs to accept a control callback in addition to the input injector.

### 4. Update handle_inbound_message

Add a match arm for `"control"` message type that invokes the control callback.

## Integration Points

1. **codelet/napi/src/session.rs** - The Session struct needs to expose an interrupt mechanism
2. **Stream loop** - The agent loop needs to check the interrupt flag
3. **Session reset** - Clear action needs to reset conversation history

## Dependencies

- `is_interrupted: Arc<AtomicBool>` already exists in the agent loop
- Session has `clear_messages()` method for clearing history
