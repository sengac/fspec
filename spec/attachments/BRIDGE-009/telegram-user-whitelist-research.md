# Telegram User ID Whitelist - Technical Research

## Overview

This document outlines the implementation approach for adding user-level access control to the Telegram bridge endpoint. The goal is to whitelist specific Telegram user IDs and silently drop messages from unauthorized users.

## Telegram API: User Identification

### Message Object Structure

When a Telegram bot receives a message, the `Message` object contains:

```typescript
interface Message {
  message_id: number;
  from?: User;        // Sender - may be undefined for channel posts
  chat: Chat;         // Chat where message was sent
  text?: string;
  // ... other fields
}
```

### User Object Structure

```typescript
interface User {
  id: number;           // Unique Telegram user ID (permanent, numeric)
  is_bot: boolean;      // True if the user is a bot
  first_name: string;   // User's first name
  last_name?: string;   // Optional last name
  username?: string;    // Optional @username (can change!)
  language_code?: string;
}
```

### Key Points

1. **`msg.from.id`** - This is the **user's unique Telegram ID** (a number like `123456789`)
   - Permanent and never changes for a user
   - Different from `msg.chat.id` (the chat ID)

2. **`msg.from` is optional** - For channel posts or anonymous messages, `from` may be undefined
   - Must handle this case in validation

3. **`msg.chat.id` vs `msg.from.id`**
   - In private chats: `chat.id === from.id` (same value)
   - In groups: `chat.id` is the group, `from.id` is the user who sent the message
   - **We MUST use `from.id` for user-level whitelisting** (not `chat.id`)

## Implementation Approach

### 1. Environment Variable Configuration

```bash
# Comma-separated list of authorized Telegram user IDs
TELEGRAM_ALLOWED_USER_IDS=123456789,987654321,555555555
```

### 2. State Changes

Add to `EndpointState`:

```typescript
export interface EndpointState {
  // ... existing fields
  allowedUserIds: Set<number> | null;  // null = allow all (no whitelist)
}
```

### 3. Validation Logic

Add early in message handler before any processing:

```typescript
bot.on('message', async msg => {
  // User ID validation (must be first check)
  const userId = msg.from?.id;
  
  // If whitelist is configured, validate
  if (state.allowedUserIds !== null) {
    // No user ID available (channel post or system message)
    if (userId === undefined) {
      console.log('[telegram-endpoint] Dropping message: no user ID');
      return;
    }
    
    // User not in whitelist
    if (!state.allowedUserIds.has(userId)) {
      console.log(`[telegram-endpoint] Dropping message from unauthorized user: ${userId}`);
      return;
    }
  }
  
  // Continue with existing message handling...
});
```

### 4. Initialization

In `startEndpoint()`:

```typescript
// Parse allowed user IDs from environment
const allowedUserIdsEnv = process.env.TELEGRAM_ALLOWED_USER_IDS;
if (allowedUserIdsEnv) {
  const ids = allowedUserIdsEnv
    .split(',')
    .map(s => parseInt(s.trim(), 10))
    .filter(n => !isNaN(n));
  
  if (ids.length > 0) {
    state.allowedUserIds = new Set(ids);
    console.log(`[telegram-endpoint] User whitelist enabled: ${ids.length} user(s)`);
  } else {
    state.allowedUserIds = null;
    console.warn('[telegram-endpoint] TELEGRAM_ALLOWED_USER_IDS set but no valid IDs found');
  }
} else {
  state.allowedUserIds = null;
  console.log('[telegram-endpoint] No user whitelist configured - accepting all users');
}
```

## Code Locations to Modify

### `bridge/telegram-endpoint.ts`

1. **Line ~66-82** - Add `allowedUserIds: Set<number> | null` to `EndpointState`
2. **Line ~97-112** - Initialize `allowedUserIds: null` in state
3. **Line ~555-559** - In `setupTelegramBot()`, add user validation at start of `bot.on('message')` handler
4. **Line ~654-690** - In `startEndpoint()`, parse `TELEGRAM_ALLOWED_USER_IDS` environment variable
5. **Line ~729-744** - In `resetState()`, reset `allowedUserIds` to null

## Getting Your Telegram User ID

Users can find their Telegram user ID by:

1. Messaging `@userinfobot` on Telegram
2. Messaging `@getmyid_bot` on Telegram
3. Using the bot's logs - first unauthorized attempt will log the user ID

## Security Considerations

1. **Silent dropping** - Unauthorized messages are silently dropped (no error response)
   - Prevents enumeration attacks
   - No feedback to attackers

2. **Logging** - Log unauthorized attempts for audit trail
   - Include user ID for tracking
   - Don't log message content (privacy)

3. **Whitelist vs Blacklist** - Whitelist approach is more secure
   - Default deny - only explicitly allowed users can access
   - New users don't automatically get access

4. **Bot tokens** - Remember that the bot token itself is also a security boundary
   - User whitelist adds defense in depth

## Testing Approach

1. **No whitelist configured** - All messages should be processed
2. **Empty whitelist** - Should warn and allow all (or deny all - decision needed)
3. **Valid whitelist** - Only whitelisted users should get through
4. **Unauthorized user** - Message should be dropped with log entry
5. **No `from` field** - Message should be dropped
6. **Invalid IDs in env** - Should be filtered out, valid IDs should work

## Open Questions

1. Should we respond to unauthorized users with a message? (Suggested: No - silent drop)
2. Should empty whitelist allow all or deny all? (Suggested: Allow all with warning)
3. Should we support runtime whitelist updates? (Suggested: No - restart required)
