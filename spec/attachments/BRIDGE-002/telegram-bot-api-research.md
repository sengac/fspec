# Telegram Bot API Research Notes

## Overview

This document contains research for implementing the Telegram Bridge Endpoint (`bridge/telegram-endpoint.js`).

---

## BotFather Setup

1. Open Telegram app
2. Search for `@BotFather` and start a chat
3. Send `/newbot` command
4. Follow instructions to set bot name and username
5. Receive API token like: `4839574812:AAFD39kkdpWt3ywyRZergyOLMaJhac60qc`

**IMPORTANT:** Keep token secure - treat it like a password.

---

## Recommended NPM Package

**`node-telegram-bot-api`** - Official/popular Node.js library for Telegram Bot API

```bash
npm install node-telegram-bot-api
```

TypeScript types available:
```bash
npm install --save-dev @types/node-telegram-bot-api
```

### Basic Usage

```javascript
const TelegramBot = require('node-telegram-bot-api');

// Get token from environment
const token = process.env.TELEGRAM_BOT_TOKEN;

// Create bot with polling
const bot = new TelegramBot(token, { polling: true });

// Listen for messages
bot.on('message', (msg) => {
  const chatId = msg.chat.id;
  bot.sendMessage(chatId, 'Received your message');
});

// Send a message
bot.sendMessage(chatId, 'Hello, World!');
```

---

## Telegram API Constraints

### Message Limits
- **Maximum message length: 4096 characters**
- Messages exceeding this must be split or truncated

### Formatting Options
- **MarkdownV2** - Preferred, supports most formatting
- **HTML** - Alternative formatting
- **Plain text** - No formatting

### MarkdownV2 Syntax
```
*bold*
_italic_
__underline__
~strikethrough~
||spoiler||
`inline code`
```pre-formatted```
```python
code block with language
```
[inline URL](http://example.com)
```

**Special characters that need escaping in MarkdownV2:**
```
_ * [ ] ( ) ~ ` > # + - = | { } . !
```

---

## Architecture Decision

### Why WebSocket Server (not direct Telegram integration)?

The endpoint runs as a **WebSocket server** that codelet's bridge connects to:

```
┌─────────────────┐        WebSocket         ┌──────────────────────────────────┐
│     CODELET     │◄───────────────────────► │   bridge/telegram-endpoint.js    │
│                 │                          │                                  │
│  BridgeManager  │  StreamChunks (JSON)     │  • WebSocket server (ws)         │
│  (Rust)         │  ────────────────────►   │  • Message formatting/truncation │
│                 │                          │  • Telegram Bot API connection   │
│                 │  Input messages          │  • Chat session management       │
│                 │  ◄────────────────────   │                                  │
└─────────────────┘                          └──────────────┬───────────────────┘
                                                            │
                                                            │ Telegram Bot API
                                                            ▼
                                                   ┌─────────────────┐
                                                   │  User's Telegram │
                                                   │     (phone)      │
                                                   └─────────────────┘
```

**Rationale:**
1. **Separation of concerns** - Bridge is a "dumb pipe", endpoint handles platform logic
2. **Flexibility** - Can swap/modify endpoint without changing codelet
3. **Single JS file** - Easy to understand, modify, deploy
4. **Bot token isolation** - Endpoint manages credentials, not codelet

---

## Message Flow

### Outbound (Codelet → Telegram)

1. Codelet's BridgeManager sends StreamChunk via WebSocket:
   ```json
   {
     "type": "chunk",
     "session_id": "uuid-here",
     "data": {
       "type": "text",
       "text": "Hello, I can help you with that."
     }
   }
   ```

2. Endpoint receives, formats, and sends to Telegram:
   - Apply 4096 char limit with smart truncation
   - Format code blocks with MarkdownV2
   - Add emoji prefixes for thinking (💭)
   - Show tool names for tool outputs

### Inbound (Telegram → Codelet)

1. User sends message in Telegram
2. Bot receives via polling
3. Endpoint sends to codelet via WebSocket:
   ```json
   {
     "type": "input",
     "session_id": "uuid-here",
     "message": "run the tests please"
   }
   ```

---

## StreamChunk Types to Handle

From BRIDGE-001's `create_outbound_message()`:

| Type | Display Format |
|------|----------------|
| `text` | Direct AI response - format with MarkdownV2 |
| `thinking` | Prefix with 💭, optionally truncate/hide |
| `tool_call` | Show tool name: `🔧 Running: Read` |
| `tool_result` | `[Read] /path/file.rs\n<content>` |
| `done` | End of response marker (optional: ✓) |
| `error` | ❌ Error: message |

---

## Truncation Strategy

### Smart Truncation (Default)

For messages > 4096 chars:
1. Keep first ~1500 chars (preserve context start)
2. Add `\n\n...[X chars omitted]...\n\n`
3. Keep last ~1500 chars (preserve conclusion)
4. Ensure total ≤ 4096

### Code Block Preservation

When truncating code blocks:
1. Detect opening ``` with language
2. If truncating mid-block, add closing ```
3. Re-open with ``` on continuation

### Truncation Indicator

Always append when truncated:
```
... [truncated, X chars]
```

---

## Chat Session Tracking

The endpoint needs to track which Telegram chat to send responses to:

```javascript
// Map: session_id -> telegram_chat_id
const chatSessions = new Map();

// When user sends first message, associate chat with session
bot.on('message', (msg) => {
  const chatId = msg.chat.id;
  // Associate with current session or create new session
  chatSessions.set(currentSessionId, chatId);
});
```

---

## Environment Variables

```bash
# Required
TELEGRAM_BOT_TOKEN=4839574812:AAFD39kkdpWt3ywyRZergyOLMaJhac60qc

# Optional
WEBSOCKET_PORT=8080
WEBSOCKET_HOST=localhost
```

---

## Dependencies

```json
{
  "dependencies": {
    "node-telegram-bot-api": "^0.66.0",
    "ws": "^8.16.0",
    "dotenv": "^16.3.1"
  }
}
```

---

## References

- [Telegram Bot API Documentation](https://core.telegram.org/bots/api)
- [From BotFather to Hello World](https://core.telegram.org/bots/tutorial)
- [node-telegram-bot-api GitHub](https://github.com/yagop/node-telegram-bot-api)
- [MarkdownV2 Formatting](https://core.telegram.org/bots/api#markdownv2-style)
