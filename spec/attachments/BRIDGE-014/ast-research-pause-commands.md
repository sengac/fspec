# AST Research: BRIDGE-014 Telegram Pause State Management Commands

## Overview

This document captures the AST analysis of existing code structures that need to be extended for pause state management commands in the Telegram bridge.

---

## TypeScript Structures (bridge/)

### 1. AVAILABLE_COMMANDS Array
**File**: `bridge/telegram-slash-commands.ts:70`
**Pattern**: `const AVAILABLE_COMMANDS = [$$$ITEMS]`

```typescript
const AVAILABLE_COMMANDS = [
  { command: '/help', description: 'Show this help message' },
  { command: '/status', description: 'Show agent session state' },
  { command: '/stop', description: 'Interrupt current agent operation' },
  { command: '/clear', description: 'Clear conversation history and reset session' },
];
```

**Modification Required**: Add new pause response commands:
- `/allowonce` - Allow blocked action once
- `/allow` - Alias for /allowonce
- `/allowsession` - Allow for entire session
- `/deny` - Deny the action

---

### 2. SlashCommandResult Interface
**File**: `bridge/telegram-slash-commands.ts:57`
**Pattern**: `interface SlashCommandResult { $$$FIELDS }`

```typescript
export interface SlashCommandResult {
  /** Whether the command was handled (true for slash commands, false for regular messages) */
  handled: boolean;
  /** The response sent to the user (if any) */
  response?: string;
  /** Action to perform (for commands that need session interaction) */
  action?: 'stop' | 'clear';
}
```

**Modification Required**: Extend action union type:
```typescript
action?: 'stop' | 'clear' | 'allow_once' | 'allow_session' | 'deny';
```

---

### 3. SlashCommandState Interface
**File**: `bridge/telegram-slash-commands.ts:46`
**Pattern**: `interface SlashCommandState { $$$FIELDS }`

```typescript
export interface SlashCommandState {
  bot: MinimalBot | null;
  chatId: string | null;
  currentSession: {
    ws: MinimalWebSocket | null;
    sessionId: string | null;
  };
  isRunning: boolean;
  agentState: AgentState;
}
```

**Modification Required**: Add pause state fields:
```typescript
isPaused: boolean;
pauseInfo?: {
  kind: 'triple';
  message: string;
  details?: string;
};
```

---

### 4. EndpointState Interface
**File**: `bridge/telegram-endpoint.ts:74`
**Pattern**: `interface EndpointState { $$$FIELDS }`

```typescript
export interface EndpointState {
  wss: WebSocketServer | null;
  bot: TelegramBotInstance | null;
  currentSession: {
    ws: WebSocket | null;
    sessionId: string | null;
  };
  chatId: string | null;
  toolNameMap: Map<string, string>;
  isRunning: boolean;
  // ... buffering state, thinking handler, whitelist, agentState
}
```

**Modification Required**: Add pause state fields:
```typescript
isPaused: boolean;
pauseInfo?: {
  kind: 'triple';
  message: string;
  toolName?: string;
  details?: string;
};
```

---

### 5. StreamChunkData Interface
**File**: `bridge/telegram-endpoint.ts:41`
**Pattern**: `interface StreamChunkData { $$$FIELDS }`

```typescript
export interface StreamChunkData {
  type: 'text' | 'thinking' | 'tool_call' | 'tool_result' | 'done' | 'error';
  text?: string;
  thinking?: string;
  name?: string;
  id?: string;
  tool_call_id?: string;
  content?: string;
  is_error?: boolean;
  error?: string;
}
```

**Modification Required**: Add pause_request type:
```typescript
type: 'text' | 'thinking' | 'tool_call' | 'tool_result' | 'done' | 'error' | 'pause_request';
// For pause_request type:
pause_kind?: 'triple';
pause_message?: string;
pause_details?: string;
pause_tool_name?: string;
```

---

## Rust Structures (codelet/tools/src/bridge_relay.rs)

### 6. Control Action Constants
**File**: `codelet/tools/src/bridge_relay.rs:31-32`

```rust
const ACTION_INTERRUPT: &str = "interrupt";
const ACTION_CLEAR: &str = "clear";
```

**Modification Required**: Add pause response action:
```rust
const ACTION_PAUSE_RESPONSE: &str = "pause_response";
```

---

### 7. Control Message Match Handler
**File**: `codelet/tools/src/bridge_relay.rs:335`

```rust
match action {
    ACTION_INTERRUPT | ACTION_CLEAR => {
        if let Some(handler) = control_handler {
            tracing::info!("Handling control action from bridge: {}", action);
            handler(action);
        } else {
            tracing::warn!("Received control action '{}' but no control handler is configured", action);
        }
        Ok(())
    }
    _ => {
        // Unknown action - log warning but don't crash
        tracing::warn!("Ignoring unknown control action: {}", action);
        Ok(())
    }
}
```

**Modification Required**: Add pause_response handling:
```rust
ACTION_PAUSE_RESPONSE => {
    if let Some(response) = inbound.response.as_deref() {
        // Call session_pause_triple with the response
        // Need to import from session_manager
        tracing::info!("Handling pause response from bridge: {}", response);
        // session_pause_triple(session_id.to_string(), response.to_string())
    }
    Ok(())
}
```

---

### 8. InboundMessage Struct
**File**: `codelet/tools/src/bridge_relay.rs:72-86`

```rust
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
}
```

**Modification Required**: Add response field for pause_response:
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub response: Option<String>,
```

---

## Control Flow Summary

### Pause Request Flow (Codelet → Telegram)
1. Codelet tool triggers PauseKind::Triple
2. Session status changes to 'paused'
3. Pause state broadcast via chunk with type 'pause_request'
4. Telegram endpoint receives chunk via WebSocket
5. Telegram endpoint sets `isPaused = true`, stores `pauseInfo`
6. Telegram displays: `⏸ Read: Sensitive file access (.ssh)`

### Pause Response Flow (Telegram → Codelet)
1. User sends `/allowonce`, `/allowsession`, or `/deny`
2. handleSlashCommand validates `state.isPaused === true`
3. Returns `{ handled: true, response: '...', action: 'allow_once' }`
4. Telegram endpoint sends control message via WebSocket:
   ```json
   {
     "type": "control",
     "action": "pause_response",
     "session_id": "...",
     "response": "allow_once"
   }
   ```
5. bridge_relay.rs receives control message
6. Calls `session_pause_triple(session_id, response)`
7. Session resumes with appropriate response

---

## Test Requirements

1. **telegram-slash-commands.ts tests**:
   - `/allowonce` command when paused → returns allow_once action
   - `/allow` alias → same as /allowonce
   - `/allowsession` when paused → returns allow_session action
   - `/deny` when paused → returns deny action
   - All pause commands when NOT paused → show error message
   - `/status` when paused → shows "⏸ Paused: Waiting for access decision"
   - `/help` → lists all new commands

2. **telegram-endpoint.ts tests**:
   - pause_request chunk sets isPaused and pauseInfo
   - pause response control message sent correctly
   - agentState transitions for pause states

3. **bridge_relay.rs tests**:
   - pause_response control action handling
   - InboundMessage with response field parsing
