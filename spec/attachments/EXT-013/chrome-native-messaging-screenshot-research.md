# EXT-013 Research: Chrome Native Messaging Screenshot Crash

## Problem Summary

When `browser_screenshot` captures a large viewport (e.g. 1728×992 on a complex page), the base64 PNG easily exceeds 1 MB. Chrome's native messaging enforces a **1 MB limit on messages FROM the native host TO the extension**, causing the port to disconnect entirely when the oversized response is written to stdout.

## Chrome Native Messaging Limits (from Chromium source)

### Direction: Native Host → Extension (the problem direction)

**Source**: `chrome/browser/extensions/api/messaging/native_message_process_host.cc`

```cpp
// Maximum message size in bytes for messages received from Native Messaging
// hosts. Message size is limited mainly to prevent Chrome from crashing when
// native application misbehaves (e.g. starts writing garbage to the pipe).
const size_t kMaximumNativeMessageSize = 1024 * 1024;  // 1 MB
```

**Behavior when exceeded**: Chrome reads the 4-byte length header, sees the message exceeds `kMaximumNativeMessageSize`, and **closes the connection** (calls `Close()` with an error). This kills the entire native messaging port — all pending and future messages fail.

### Direction: Extension → Native Host

Per Chrome docs (updated 2025): **64 MiB** (recently reduced from 4 GB — see [Chromium issue #420956735](https://issues.chromium.org/issues/420956735)). This direction is NOT the problem for screenshots.

### Protocol Format

```
[4 bytes: uint32 LE length] [N bytes: JSON UTF-8]
```

Each message is framed with a 4-byte little-endian length prefix followed by the JSON payload.

## Current Code Path (Screenshot Flow)

### 1. MCP Agent calls `browser_screenshot`

Agent → HTTP POST to MCP server (`localhost:19876/mcp`) → `tools/call` with `browser_screenshot`

### 2. MCP Server relays to extension via native messaging

```
mcp-server.mjs: callExtension() →
  encodeNativeMessage({type: 'TOOL_CALL', correlationId, params}) →
  stdout.write(frame)  // → Chrome reads from native host's stdout
```

Chrome delivers this to the service worker's `port.onMessage`.

### 3. Service Worker processes the tool call

```
service-worker.ts: port.onMessage →
  message-router.ts: handleNativeMessage() →
    browserTools.getHandler('browser_screenshot') →
    browser-tools.ts: handler executes
```

### 4. browser_screenshot handler captures image

```typescript
// browser-tools.ts line 304-316
const dataUrl = await tabs.captureVisibleTab(windowId, { format: 'png' });
const base64Data = dataUrl.replace(/^data:image\/png;base64,/, '');
return {
  content: [
    { type: 'image', data: base64Data, mimeType: 'image/png' },
  ],
};
```

`captureVisibleTab()` returns a data URL. For a 1728×992 viewport on a complex page, the PNG can be **2-4 MB** of base64 data.

### 5. Result sent back via native messaging

```
message-router.ts: sendToNativeHost({correlationId, result}) →
  port.postMessage(message)  // Extension → Native Host direction (64 MB limit, OK)
```

**Wait — the extension-to-host direction has a 64 MB limit!** So `port.postMessage()` from the service worker should succeed...

### 6. Native host receives and relays to MCP

```
native-messaging.mjs: createNativeMessageReader() reads from stdin →
  mcp-server.mjs: handleNativeMessage() →
    pendingCalls.get(correlationId).resolve({result}) →
      MCP HTTP response sent to agent
```

### 7. THE ACTUAL CRASH POINT

The crash happens in `mcp-server.mjs` when it tries to send the response back to the HTTP client. But wait — let me re-examine...

Actually, re-reading the bug description more carefully:

> "Chrome's native messaging enforces a 1MB limit on port.postMessage() in BOTH directions"

This is **incorrect** per the Chromium source. The 1 MB limit is only **host → extension**. Extension → host is 64 MiB. BUT let's look at the actual flow more carefully:

The MCP server writes the response to the HTTP response — that's fine, no native messaging involved.

**BUT**: the native host writes back to Chrome via stdout in `encodeNativeMessage()`:

```javascript
// native-messaging.mjs line 22-24
if (jsonBytes.length > MAX_MESSAGE_SIZE) {
    throw new Error(`Message exceeds max size: ${jsonBytes.length} > ${MAX_MESSAGE_SIZE}`);
}
```

Wait — this is the **native host's own code** that enforces the 1 MB limit! The `encodeNativeMessage()` function in `native-messaging.mjs` throws when the message exceeds 1 MB. But this code is only used for sending messages FROM the native host TO the extension. Screenshots flow in the opposite direction (extension → native host), so this shouldn't be hit.

### Re-analysis: Where Does the Crash Actually Happen?

Let me trace more carefully:

1. Agent calls `browser_screenshot` → MCP server → native messaging → Chrome → service worker
2. Service worker's browser_screenshot handler captures PNG, returns result
3. **The result goes FROM the service worker TO the native host** via `port.postMessage()` — this writes to the native host's stdin. **Extension → host direction = 64 MB limit. Should be fine.**
4. The native host reads the message from stdin via `createNativeMessageReader()`, resolves the pending promise, and sends the HTTP response.

So where does it crash? Looking at the native-messaging.mjs reader:

```javascript
// Line 72-74
if (length > MAX_MESSAGE_SIZE) {
    // Invalid frame — skip
    buffer = Buffer.alloc(0);
    break;
}
```

**This is the secondary bug!** The reader applies the same 1 MB check to INCOMING messages, but Chrome allows up to 64 MB in this direction. When Chrome sends a message larger than 1 MB from the extension, the reader **discards it** and corrupts the buffer.

### Root Cause Summary

1. **Primary bug**: `native-messaging.mjs` `createNativeMessageReader()` line 72-74 enforces `MAX_MESSAGE_SIZE` (1 MB) on INCOMING messages from Chrome, but Chrome allows up to 64 MiB in the extension→host direction. When the screenshot result exceeds 1 MB, the reader treats it as an "invalid frame" and zeros the buffer, which:
   - Drops the screenshot response (pending call times out after 30s)
   - Corrupts the stream by discarding ALL buffered data, causing all subsequent messages to fail

2. **Secondary bug**: Even if we fix the reader, the raw base64 PNG data is very large and wasteful for LLM consumption. A 1728×992 PNG screenshot can be 2-4 MB of base64, while Claude's optimal image size is 1568px on the long edge.

## Fix Strategy for EXT-013

### Fix 1: Native Message Reader — Increase Incoming Limit

The reader in `native-messaging.mjs` should use a separate, larger limit for incoming messages (since Chrome allows 64 MiB in this direction):

```javascript
const MAX_OUTGOING_MESSAGE_SIZE = 1024 * 1024;       // 1 MB (host → extension)
const MAX_INCOMING_MESSAGE_SIZE = 64 * 1024 * 1024;  // 64 MiB (extension → host)
```

And the reader should use `MAX_INCOMING_MESSAGE_SIZE` for its validation.

### Fix 2: Fix buffer corruption on oversized messages

When an oversized message is detected, instead of `buffer = Buffer.alloc(0)` (which discards ALL buffered data including partial next messages), skip exactly `4 + length` bytes:

```javascript
if (length > MAX_INCOMING_MESSAGE_SIZE) {
    // Skip the oversized message but preserve stream integrity
    if (buffer.length >= 4 + length) {
        buffer = buffer.subarray(4 + length);
    } else {
        // Not enough data yet — wait for more and skip once complete
        // Need to track "skip remaining" state
    }
    break;
}
```

### Fix 3: Screenshot Slicing in Service Worker

Even with the reader fix, sending multi-MB images through the pipeline is wasteful. The `browser_screenshot` handler should:

1. **Resize** the captured PNG down to 1568px max on the long edge (Claude's optimal)
2. **Convert** PNG → JPEG at quality 80% (screenshots are photos of rendered pages)
3. **Check size**: If the resulting base64 + JSON wrapper exceeds ~800 KB, slice the image into vertical tiles
4. **Return multiple image content blocks** if sliced

This can be done in the service worker using `OffscreenCanvas` (available in service workers):

```typescript
// Pseudo-code for the slicing approach
const dataUrl = await tabs.captureVisibleTab(windowId, { format: 'png' });
const bitmap = await createImageBitmap(await fetch(dataUrl).then(r => r.blob()));

const maxDim = 1568;
const scale = Math.min(1, maxDim / Math.max(bitmap.width, bitmap.height));
const targetW = Math.round(bitmap.width * scale);
const targetH = Math.round(bitmap.height * scale);

// Calculate max tile height to stay under 800KB per tile
const MAX_TILE_BYTES = 800 * 1024; // 800KB target (well under 1MB)
const tileHeight = Math.min(targetH, estimateSafeTileHeight(targetW, targetH));

const tiles: string[] = [];
for (let y = 0; y < targetH; y += tileHeight) {
    const h = Math.min(tileHeight, targetH - y);
    const canvas = new OffscreenCanvas(targetW, h);
    const ctx = canvas.getContext('2d')!;
    ctx.drawImage(bitmap, 0, y / scale, bitmap.width, h / scale, 0, 0, targetW, h);
    const blob = await canvas.convertToBlob({ type: 'image/jpeg', quality: 0.80 });
    // Convert blob to base64
    const arrayBuffer = await blob.arrayBuffer();
    const base64 = btoa(String.fromCharCode(...new Uint8Array(arrayBuffer)));
    tiles.push(base64);
}

return {
    content: tiles.map(base64 => ({
        type: 'image' as const,
        data: base64,
        mimeType: 'image/jpeg',
    })),
};
```

### Size Estimation

For a 1568×907 JPEG at quality 80%:
- Typical JPEG size: 100-300 KB (raw bytes)
- Base64 expansion: ~33% overhead → 133-400 KB
- JSON wrapper overhead: ~100 bytes
- **Total per tile: well under 800 KB**

For a 1568×3000 full-page screenshot:
- Split into ~3 tiles of 1568×1000 each
- Each tile: ~200-400 KB base64
- **Total: 600 KB - 1.2 MB across 3 content blocks**

## Key Chromium Source References

### native_message_process_host.cc

- `kMaximumNativeMessageSize = 1024 * 1024` — 1 MB limit for host→extension
- `ProcessIncomingData()` — reads messages from native host's stdout
- When message exceeds limit, calls `Close(kHostInputOutputError)` which disconnects the port

### messaging_util.cc

- Extension→host direction: enforces 64 MiB limit (changed from 4 GB around 2025)
- `kMaximumExtensionMessageSize = 64 * 1024 * 1024`

### Chrome docs (developer.chrome.com)

- "The maximum size of a single message from the native messaging host is 1 MB"
- "The maximum size of the message sent to the native messaging host is 64 MiB"

## Files to Modify

| File | Change |
|------|--------|
| `extension/host/lib/native-messaging.mjs` | Fix reader: separate incoming/outgoing limits, fix buffer corruption |
| `extension/src/background/browser-tools.ts` | Add screenshot resize + JPEG conversion + slicing |
| `extension/src/background/browser-tools-types.ts` | No changes needed (McpImageContent already supports arrays) |
| `extension/host/lib/mcp-server.mjs` | No changes needed (passes through result as-is) |

## Risk Assessment

- **OffscreenCanvas availability**: Available in Chrome service workers since Chrome 69. Our minimum Chrome version supports this.
- **createImageBitmap**: Available in service workers. Can create from blob/data URL.
- **convertToBlob**: Available on OffscreenCanvas. Supports JPEG output with quality parameter.
- **Base64 encoding in service worker**: Use `FileReader` or manual `btoa()` approach. `btoa()` has string length limits but we're working with tiles under 1 MB, so this is fine.
- **Memory**: Creating ImageBitmaps and OffscreenCanvases for large images may use significant memory. For a 1728×992 image, this is ~7 MB of RGBA data — well within service worker memory limits.
