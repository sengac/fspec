# Telegram Bridge

Monitor and interact with your fspec sessions from your phone. The bridge server connects sessions to external clients via WebSocket, with a built-in Telegram integration.

## Architecture

The bridge package (`bridge/`) contains three components:

| Component | File | Purpose |
|-----------|------|---------|
| **Relay Server** | `relay-server.ts` | WebSocket hub — routes messages between endpoints and clients by channel |
| **Telegram Endpoint** | `telegram-endpoint.ts` | Bridges fspec sessions to Telegram Bot API |
| **Relay Endpoint** | `relay-endpoint.ts` | Platform-agnostic bridge connecting fspec to the relay server |

## Setup

### 1. Create a Telegram Bot

Message [@BotFather](https://t.me/botfather), send `/newbot`, follow prompts to get your token.

### 2. Configure the Bridge

Create `bridge/.env`:

```bash
TELEGRAM_BOT_TOKEN=your_token_here
TELEGRAM_ALLOWED_USER_IDS=123456789   # Your Telegram user ID (optional but recommended)
```

### 3. Start the Endpoint

```bash
cd bridge
npm run start:telegram
```

Or run in the background:

```bash
npm run start:telegram:bg
```

### 4. Message Your Bot

Send any message to your bot in Telegram to link your chat ID.

### 5. Connect the Agent

Tell the agent:

```
Connect to the Telegram bridge at ws://localhost:8181
```

Now all agent output streams to Telegram. Send messages back to provide input.

## Security: User Whitelist

By default, anyone who finds your bot can interact with it. Set `TELEGRAM_ALLOWED_USER_IDS` to restrict access:

```bash
# Single user
TELEGRAM_ALLOWED_USER_IDS=123456789

# Multiple users (comma-separated)
TELEGRAM_ALLOWED_USER_IDS=123456789,987654321
```

To find your Telegram user ID, message [@userinfobot](https://t.me/userinfobot) or check the bridge console output when you send a message.

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

## Relay Server (Optional)

For multi-client setups (e.g., mobile app + Telegram simultaneously), start the relay server:

```bash
cd bridge
npm run start:server
```

Configure in `.env`:

```bash
RELAY_SERVER_PORT=8765
RELAY_SERVER_API_KEY=your_api_key_here   # Optional — open mode if unset
```

Then connect relay endpoints:

```bash
RELAY_URL=ws://localhost:8765
RELAY_CHANNEL_ID=fspec-main
RELAY_API_KEY=your_api_key_here
```
