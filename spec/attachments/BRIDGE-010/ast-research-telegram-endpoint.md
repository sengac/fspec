# AST Research: Telegram Endpoint for Slash Commands

## Research Purpose
Understanding the telegram-endpoint.ts structure to implement slash commands.

## File Analyzed
`bridge/telegram-endpoint.ts`

## Key Findings

### Message Handler Structure (Lines 566-653)
The `setupTelegramBot` function's message handler is the key integration point:

```typescript
bot.on('message', async msg => {
  const chatId = msg.chat.id.toString();

  // User ID validation (whitelist check)
  const authResult = isUserAuthorized(msg.from?.id, state.allowedUserIds);
  if (!authResult.authorized) {
    console.log(`[telegram-endpoint] Dropping message: ${authResult.reason}`);
    return;
  }

  // ... photo handling ...

  // Regular text message - THIS IS WHERE SLASH COMMANDS SHOULD BE INTERCEPTED
  const text = msg.text || '';

  // Update active chat ID
  state.chatId = chatId;

  // If we have a connected session, forward the message
  if (state.currentSession.ws && state.currentSession.ws.readyState === WebSocket.OPEN) {
    const inputMessage = handleTelegramMessage(chatId, text);
    state.currentSession.ws.send(JSON.stringify(inputMessage));
  }
});
```

### Integration Point
**Before** forwarding the message to the session (line 639-648), we should:
1. Check if `text.startsWith('/')`
2. Parse the command and any arguments
3. Handle known commands (/help, /status, /stop, /clear)
4. For unknown commands, return error with available commands
5. Send response via `bot.sendMessage(chatId, response)`
6. Return early (don't forward to agent)

### State Access
The handler has access to:
- `state.bot` - Telegram bot instance for sending responses
- `state.chatId` - Current chat ID
- `state.currentSession` - Session info (ws, sessionId)
- `state.isRunning` - Whether endpoint is running

### Session State for /status
Need to track agent state - currently not exposed directly.
Options:
1. Add `agentState: 'idle' | 'thinking' | 'executing'` to state
2. Track based on received chunks (tool_call = executing, text = thinking, done = idle)

### For /stop
Need to send interrupt signal to session. The WebSocket protocol may need extension.

### For /clear
Need to send session reset command. May require new message type.

## Proposed Architecture

```typescript
// New file: bridge/telegram-slash-commands.ts

export interface SlashCommandResult {
  handled: boolean;
  response?: string;
}

export async function handleSlashCommand(
  text: string,
  state: EndpointState,
  bot: TelegramBotInstance,
  chatId: string
): Promise<SlashCommandResult>
```

## Dependencies
- No external dependencies required
- Uses existing bot.sendMessage for responses
- May need WebSocket protocol extension for /stop and /clear
