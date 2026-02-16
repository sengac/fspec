# AST Research: Telegram Endpoint Structure

## Purpose
Identify code locations for implementing user ID whitelist feature.

## Key Code Locations

### 1. EndpointState Interface (line 66)
```
/home/rquast/projects/fspec/bridge/telegram-endpoint.ts:66
```
**Action:** Add `allowedUserIds: Set<number> | null` field

### 2. Message Handler (line 558)
```
/home/rquast/projects/fspec/bridge/telegram-endpoint.ts:558
bot.on('message', async msg => { ... })
```
**Action:** Add user ID validation at the start of this handler before any processing

### 3. startEndpoint Function (line 654)
```
/home/rquast/projects/fspec/bridge/telegram-endpoint.ts:654
function startEndpoint(): EndpointState { ... }
```
**Action:** Parse `TELEGRAM_ALLOWED_USER_IDS` env var and initialize `state.allowedUserIds`

## Integration Points

1. **State initialization (line 97-112):** Initialize `allowedUserIds: null`
2. **resetState function (line 729-744):** Reset `allowedUserIds` to null
3. **Message handler validation:** Must check `msg.from?.id` before existing chat ID handling

## User ID Access

From Telegram Bot API types:
- `msg.from?.id` - User's unique Telegram ID (number)
- `msg.from` is optional (undefined for channel posts)
- Different from `msg.chat.id` (chat ID, same for private chats but different for groups)

## Test File Location
```
/home/rquast/projects/fspec/bridge/__tests__/telegram-endpoint.test.ts
```
New tests should be added here following existing patterns.
