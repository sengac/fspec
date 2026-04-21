# Telegram Bridge Endpoint

Bridges codelet AI sessions to Telegram so you can monitor and interact with the agent from your phone.

## Quick Start

### Step 1: Create a Telegram Bot

1. Open Telegram on your phone or desktop
2. Search for **@BotFather** and start a chat
3. Send `/newbot`
4. Follow the prompts:
   - Enter a **name** for your bot (e.g., "My Codelet Bridge")
   - Enter a **username** ending in `bot` (e.g., `my_codelet_bridge_bot`)
5. BotFather will give you a token like:
   ```
   4839574812:AAFD39kkdpWt3ywyRZergyOLMaJhac60qc
   ```
   **Keep this token secret!**

### Step 2: Configure the Endpoint

Edit the `.env` file in this directory:

```bash
nano bridge/.env
```

Replace `your_bot_token_here` with your actual token:

```bash
TELEGRAM_BOT_TOKEN=4839574812:AAFD39kkdpWt3ywyRZergyOLMaJhac60qc
```

### Step 3: Start the Endpoint

```bash
# Foreground (see logs)
npm run bridge:telegram

# Background (runs in background, logs to bridge/telegram.log)
npm run bridge:telegram:bg

# Stop background process
npm run bridge:telegram:stop
```

You should see:
```
[telegram-endpoint] No TELEGRAM_CHAT_ID configured - waiting for first Telegram message
[telegram-endpoint] WebSocket server listening on localhost:8181
[telegram-endpoint] Telegram bot connected with polling mode
```

### Step 4: Link Your Chat

1. Open Telegram
2. Search for your bot by username (e.g., `@my_codelet_bridge_bot`)
3. Send any message (e.g., "hi")
4. The endpoint will learn your chat ID and start sending messages to you

### Step 5: Connect the AI Session

Run the skill file to connect:

```bash
# In your codelet/claude session, run:
/skill skills/telegram-bridge.md
```

Or manually ask the agent:
```
Connect to the Telegram bridge at ws://localhost:8181
```

---

## NPM Scripts

| Script | Description |
|--------|-------------|
| `npm run bridge:telegram` | Start bridge in foreground |
| `npm run bridge:telegram:bg` | Start bridge in background |
| `npm run bridge:telegram:stop` | Stop background bridge |

---

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `TELEGRAM_BOT_TOKEN` | Yes | - | Bot token from BotFather |
| `TELEGRAM_CHAT_ID` | No | - | Pre-configure chat ID for immediate delivery |
| `WEBSOCKET_PORT` | No | 8181 | Port for WebSocket server |
| `WEBSOCKET_HOST` | No | localhost | Host for WebSocket server |

---

## Message Format

### AI → Telegram

| Chunk Type | Display |
|------------|---------|
| text | Direct message with MarkdownV2 formatting |
| thinking | 💭 [thinking content] |
| tool_call | 🔧 Running: [tool name] |
| tool_result | [tool name] [result content] |
| error | ❌ Error: [message] |
| done | ✓ |

Long messages (>4096 chars) are automatically truncated with a smart algorithm that preserves the beginning and end.

### Telegram → AI

Any message you send in Telegram is forwarded to the connected codelet session as user input.

---

## Troubleshooting

### "Missing required TELEGRAM_BOT_TOKEN"
- Make sure `.env` file exists in the `bridge/` directory
- Check that the token is set correctly (no extra spaces)

### "No chat ID linked - dropping chunk"
- Send a message to your bot in Telegram first
- Or set `TELEGRAM_CHAT_ID` in `.env`

### Bot not responding
- Check that the endpoint is running (`npm run bridge:telegram`)
- Check the console for error messages
- Verify your bot token is valid

### Getting your Chat ID
1. Message your bot in Telegram
2. Look at the endpoint console - it logs received messages with chat IDs
3. Or use `@userinfobot` in Telegram to get your chat ID
