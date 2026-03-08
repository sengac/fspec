# AST Research: Element-Targeted Screenshot

## Date: 2026-03-08
## Work Unit: EXT-015

---

## 1. Handler Registration Points

All browser tool handlers are registered via `handlers.set()` in `browser-tools.ts`:

| Handler | Location |
|---------|----------|
| `browser_screenshot` | `browser-tools.ts:340` |
| `browser_click_element` | `browser-tools.ts:583` |
| `browser_fill_form` | `browser-tools.ts:621` |
| `browser_scan_page` | `browser-tools.ts:718` |

The `browser_screenshot` handler at line 340 accepts `args.tabId` only — no `selector` parameter.

## 2. Ref Resolution Infrastructure

### resolveRefSelector (line 558)
```typescript
function resolveRefSelector(
  selector: string,
  tabId: number
): { selector: string; frameId: number } | McpToolResult
```

- If selector starts with `@`: looks up ref via `resolveRef(tabId, refKey)`
- Returns `{ selector: cssSelector, frameId }` on success
- Returns `errorResult()` if ref not found
- Non-`@` selectors pass through unchanged with `frameId: 0`

### isResolveError (line 576)
Type guard checking if result has `content` property (error case).

### Usage pattern (from browser_click_element, line 583-596):
```typescript
const resolved = resolveRefSelector(selector, tabId);
if (isResolveError(resolved)) {
  return resolved;
}
selector = resolved.selector;
const frameId = resolved.frameId;
const target = frameId > 0 ? { tabId, frameIds: [frameId] } : { tabId };
```

This exact pattern can be reused in browser_screenshot.

## 3. Current Screenshot Pipeline (lines 340-425)

1. `resolveTabId(args.tabId)` → get tab ID
2. `tabs.get(tabId)` → get tab metadata (windowId)
3. `tabs.captureVisibleTab(windowId, { format: 'png' })` → PNG data URL
4. `fetch(dataUrl)` → blob → `createImageBitmap(blob)` → bitmap
5. Calculate resize: `MAX_LONG_EDGE = 1568`, scale proportionally
6. `new OffscreenCanvas(targetW, targetH)` → draw bitmap resized
7. `canvasToJpegBase64(fullCanvas, JPEG_QUALITY=0.8)`
8. If < 800KB → return single image block
9. If > 800KB → tile vertically, quality fallback per tile

### canvasToJpegBase64 (line 126)
```typescript
async function canvasToJpegBase64(canvas: OffscreenCanvas, quality: number): Promise<string>
```
Converts OffscreenCanvas → JPEG blob → ArrayBuffer → Uint8Array → base64 string.

### OffscreenCanvas drawImage crop pattern (line 406, tiling code):
```typescript
tileCtx.drawImage(fullCanvas, 0, y, targetW, h, 0, 0, targetW, h);
```
Uses the 9-argument `drawImage(source, sx, sy, sw, sh, dx, dy, dw, dh)` form for cropping.

## 4. executeScript Pattern (for scrollIntoView + getBoundingClientRect)

From `browser_click_element` (line 598-619):
```typescript
const results = await scripting.executeScript({
  target: target,
  func: (sel: string) => {
    const el = document.querySelector(sel);
    if (!el) return null;
    // ... interact with element ...
  },
  args: [selector],
});
```

From `browser_fill_form` (line 643-662):
```typescript
const results = await scripting.executeScript({
  target: target,
  func: (sel: string, val: string) => {
    const el = document.querySelector(sel) as HTMLInputElement | ...;
    if (!el) return { success: false, error: '...' };
    el.scrollIntoView({ block: 'center', behavior: 'instant' });
    // ...
  },
  args: [selector, value],
});
```

Key observations:
- `scrollIntoView({ block: 'center' })` already used in fill_form
- `document.querySelector()` already used in click/fill
- Frame-aware targeting via `{ tabId, frameIds: [frameId] }` already proven

## 5. getBoundingClientRect Usage

From `scan-page-dom.ts` (line 296, 413):
```typescript
const rect = el.getBoundingClientRect();
```

From `dom-scanner-helpers.ts` (line 172):
```typescript
const rect = el.getBoundingClientRect();
```

These return `{ x, y, width, height, top, right, bottom, left }` in CSS pixels.

## 6. MCP Tool Schema (mcp-server.mjs line 34-44)

```javascript
{
  name: 'browser_screenshot',
  description: 'Capture a screenshot of a browser tab',
  inputSchema: {
    type: 'object',
    properties: {
      tabId: { type: 'number', description: 'Tab ID to capture' },
      fullPage: { type: 'boolean', description: 'Capture full scrollable page' },
    },
  },
},
```

Needs: add `selector` property to `inputSchema.properties`.

## 7. Implementation Plan

### Changes needed:

1. **`browser-tools.ts` — `browser_screenshot` handler (line 340)**
   - Add `selector` parameter extraction from args
   - If selector present: resolve via `resolveRefSelector()`
   - executeScript to scrollIntoView + getBoundingClientRect + devicePixelRatio
   - Validate non-zero dimensions
   - After captureVisibleTab: crop bitmap using element rect (DPR-scaled)
   - Feed cropped canvas into existing resize/JPEG/tile pipeline

2. **`mcp-server.mjs` — NATIVE_TOOLS entry (line 34-44)**
   - Add `selector` property to inputSchema

### Reusable infrastructure (no changes needed):
- `resolveRefSelector()` — already handles @ref → CSS + frameId
- `isResolveError()` — already handles error type guard
- `canvasToJpegBase64()` — already handles JPEG conversion
- `OffscreenCanvas.drawImage()` 9-arg crop pattern — already used in tiling
- `scripting.executeScript()` with frame targeting — already proven

### Estimated complexity: Low-Moderate
All building blocks exist. Primary new code is ~50 lines in the handler for selector resolution, scroll/rect acquisition, DPR scaling, and crop.
