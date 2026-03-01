# BRIDGE-015: Platform-Agnostic Relay Bridge Endpoint — Starting Context

## Counterpart Story

**fspec-mobile project → MOBILE-010** "Real Relay Bridge Client Integration" — the mobile/desktop client side that connects to the relay. Develop in parallel; this document covers the fspec-side endpoint.

---

## What Exists Today

### The Bridge Tool System (All DONE)

fspec already has a complete bridge tool system. The AI agent calls `Bridge(action: connect, url: "ws://...")` and fspec connects as a **WebSocket CLIENT** to bridge endpoints.

| Done Work Unit | What It Built | Key Files |
|---|---|---|
| BRIDGE-001 | Bridge Tool core (connect/disconnect/list) | `codelet/tools/src/bridge.rs` |
| BRIDGE-001 | WebSocket relay task | `codelet/tools/src/bridge_relay.rs` |
| BRIDGE-001 | Handler pattern for session context | `codelet/tools/src/bridge_handler.rs` |
| BRIDGE-002 | Telegram bridge endpoint | `bridge/telegram-endpoint.ts` |
| BRIDGE-007 | Image attachments from Telegram | `bridge_relay.rs` (InjectedInput) |
| BRIDGE-008 | Stream control channel (interrupt/clear) | `bridge_relay.rs` (ControlHandler) |
| BRIDGE-009 | Telegram user whitelist | `bridge/telegram-whitelist.ts` |
| BRIDGE-010 | Telegram slash commands | `bridge/telegram-slash-commands.ts` |
| BRIDGE-012 | Global chunk callback architecture | Decoupled from per-session |
| BRIDGE-013 | TUI persistent chunk handler | TUI bridge input display |
| BRIDGE-014 | Telegram pause state management | `pause_response` control action |

### The Telegram Endpoint Pattern

The Telegram endpoint (`bridge/telegram-endpoint.ts`) is the **reference implementation** for a bridge endpoint. Key characteristics:

1. **WebSocket SERVER** — codelet's BridgeTool connects TO it as a client
2. **Single session** — one fspec session at a time
3. **Translates protocol** — converts StreamChunks to Telegram MarkdownV2 messages
4. **Handles 2 inbound message types**: `input` (text from Telegram) and `control` (slash commands)
5. **Does NOT handle `command`** — Telegram bridge has no concept of fspec CLI command queries

### Existing Message Flow (fspec ↔ Telegram)

```
fspec BridgeTool                    Telegram Endpoint
    |                                      |
    |---[connects as WS client]----------->|
    |---{"type":"connected","session_id"}-->|   (handshake)
    |---{"type":"chunk","data":StreamChunk}>|   (stream output)
    |                                      |
    |<--{"type":"input","message":"..."}---|   (user input)
    |<--{"type":"control","action":"..."}--|   (interrupt/clear)
```

---

## What This Story Must Build

A **new bridge endpoint** (like Telegram, but for the relay) that:

1. Connects fspec to the platform-agnostic relay server
2. Speaks the relay protocol (not Telegram API)
3. Handles **all 5 message types** (not just 2 like Telegram)
4. Enables ANY client (mobile, desktop, web) to interact with fspec

### Architecture Decision: Endpoint Location

**Option A**: Standalone TypeScript file in `bridge/` (like Telegram)
- Pro: Follows existing pattern, easy to iterate
- Pro: Can run independently of fspec process
- Con: Needs separate process management

**Option B**: Built into codelet Rust (new relay mode in bridge_relay.rs)
- Pro: No separate process needed
- Con: More complex, tighter coupling

**Recommendation**: Option A — `bridge/relay-endpoint.ts` following the Telegram pattern. The relay endpoint is conceptually the same as Telegram: a standalone server that bridges fspec to an external system.

### The 5 Message Types (ALL Session-Scoped)

From the fspec-mobile architecture notes and corrected understanding:

| Type | Scope | Pattern | Direction | Purpose |
|------|-------|---------|-----------|---------|
| `input` | session | fire-and-forget | client→fspec | Inject AI prompt text + optional images |
| `sessionControl` | session | fire-and-forget | client→fspec | `interrupt`, `clear` |
| `command` | session | request/response | client→fspec | fspec CLI commands (separate from agent conversation) |
| `commandResponse` | session | response | fspec→client | fspec command results |
| `chunk` | session | stream | fspec→client | StreamChunk output from AI |

Plus: `auth`/`authSuccess`/`authError` handshake, `connected` session notification, `ping`/`pong` heartbeat.

**ALL message types are session-scoped.** Commands provide a separate channel within the session for fspec CLI operations (board, show-work-unit, etc.), distinct from the agent conversation (input/chunk). The mobile app is laid out per-session — board, work unit detail, session stream views all operate within a session context.

### What's NEW vs Telegram (the `command` channel)

The Telegram endpoint only handles `input` and `control`. This endpoint must ALSO handle `command` — a session-scoped request/response channel for fspec CLI commands:

**Inbound from relay**:
```json
{
  "type": "command",
  "session_id": "sess-1",
  "request_id": "uuid",
  "data": { "command": "board", "args": {} }
}
```

**The relay endpoint does NOT execute commands directly.** It is a **PURE PROTOCOL TRANSLATOR**. It translates the relay format to an InboundMessage and forwards it to the codelet's bridge WebSocket. The command execution flows through the existing StreamChunk event system in Rust:

1. Relay endpoint translates to `InboundMessage {type:"command", session_id, request_id, command, args_json}` → forwards to codelet bridge WS
2. `bridge_relay.rs` receives the `command` InboundMessage → emits `FspecCommandRequest` StreamChunk into the session
3. `GlobalSessionStreamManager` intercepts `FspecCommandRequest` → calls `fspecCallback` (same path as when LLM invokes Fspec tool)
4. `FspecCommandResult` StreamChunk flows back through the session's broadcast channel
5. `bridge_relay.rs` intercepts `FspecCommandResult` (does NOT forward as regular chunk) → formats as `commandResponse` OutboundMessage with matching `request_id`
6. Relay endpoint translates OutboundMessage to relay protocol format → sends to relay

**Outbound to relay**:
```json
{
  "type": "commandResponse",
  "session_id": "sess-1",
  "request_id": "uuid",
  "data": { "command": "board", "success": true, "result": { "columns": { ... }, "summary": "..." } }
}
```

### Available fspec Commands (from mobile architecture notes Part 4)

**Project queries** (always available within a session):
- `board` — Kanban board state
- `list-work-units`, `show-work-unit`, `query-work-units` — work unit operations
- `list-features`, `show-feature`, `get-scenarios` — feature files
- `show-coverage` — coverage status
- `show-foundation`, `show-foundation-event-storm` — project foundation
- `list-epics` — epic listing

**Mutations** (always available within a session):
- `update-work-unit-status` — move through workflow
- `add-rule`, `remove-rule`, `add-example`, `remove-example`, `add-question`, `answer-question` — example mapping
- `prioritize-work-unit` — reorder backlog

### How fspec Commands Are Executed

There is ONE unified command execution path through the StreamChunk event system. The relay endpoint does NOT call `fspecCallback` directly — it forwards command messages through the bridge WebSocket to Rust, which channels them through the same `FspecCommandRequest`/`FspecCommandResult` StreamChunk pipeline used when the LLM invokes the Fspec tool:

1. **LLM invokes Fspec tool** → Rust `session_manager.rs` emits `FspecCommandRequest` StreamChunk → TypeScript's `GlobalSessionStreamManager.handleFspecCommandRequest` calls `fspecCallback` → result returned via `sessionSendFspecResult` as `FspecCommandResult` StreamChunk
2. **Relay sends `command` message** → Relay endpoint translates and forwards to codelet bridge WS → Rust `bridge_relay.rs` emits `FspecCommandRequest` StreamChunk → same `GlobalSessionStreamManager` handling → `FspecCommandResult` flows back through broadcast channel → `bridge_relay.rs` intercepts and sends `commandResponse` OutboundMessage

Both paths converge on `GlobalSessionStreamManager` → `fspecCallback`. No child process (`exec`, `execFile`, `spawn`) is needed — `fspecCallback` invokes Commander.js programmatically.

### What Must Be Extended in Rust

**`InboundMessage` in `bridge_relay.rs`** — currently supports `input` and `control` types. Must add:
- `command` type with `request_id`, `command` (command name), and `args_json` fields

**`handle_inbound_message` in `bridge_relay.rs`** — currently handles `input` and `control`. Must add:
- `command` handling: emit `FspecCommandRequest` StreamChunk into the session

**Outbound message loop in `bridge_relay.rs`** — currently forwards all broadcast chunks as `chunk` OutboundMessages. Must add:
- Intercept `FspecCommandResult` chunks from broadcast channel
- Format as `commandResponse` OutboundMessage with matching `request_id` (NOT forwarded as regular `chunk`)

**`OutboundMessage` in `bridge.rs`** — currently has `msg_type`, `session_id`, `data`. Must add:
- Optional `request_id` field for `commandResponse` messages

**Key files:**
- `src/utils/fspec-callback.ts` — The `fspecCallback` function (in-process command executor)
- `src/tui/services/globalSessionStreamManager.ts` — `handleFspecCommandRequest` method (handles FspecCommandRequest from ANY session)
- `codelet/napi/src/types.rs` — `FspecCommandRequest` and `FspecCommandResult` StreamChunk types, `FspecRequest` and `FspecResult` structs
- `codelet/tools/src/bridge_relay.rs` — Message formats, relay loop, inbound handling (MUST BE EXTENDED)
- `codelet/tools/src/bridge.rs` — `OutboundMessage` struct (MUST BE EXTENDED with request_id)
- `codelet/tools/src/bridge_handler.rs` — `BridgeSessionContext` — broadcast_rx_factory, input_injector, control_handler
- `codelet/tools/src/fspec_handler.rs` — Rust-side handler that bridges to TypeScript

---

## Key Files to Study

| File | Why |
|------|-----|
| `bridge/telegram-endpoint.ts` | **Primary reference** — copy this pattern for the TypeScript relay endpoint |
| `bridge/telegram-slash-commands.ts` | Shows how Telegram handles /stop, /status → translate to relay commands |
| `bridge/telegram-whitelist.ts` | Auth pattern (relay uses channel_id + api_key instead) |
| `bridge/telegram-content-chunker.ts` | Chunk processing (relay doesn't need Telegram truncation) |
| `codelet/tools/src/bridge_relay.rs` | **MUST EXTEND** — Message formats, relay loop, inbound handling, outbound chunk interception |
| `codelet/tools/src/bridge.rs` | **MUST EXTEND** — `OutboundMessage` struct with optional `request_id` |
| `codelet/tools/src/bridge_handler.rs` | `BridgeSessionContext` — broadcast_rx_factory, input_injector, control_handler |
| `codelet/napi/src/types.rs` | `FspecCommandRequest`, `FspecCommandResult`, `FspecRequest`, `FspecResult` structs |
| `codelet/napi/src/session_manager.rs` | Reference for how LLM's Fspec tool emits `FspecCommandRequest` |
| `src/tui/services/globalSessionStreamManager.ts` | Reference for how `FspecCommandRequest` is handled globally |

### Protocol Format in bridge_relay.rs

**Outbound (fspec → endpoint)** — `OutboundMessage` (MUST BE EXTENDED):
```rust
pub struct OutboundMessage {
    pub msg_type: String,      // "chunk", "connected", or "commandResponse"
    pub session_id: String,
    pub data: serde_json::Value, // StreamChunk JSON or command result
    // NEW: optional request_id for commandResponse correlation
    pub request_id: Option<String>,
}
```

**Inbound (endpoint → fspec)** — `InboundMessage` (MUST BE EXTENDED):
```rust
pub struct InboundMessage {
    pub msg_type: String,      // "input", "control", or "command"
    pub session_id: String,
    pub message: String,       // for input
    pub images: Option<Vec<ImageData>>,  // for input with images
    pub action: Option<String>,          // for control: "interrupt", "clear"
    pub response: Option<String>,        // for pause_response
    // NEW: for command messages
    pub request_id: Option<String>,      // correlation ID
    pub command: Option<String>,         // fspec command name (e.g., "board")
    pub args_json: Option<String>,       // command arguments as JSON string
}
```

**Important**: fspec's `InboundMessage` uses flat fields (`message`, `action`), while the relay protocol wraps these in `data`. The endpoint must translate between formats.

---

## Message Translation Table

| From Relay | To fspec (InboundMessage) | Translation |
|---|---|---|
| `{type:"input", session_id, data:{message, images}}` | `{type:"input", session_id, message, images}` | Unwrap `data` |
| `{type:"sessionControl", session_id, data:{action}}` | `{type:"control", session_id, action}` | Rename type, unwrap `data` |
| `{type:"command", session_id, request_id, data:{command, args}}` | `{type:"command", session_id, request_id, command, args_json}` | Unwrap `data`, stringify args |

| From fspec (OutboundMessage) | To Relay | Translation |
|---|---|---|
| `{type:"chunk", session_id, data:StreamChunk}` | `{type:"chunk", session_id, data:StreamChunk}` | Pass through |
| `{type:"connected", session_id, data:{}}` | `{type:"connected", session_id, data:{}}` | Pass through |
| `{type:"commandResponse", session_id, request_id, data:{...}}` | `{type:"commandResponse", session_id, request_id, data:{...}}` | Pass through |

---

## Related Backlog Items

- **BRIDGE-004** "TUI Bridge Management" — TUI /bridge command, currently in backlog. The relay endpoint could be started/stopped from TUI.
- **CONFIG-006** "Connection Configuration File" — centralized config for relay URL, channel_id, api_key. The relay endpoint should read from this.

---

## Fake Relay for Testing

The fspec-mobile project has a fake relay server at `~/projects/fspec-mobile/tools/fake_relay_server.dart` that simulates the full protocol. Use it for testing the fspec-side endpoint during development.

Run it: `cd ~/projects/fspec-mobile && dart run tools/fake_relay_server.dart 8765`

It supports: auth handshake, board commands, work unit details, session streaming, input injection, and session control.
