# AST Research: Bridge Message Types for BRIDGE-016

## InboundMessage struct (bridge_relay.rs:74)

```
pub struct InboundMessage {
    pub msg_type: String,        // #[serde(rename = "type")]
    pub session_id: String,
    pub message: String,         // #[serde(default)]
    pub images: Option<Vec<ImageData>>,  // #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,          // #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,        // #[serde(default, skip_serializing_if = "Option::is_none")]
}
```

**New fields to add:**
- `request_id: Option<String>` — correlation ID for command request/response
- `command: Option<String>` — fspec command name
- `args_json: Option<String>` — command arguments as JSON string

## OutboundMessage struct (bridge.rs:59)

```
pub struct OutboundMessage {
    pub msg_type: String,        // #[serde(rename = "type")]
    pub session_id: String,
    pub data: serde_json::Value,
}
```

**New field to add:**
- `request_id: Option<String>` — for commandResponse correlation

## OutboundMessage Construction Sites (ALL need `request_id: None`)

| File | Line | Context |
|------|------|---------|
| bridge_relay.rs | 256-259 | Outbound chunk loop |
| bridge_relay.rs | 418-422 | `send_connected_message` |
| bridge.rs | 567 | `test_outbound_message_serialize` (test) |
| bridge.rs | 657 | Buffer test (test) |
| bridge.rs | 663 | Buffer test (test) |
| bridge.rs | 728 | Buffer overflow test (test) |
| bridge_relay.rs | 568 | `test_outbound_message_serialize` in relay tests |
| bridge_integration_tests.rs | 337 | Integration test |
| bridge_integration_tests.rs | 442 | Integration test |
| bridge_integration_tests.rs | 449 | Integration test |
| bridge_integration_tests.rs | 528 | Integration test |
| bridge_integration_tests.rs | 584 | Integration test |

## Message Type Constants (bridge_relay.rs:27-28)

```
const MSG_TYPE_INPUT: &str = "input";
const MSG_TYPE_CONTROL: &str = "control";
```

**New constant to add:**
- `const MSG_TYPE_COMMAND: &str = "command";`

## Existing Test Pattern (follows serde roundtrip)

Tests use `serde_json::from_str` / `serde_json::to_string` for verification, checking field values after deserialization.
