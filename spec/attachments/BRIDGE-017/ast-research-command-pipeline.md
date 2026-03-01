# BRIDGE-017: AST Research — Command Handling Pipeline

## Research performed during specifying phase

### 1. Existing callback types in bridge_relay.rs

```
bridge_relay.rs:103: pub type InputInjector = Arc<dyn Fn(InjectedInput) + Send + Sync>;
bridge_relay.rs:107: pub type ControlHandler = Arc<dyn Fn(&str, Option<&str>) + Send + Sync>;
```

CommandEmitter will follow the same pattern: `pub type CommandEmitter = Arc<dyn Fn(String, String, String, String) + Send + Sync>;`

### 2. BridgeSessionContext in bridge_handler.rs

```
bridge_handler.rs:50: pub struct BridgeSessionContext {
    pub broadcast_rx_factory: BroadcastReceiverFactory,
    pub input_injector: InputInjector,
    pub control_handler: Option<ControlHandler>,
}
```

Needs extension with: `pub command_emitter: Option<CommandEmitter>`

### 3. Current function signatures to extend

- `spawn_relay_task(session_id, url, stream_rx, input_injector, control_handler)` — add `command_emitter`
- `relay_loop(session_id, url, stream_rx, input_injector, control_handler)` — add `command_emitter`
- `connect_and_relay(session_id, url, stream_rx, input_injector, control_handler)` — add `command_emitter` + create `pending_commands`
- `handle_inbound_message(text, session_id, input_injector, control_handler)` — add `command_emitter` + `pending_commands`

### 4. Message type constants already defined

```
const MSG_TYPE_INPUT: &str = "input";
const MSG_TYPE_CONTROL: &str = "control";
const MSG_TYPE_COMMAND: &str = "command";
```

MSG_TYPE_COMMAND is already defined from BRIDGE-016.

### 5. Current exports from lib.rs

```
pub use bridge_relay::{spawn_relay_task, InputInjector, InjectedInput, ImageData, ControlHandler};
```

CommandEmitter needs to be added here.

### 6. FspecCommandResult JSON format (from types.rs)

```json
{
  "type": "fspecCommandResult",
  "fspecResult": {
    "success": true,
    "data": "...",
    "error": null,
    "systemReminder": null,
    "toolCallId": "abc-123-uuid"
  }
}
```

### 7. FspecCommandRequest JSON format (from types.rs)

```json
{
  "type": "fspecCommandRequest",
  "fspecRequest": {
    "command": "board",
    "argsJson": "{}",
    "projectRoot": "/project",
    "toolCallId": "abc-123-uuid"
  }
}
```

Both need to be intercepted in the outbound loop.

### 8. set_bridge_session_context call site (session_manager.rs:5360-5365)

```rust
codelet_tools::set_bridge_session_context(
    session_id_for_bridge,
    broadcast_rx_factory,
    input_injector,
    Some(control_handler),
);
```

Needs to add `Some(command_emitter)` as 5th parameter.
