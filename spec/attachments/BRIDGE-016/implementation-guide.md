# BRIDGE-016: Extend Rust Bridge Message Types for Command Support

## Objective

Add fields to `InboundMessage` and `OutboundMessage` so that the existing bridge WebSocket protocol can carry `command` and `commandResponse` messages. **No behavioral changes** — this is a pure type extension with serialization/deserialization support.

---

## Files to Modify

### 1. `codelet/tools/src/bridge_relay.rs` — InboundMessage

**Current struct** (lines 73-90):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub session_id: String,
    #[serde(default)]
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageData>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
}
```

**Add these three fields** (after the `response` field):

```rust
    /// BRIDGE-016: Correlation ID for command request/response pattern
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// BRIDGE-016: fspec command name (e.g., "board", "show-work-unit")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// BRIDGE-016: Command arguments as JSON string (e.g., '{"_":["AUTH-001"]}')
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args_json: Option<String>,
```

**Also add a constant** alongside the existing ones (lines 27-28):

```rust
const MSG_TYPE_COMMAND: &str = "command";
```

### 2. `codelet/tools/src/bridge.rs` — OutboundMessage

**Current struct** (lines 58-64):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub session_id: String,
    pub data: serde_json::Value,
}
```

**Add this field** (after `data`):

```rust
    /// BRIDGE-016: Optional request_id for commandResponse correlation
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
```

### 3. Fix ALL existing OutboundMessage construction sites

Adding `request_id: Option<String>` means every place that constructs `OutboundMessage` must include the field. Search the codebase for `OutboundMessage {` — these are the sites:

| File | Line(s) | Context | Fix |
|------|---------|---------|-----|
| `bridge_relay.rs` | ~256-259 | Outbound chunk loop | Add `request_id: None` |
| `bridge_relay.rs` | ~418-422 | `send_connected_message` | Add `request_id: None` |
| `bridge.rs` tests | ~568-571 | `test_outbound_message_serialize` | Add `request_id: None` |
| `bridge.rs` tests | ~654-660 (approx) | Buffer tests | Add `request_id: None` |

Each of these just needs `request_id: None` added. No behavioral change.

---

## Tests to Write

### In `bridge_relay.rs` tests:

```rust
#[test]
fn test_inbound_message_parse_command() {
    let json = r#"{
        "type": "command",
        "session_id": "test-id",
        "message": "",
        "request_id": "req-001",
        "command": "board",
        "args_json": "{}"
    }"#;
    let msg: InboundMessage = serde_json::from_str(json).unwrap();
    assert_eq!(msg.msg_type, "command");
    assert_eq!(msg.session_id, "test-id");
    assert_eq!(msg.request_id, Some("req-001".to_string()));
    assert_eq!(msg.command, Some("board".to_string()));
    assert_eq!(msg.args_json, Some("{}".to_string()));
}

#[test]
fn test_inbound_message_backward_compatibility_no_command_fields() {
    // Existing input messages should still parse without the new fields
    let json = r#"{"type": "input", "session_id": "test-123", "message": "Hello"}"#;
    let msg: InboundMessage = serde_json::from_str(json).unwrap();
    assert_eq!(msg.msg_type, "input");
    assert!(msg.request_id.is_none());
    assert!(msg.command.is_none());
    assert!(msg.args_json.is_none());
}
```

### In `bridge.rs` tests:

```rust
#[test]
fn test_outbound_message_serialize_with_request_id() {
    let msg = OutboundMessage {
        msg_type: "commandResponse".to_string(),
        session_id: "test-id".to_string(),
        data: serde_json::json!({
            "command": "board",
            "success": true,
            "result": {"columns": {}}
        }),
        request_id: Some("req-001".to_string()),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"request_id\":\"req-001\""));
    assert!(json.contains("\"type\":\"commandResponse\""));
}

#[test]
fn test_outbound_message_serialize_without_request_id() {
    let msg = OutboundMessage {
        msg_type: "chunk".to_string(),
        session_id: "test-id".to_string(),
        data: serde_json::json!({"type": "text", "text": "hello"}),
        request_id: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    // request_id should NOT appear in JSON when None (skip_serializing_if)
    assert!(!json.contains("request_id"));
}
```

---

## Verification

After implementation:
1. `cargo test -p codelet-tools` — all existing tests still pass
2. New tests pass
3. `cargo build` — no compile errors (all OutboundMessage construction sites updated)
4. JSON serialization of OutboundMessage with `request_id: None` does NOT include the field (verified by test)

---

## Estimate: 2 points

Trivial type extension. The only risk is missing a construction site, but `cargo build` catches that immediately.
