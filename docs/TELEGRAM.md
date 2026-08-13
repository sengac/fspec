# Telegram Bridge

Monitor and interact with your factory from your phone. The Bridge tool connects any session to external WebSocket endpoints, with a built-in Telegram integration.

## Setup

1. **Create a Telegram bot** — Message [@BotFather](https://t.me/botfather), send `/newbot`, get your token

2. **Configure the bridge** — Create `bridge/.env`:
   ```bash
   TELEGRAM_BOT_TOKEN=your_token_here
   TELEGRAM_ALLOWED_USER_IDS=123456789   # Your Telegram user ID (optional but recommended)
   ```

3. **Start the endpoint**:
   ```bash
   npx tsx bridge/telegram-endpoint.ts
   ```

4. **Message your bot** — Send any message to link your chat

5. **Connect the agent** — Tell it:
   ```
   Connect to the Telegram bridge at ws://localhost:8181
   ```

Now all agent output streams to Telegram. Send messages back to provide input. Run the factory overnight and check production from bed.

## Security: User Whitelist

By default, anyone who finds your bot can interact with it. Set `TELEGRAM_ALLOWED_USER_IDS` to restrict access:

```bash
# Single user
TELEGRAM_ALLOWED_USER_IDS=123456789

# Multiple users (comma-separated)
TELEGRAM_ALLOWED_USER_IDS=123456789,987654321
```

To find your Telegram user ID, message [@userinfobot](https://t.me/userinfobot) or check the bridge console output when you send a message.
