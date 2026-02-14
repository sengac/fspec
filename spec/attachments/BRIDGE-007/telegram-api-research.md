# Telegram Bot API Research: Photo Message Handling

## Source
- Official Telegram Bot API: https://core.telegram.org/bots/api
- Stack Overflow: https://stackoverflow.com/questions/59312499/how-should-i-download-received-files-from-telegram-api
- DEV.to article: https://dev.to/benkhalife/send-and-receive-files-from-the-telegram-bot-3h54

---

## Photo Message Structure

When a user sends a photo to a Telegram bot, the `Message` object contains:

```typescript
interface TelegramMessage {
  chat: { id: number };
  text?: string;           // NOT present for photo messages
  caption?: string;        // Text caption for media messages
  photo?: PhotoSize[];     // Array of photo sizes (ONLY for photo messages)
}
```

### PhotoSize Object
```typescript
interface PhotoSize {
  file_id: string;         // Unique identifier - use this to download
  file_unique_id: string;  // Unique across different bots
  width: number;           // Photo width in pixels
  height: number;          // Photo height in pixels
  file_size?: number;      // File size in bytes (optional)
}
```

### CRITICAL: Photo Array Order
**Telegram sends photos as an array sorted by resolution ASCENDING:**
- `msg.photo[0]` = LOWEST resolution (thumbnail, ~90px)
- `msg.photo[msg.photo.length - 1]` = HIGHEST resolution

**Always use the LAST element for best quality!**

---

## Download Process

### Step 1: Get File Info
```typescript
const fileInfo = await bot.getFile(file_id);
// Returns: { file_id, file_unique_id, file_size, file_path }
// file_path example: "photos/file_123.jpg"
```

### Step 2: Download File
Option A - Get download URL:
```typescript
const link = await bot.getFileLink(file_id);
// Returns: "https://api.telegram.org/file/bot<TOKEN>/<file_path>"
```

Option B - Get as stream:
```typescript
const stream = bot.getFileStream(file_id);
```

Option C - Manual URL construction:
```
https://api.telegram.org/file/bot<BOT_TOKEN>/<file_path>
```

---

## Text vs Caption

| Message Type | `msg.text` | `msg.caption` |
|-------------|------------|---------------|
| Text message | ✅ Present | ❌ undefined |
| Photo message | ❌ undefined | ✅ Present (if user added one) |
| Photo without caption | ❌ undefined | ❌ undefined |

**Correct handling:**
```typescript
const messageText = msg.caption || msg.text || '';
```

---

## File Size Limits

- Telegram Bot API can download files up to **20MB**
- Photos are typically compressed by Telegram before being sent
- Highest resolution is usually under 5MB

---

## Media Types

File extension can be determined from `file_path`:
- `.jpg`, `.jpeg` → `image/jpeg`
- `.png` → `image/png`  
- `.gif` → `image/gif`
- `.webp` → `image/webp`

---

## node-telegram-bot-api Specifics

The library we use (`node-telegram-bot-api`) provides:

```typescript
// Get file metadata
bot.getFile(file_id): Promise<{file_id, file_unique_id, file_size, file_path}>

// Get direct download URL
bot.getFileLink(file_id): Promise<string>

// Get readable stream
bot.getFileStream(file_id): ReadableStream
```

**Event handling:**
```typescript
bot.on('message', (msg) => {
  if (msg.photo) {
    // This is a photo message
    const photos = msg.photo; // PhotoSize[]
    const caption = msg.caption; // string | undefined
  }
});

// Or use specific photo event
bot.on('photo', (msg) => {
  // Only fires for photo messages
});
```
