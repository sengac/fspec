# AST Research: Image Handling for Telegram Bridge

## Research Goal
Identify code locations that need modification to support incoming image attachments from Telegram.

## Telegram Bot API Research Summary (see telegram-api-research.md for details)

- `msg.photo` is an array of `PhotoSize` objects sorted by resolution ASCENDING
- Always use LAST element for highest resolution: `msg.photo[msg.photo.length - 1]`
- Use `msg.caption` (NOT `msg.text`) for photo messages
- Download via `bot.getFileLink(file_id)` → base64 conversion

---

## Code Locations to Modify

### 1. InboundMessage Interface (line 59)
```typescript
// bridge/telegram-endpoint.ts:59
// CURRENT:
interface InboundMessage {
  type: 'input';
  session_id: string;
  message: string;
}

// CHANGE TO:
interface InboundMessage {
  type: 'input';
  session_id: string;
  message: string;
  images?: Array<{data: string, media_type: string}>;
}
```

### 2. Add New Helper: getMediaTypeFromPath()
```typescript
// Add near line 112 (after escapeMarkdownV2)
export function getMediaTypeFromPath(filePath: string): string {
  const ext = filePath.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'png': return 'image/png';
    case 'jpg':
    case 'jpeg': return 'image/jpeg';
    case 'gif': return 'image/gif';
    case 'webp': return 'image/webp';
    default: return 'image/jpeg'; // Default for Telegram photos
  }
}
```

### 3. Add New Helper: downloadPhotoAsBase64()
```typescript
// Add near line 130 (after getMediaTypeFromPath)
export async function downloadPhotoAsBase64(
  bot: TelegramBotInstance,
  fileId: string
): Promise<{data: string, media_type: string} | null> {
  try {
    const fileLink = await bot.getFileLink(fileId);
    const response = await fetch(fileLink);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    const buffer = await response.arrayBuffer();
    const base64 = Buffer.from(buffer).toString('base64');
    const mediaType = getMediaTypeFromPath(fileLink);
    return { data: base64, media_type: mediaType };
  } catch (error) {
    console.error('[telegram-endpoint] Photo download failed:', error);
    return null;
  }
}
```

### 4. Modify handleTelegramMessage() (line 398)
```typescript
// CURRENT:
function handleTelegramMessage(chatId: string, text: string): InboundMessage

// CHANGE TO:
function handleTelegramMessage(
  chatId: string,
  text: string,
  images?: Array<{data: string, media_type: string}>
): InboundMessage {
  state.chatId = chatId;
  return {
    type: 'input',
    session_id: state.currentSession.sessionId || '',
    message: text,
    ...(images && images.length > 0 && { images }),
  };
}
```

### 5. Modify bot.on('message') Handler (line 478)
```typescript
// CURRENT:
bot.on('message', msg => {
  const chatId = msg.chat.id.toString();
  const text = msg.text || '';
  // ... forwards text only
});

// CHANGE TO:
bot.on('message', async msg => {
  const chatId = msg.chat.id.toString();
  
  // Check for photo message
  if (msg.photo && msg.photo.length > 0) {
    const text = msg.caption || '';  // Use caption, not text!
    const highestRes = msg.photo[msg.photo.length - 1]; // Last = highest resolution
    
    let images: Array<{data: string, media_type: string}> = [];
    const imageData = await downloadPhotoAsBase64(bot, highestRes.file_id);
    if (imageData) {
      images = [imageData];
    } else {
      console.error('[telegram-endpoint] Photo download failed, forwarding caption only');
    }
    
    // Don't send if no caption and no image
    if (!text && images.length === 0) {
      console.warn('[telegram-endpoint] Photo download failed with no caption, dropping message');
      return;
    }
    
    if (state.currentSession.ws && state.currentSession.ws.readyState === WebSocket.OPEN) {
      const inputMessage = handleTelegramMessage(chatId, text, images);
      state.currentSession.ws.send(JSON.stringify(inputMessage));
    }
    return;
  }
  
  // Regular text message (existing logic)
  const text = msg.text || '';
  // ... rest of existing code
});
```

---

## Test Scenarios (9 total)

1. Download highest resolution photo from Telegram
2. Include caption text with photo
3. Handle photo without caption
4. Detect correct media type from file extension (5 examples)
5. Default to image/jpeg for unknown extension
6. Drop photo when no active session
7. Forward caption when photo download fails
8. Drop message completely when photo download fails and no caption
9. Ignore non-photo media types

---

## Files Affected

| File | Changes |
|------|---------|
| `bridge/telegram-endpoint.ts` | InboundMessage interface, 2 new helpers, modified handler |
| `bridge/__tests__/telegram-image-handling.test.ts` | New test file (to be created) |
