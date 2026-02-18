# BRIDGE-014: Telegram Pause State Management Commands - Research Notes

## Related Work Units

### BLOCK-007: Integrate Blocklist Prompt Action with Tool Pause System
- **Status**: Done
- **Key Implementation**: Wired blocklist 'prompt' action to PauseKind::Triple
- **Responses**: AllowOnce, AllowSession, Deny
- **TUI Implementation**: InputTransition shows inline triple-choice UI with ←/→ navigation

### BRIDGE-010: Telegram Slash Commands for Agent Control  
- **Status**: Done
- **Commands**: /help, /status, /stop, /clear
- **Pattern**: Commands intercepted before forwarding to agent, handled via SlashCommandHandler

### BRIDGE-008: Telegram Stream Control Channel
- **Status**: Done
- **Architecture**: Control messages have type 'control' with 'action' field
- **Actions**: 'interrupt' (for /stop), 'clear' (for /clear)
- **Processing**: BridgeManager WebSocket message handler

---

## Existing Code Analysis

### Telegram Slash Commands (`bridge/telegram-slash-commands.ts`)

```typescript
// Current commands available
const AVAILABLE_COMMANDS = [
  { command: '/help', description: 'Show this help message' },
  { command: '/status', description: 'Show agent session state' },
  { command: '/stop', description: 'Interrupt current agent operation' },
  { command: '/clear', description: 'Clear conversation history and reset session' },
];

// handleSlashCommand returns result with optional action
export interface SlashCommandResult {
  handled: boolean;
  response?: string;
  action?: 'stop' | 'clear';  // <-- Need to extend for pause responses
}
```

### Control Message Flow (`bridge/telegram-endpoint.ts`)

```typescript
// Send control message to WebSocket session
function sendControlMessage(ws: WebSocket, sessionId: string, action: string): void {
  ws.send(JSON.stringify({
    type: 'control',
    action,
    session_id: sessionId,
  }));
}

// Action mapping in message handler
if (result.action && state.currentSession.ws) {
  const actionMap: Record<string, string> = {
    stop: 'interrupt',
    clear: 'clear',
  };
  sendControlMessage(state.currentSession.ws, ...);
}
```

### Tool Pause Types (`codelet/tools/src/tool_pause.rs`)

```rust
pub enum PauseKind {
    Continue,
    Confirm,
    Triple,  // BLOCK-007: For blocklist prompts
}

pub enum PauseResponse {
    Resumed,
    Approved,
    Denied,
    Interrupted,
    AllowOnce,    // BLOCK-007: Permit once
    AllowSession, // BLOCK-007: Permit for session
}
```

### NAPI Binding (`codelet/napi/index.d.ts`)

```typescript
export declare function sessionPauseTriple(sessionId: string, choice: string): void
// choice: 'allow_once' | 'allow_session' | 'deny'
```

### TUI Pause Handling (`src/tui/components/AgentView.tsx`)

```typescript
// Triple pause UI with keyboard navigation
if (displayPauseInfo.kind === 'triple') {
  const choices = ['allow_once', 'allow_session', 'deny'];
  sessionPauseTriple(currentSessionId, choices[triplePauseSelection]);
}
```

---

## Required Implementation

### 1. New Commands

| Command | Action | Description |
|---------|--------|-------------|
| `/allowonce` or `/allow` | `pause_allow_once` | Allow blocked action once |
| `/allowsession` | `pause_allow_session` | Allow for entire session |
| `/deny` | `pause_deny` | Deny the action |

### 2. State Tracking

Need to track whether the session is currently paused:
- Add `isPaused: boolean` to `EndpointState`
- Add `pauseInfo?: { kind: 'triple', message: string }` to track pause details

### 3. Pause State Notification

The Telegram endpoint needs to know when the agent is paused. Options:
1. **WebSocket message from codelet**: Add a new chunk type `'pause_request'` that includes pause info
2. **Polling**: Query pause state periodically (not ideal)

Recommendation: Option 1 - Add pause state to chunk types

### 4. Control Message Extension

Extend control messages to support pause responses:

```typescript
// New control message format for pause responses
{
  type: 'control',
  action: 'pause_response',
  session_id: string,
  response: 'allow_once' | 'allow_session' | 'deny'
}
```

### 5. Rust BridgeManager Updates

The `bridge_handler.rs` needs to handle the new `pause_response` control action:

```rust
match action.as_str() {
    "interrupt" => { /* existing */ },
    "clear" => { /* existing */ },
    "pause_response" => {
        let response = message.get("response").unwrap();
        // Call session_pause_triple(session_id, response)
    }
}
```

---

## Integration Points

1. **telegram-slash-commands.ts**
   - Add new commands: `/allowonce`, `/allowsession`, `/deny`
   - Extend `SlashCommandResult.action` type
   - Add pause state validation (only allow when paused)

2. **telegram-endpoint.ts**
   - Track `isPaused` state
   - Handle pause request chunks from codelet
   - Extend action map for pause responses

3. **codelet/tools/src/bridge_handler.rs**
   - Handle `pause_response` control action
   - Call `session_pause_triple` with appropriate choice

4. **Chunk type for pause requests**
   - Extend `StreamChunkData` with pause notification type

---

## Dependencies

- BLOCK-007 (done): Tool pause system with PauseKind::Triple
- BRIDGE-008 (done): Control channel infrastructure
- BRIDGE-010 (done): Slash command pattern

---

## Questions to Resolve

1. Should pause state commands show an error if not currently paused, or silently ignore?
   - Recommendation: Show error "⚠️ No pending pause to respond to"

2. Should we show the pause details when paused (like in TUI)?
   - Recommendation: Yes, show pause message like "⏸ Read: Sensitive file access (.env)"

3. How to display pause state in /status?
   - Add 'paused' state alongside idle/thinking/executing

---

## Estimates

- Small complexity change - extends existing slash command pattern
- Estimated: 3-5 story points
