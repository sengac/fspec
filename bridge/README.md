# fspec Bridge Server

WebSocket relay hub and endpoint bridges for connecting fspec AI sessions to external clients — Telegram, mobile apps, or custom WebSocket clients.

## Architecture

```
┌──────────────┐     WebSocket      ┌──────────────┐     WebSocket      ┌──────────────┐
│  Telegram    │ ◄─────────────────►│  Telegram    │                    │              │
│  Endpoint    │                    │  Endpoint    │                    │  Relay      │
│  (port 8181) │                    │              │                    │  Server     │
└──────────────┘                    │  Relay       │ ◄─────────────────►│  (port 8765) │
                                    │  Endpoint    │                    └──────────────┘
                                    │  (port 8181) │
                                    └──────────────┘
```

- **Relay Server** (`relay-server.ts`) — Pure WebSocket message router. Channel-based routing via `channel_id` from auth handshake. Handles auth, ping/pong directly; forwards everything else.
- **Telegram Endpoint** (`telegram-endpoint.ts`) — Bridges codelet sessions to Telegram. WebSocket server for codelet connections + Telegram Bot API polling for user messages.
- **Relay Endpoint** (`relay-endpoint.ts`) — Platform-agnostic bridge. WebSocket client connecting TO the relay server + local WebSocket server for codelet connections.

---

## Components

### Relay Server (`bridge:server`)

Standalone WebSocket hub that routes messages between relay endpoints and mobile/custom clients.

**Environment variables:**
| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RELAY_SERVER_PORT` | No | 8765 | Port for the relay server |
| `RELAY_SERVER_API_KEY` | No | (open) | API key required for clients to authenticate |

### Telegram Endpoint (`bridge:telegram`)

Bridges codelet AI sessions to Telegram so you can monitor and interact with the agent from your phone.

**Environment variables:**
| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `TELEGRAM_BOT_TOKEN` | Yes | - | Bot token from BotFather |
| `TELEGRAM_CHAT_ID` | No | - | Pre-configure chat ID for immediate delivery |
| `WEBSOCKET_PORT` | No | 8181 | Port for WebSocket server |
| `WEBSOCKET_HOST` | No | localhost | Host for WebSocket server |

### Relay Endpoint (`bridge:relay`)

Platform-agnostic bridge endpoint connecting fspec to the relay server.

**Environment variables:**
| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RELAY_URL` | Yes | - | Full WebSocket URL of the relay server (e.g. `ws://localhost:8765`) |
| `RELAY_CHANNEL_ID` | Yes | - | Channel ID for this fspec instance |
| `RELAY_API_KEY` | Yes | - | API key for relay authentication |
| `WEBSOCKET_PORT` | No | 8181 | Port for local WebSocket server |

---

## Quick Start

### Relay Server

```bash
# Foreground (see logs)
npm run start:server

# Background (runs in background, logs to server.log)
npm run start:server:bg

# Stop background process
npm run stop:server
```

### Telegram Endpoint

#### Step 1: Create a Telegram Bot

1. Open Telegram on your phone or desktop
2. Search for **@BotFather** and start a chat
3. Send `/newbot`
4. Follow the prompts:
   - Enter a **name** for your bot (e.g., "My Fspec Bridge")
   - Enter a **username** ending in `bot` (e.g., `my_fspec_bridge_bot`)
5. BotFather will give you a token like:
   ```
   4839574812:AAFD39kkdpWt3ywyRZergyOLMaJhac60qc
   ```
   **Keep this token secret!**

#### Step 2: Configure the Endpoint

Edit the `.env` file in this directory:

```bash
nano .env
```

Replace `your_bot_token_here` with your actual token:

```bash
TELEGRAM_BOT_TOKEN=4839574812:AAFD39kkdpWt3ywyRZergyOLMaJhac60qc
```

#### Step 3: Start the Endpoint

```bash
# Foreground (see logs)
npm run start:telegram

# Background (runs in background, logs to telegram.log)
npm run start:telegram:bg

# Stop background process
npm run stop:telegram
```

You should see:
```
[telegram-endpoint] No TELEGRAM_CHAT_ID configured - waiting for first Telegram message
[telegram-endpoint] WebSocket server listening on localhost:8181
[telegram-endpoint] Telegram bot connected with polling mode
```

#### Step 4: Link Your Chat

1. Open Telegram
2. Search for your bot by username (e.g., `@my_fspec_bridge_bot`)
3. Send any message (e.g., "hi")
4. The endpoint will learn your chat ID and start sending messages to you

#### Step 5: Connect the AI Session

Run the skill file to connect:

```bash
# In your fspec session, run:
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
| `npm run start:server` | Start relay server in foreground |
| `npm run start:server:bg` | Start relay server in background |
| `npm run stop:server` | Stop background relay server |
| `npm run start:telegram` | Start Telegram endpoint in foreground |
| `npm run start:telegram:bg` | Start Telegram endpoint in background |
| `npm run stop:telegram` | Stop background Telegram endpoint |
| `npm run start:relay` | Start relay endpoint in foreground |
| `npm run start:relay:bg` | Start relay endpoint in background |
| `npm run stop:relay` | Stop background relay endpoint |

---

## Message Format (Telegram)

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

Any message you send in Telegram is forwarded to the connected fspec session as user input.

---

## Troubleshooting

### "Missing required TELEGRAM_BOT_TOKEN"
- Make sure `.env` file exists in this directory
- Check that the token is set correctly (no extra spaces)

### "No chat ID linked - dropping chunk"
- Send a message to your bot in Telegram first
- Or set `TELEGRAM_CHAT_ID` in `.env`

### Bot not responding
- Check that the endpoint is running (`npm run start:telegram`)
- Check the console for error messages
- Verify your bot token is valid

### Getting your Chat ID
1. Message your bot in Telegram
2. Look at the endpoint console - it logs received messages with chat IDs
3. Or use `@userinfobot` in Telegram to get your chat ID
