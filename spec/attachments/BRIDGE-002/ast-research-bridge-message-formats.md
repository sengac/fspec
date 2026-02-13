# AST Research: Bridge Message Formats

## Research Summary

Analysis of BRIDGE-001 implementation to understand the exact message formats that the Telegram endpoint must handle.

---

## Message Envelope Formats

### Outbound Messages (Codelet → Endpoint)

```typescript
interface OutboundMessage {
  type: "chunk";
  session_id: string;  // UUID format
  data: StreamChunkData;
}
```

### Inbound Messages (Endpoint → Codelet)

```typescript
interface InboundMessage {
  type: "input";
  session_id: string;  // UUID format
  message: string;     // User input text
}
```

---

## StreamChunk Data Formats

From `bridge.rs:456-507`, the `create_outbound_message` function shows exactly what data is sent:

### Text Chunk
```json
{
  "type": "text",
  "text": "Hello, I can help you with that."
}
```

### Thinking Chunk
```json
{
  "type": "thinking",
  "thinking": "Let me analyze this problem..."
}
```

### Tool Call Chunk
```json
{
  "type": "tool_call",
  "name": "Read",
  "id": "abc123"
}
```

### Tool Result Chunk
```json
{
  "type": "tool_result",
  "tool_call_id": "abc123",
  "content": "file contents here...",
  "is_error": false
}
```

### Done Chunk
```json
{
  "type": "done"
}
```

### Error Chunk
```json
{
  "type": "error",
  "error": "Connection failed"
}
```

---

## Data Structures (from types.rs)

### ToolCallInfo (lines 155-159)
```rust
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub input: String,  // JSON string of input (not sent to bridge)
}
```

### ToolResultInfo (lines 164-168)
```rust
pub struct ToolResultInfo {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}
```

---

## Key Implementation Details

1. **Session ID Validation**: Bridge verifies `session_id` matches on inbound messages (bridge.rs:422-427)

2. **Input Injection**: Uses `WatcherInput` with `RoleAuthority::Peer` (bridge.rs:434-444)

3. **Skipped Chunks**: Internal state changes (SessionStateChange, TokenUpdate, etc.) are NOT forwarded (bridge.rs:498-499)

4. **Correlation IDs**: Present in chunks but NOT forwarded to external endpoints (simplified JSON)

---

## Telegram Endpoint Implications

1. **Tool Correlation**: Must maintain `Map<id, name>` from tool_call chunks to display tool names in tool_result
2. **Message Format**: Must parse nested `data` object to determine chunk type
3. **Input Format**: Must send `{type: "input", session_id, message}` exactly as specified
4. **Error Handling**: Should handle any `is_error: true` tool results specially

---

## Files Analyzed

| File | Purpose |
|------|---------|
| `codelet/napi/src/bridge.rs` | Complete bridge implementation (741 lines) |
| `codelet/napi/src/types.rs` | StreamChunk enum and related types |

---

## AST Patterns Used

```bash
# Find all public enums
ast-grep --pattern "pub enum $NAME { $$$VARIANTS }" --lang rust

# Find specific struct definitions  
ast-grep --pattern "pub struct ToolCallInfo { $$$FIELDS }" --lang rust
```
