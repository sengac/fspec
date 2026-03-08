# Element-Targeted Screenshot Research

## Date: 2026-03-08
## Work Unit: EXT-014

---

## 1. Current Screenshot Scope

### Extension (`browser_screenshot` in `browser-tools.ts`, lines 340–425)

The current handler accepts exactly **one** targeting parameter:

- **`tabId`** (optional) — which tab to capture. Defaults to the active tab.

The actual capture call is:

```typescript
const dataUrl = await tabs.captureVisibleTab(windowId, { format: 'png' });
```

`chrome.tabs.captureVisibleTab()` captures **exactly the visible viewport** of the tab's window. There is currently:

- ❌ No element/selector targeting
- ❌ No clip region (x, y, width, height)
- ❌ No scroll-to-element-then-capture
- ❌ No full-page capture via this API (full-page is CDP-only, used in the Rust headless path)

All downstream processing (resize to 1568px long edge, PNG→JPEG conversion at 80% quality, vertical tiling for >800KB) operates on the raw viewport capture.

### MCP Tool Schema (what the agent sees)

The `browser_screenshot` tool in the extension MCP server currently exposes no parameters for element targeting. The only parameter is `tabId`.

---

## 2. Chromium Internals — `captureVisibleTab` Has Hidden `rect` Support

### CRITICAL FINDING: Undocumented `rect` and `scale` parameters already exist

The Chromium source reveals that `ImageDetails` (the options object passed to `captureVisibleTab`) already supports `rect` and `scale` properties — they're just hidden with `"nodoc": true`.

#### Source: `extensions/common/api/extension_types.json`
**URL:** https://chromium.googlesource.com/chromium/src/+/refs/heads/main/extensions/common/api/extension_types.json

```json
{
  "id": "Rect",
  "type": "object",
  "description": "An object specifying the area of the document to capture, in CSS pixels, relative to the page. All properties default to 0.",
  "properties": {
    "x":      { "type": "integer", "minimum": 0 },
    "y":      { "type": "integer", "minimum": 0 },
    "width":  { "type": "integer", "minimum": 0 },
    "height": { "type": "integer", "minimum": 0 }
  },
  "nodoc": true
},
{
  "id": "ImageDetails",
  "type": "object",
  "properties": {
    "format":  { "$ref": "ImageFormat", "optional": true },
    "quality": { "type": "integer", "optional": true, "minimum": 0, "maximum": 100 },
    "rect":    { "$ref": "Rect", "optional": true, "nodoc": true },
    "scale":   { "type": "number", "optional": true, "minimum": 0, "nodoc": true }
  }
}
```

**Key:** `"nodoc": true` means the property is omitted from public documentation but IS present in the generated types and IS handled by the C++ implementation. It's an undocumented-but-functional API surface.

#### Source: `extensions/browser/api/web_contents_capture_client.cc`
**URL:** https://chromium.googlesource.com/chromium/src/+/refs/heads/main/extensions/browser/api/web_contents_capture_client.cc

The C++ implementation that processes `captureVisibleTab` calls already handles the `rect` parameter:

```cpp
WebContentsCaptureClient::CaptureResult WebContentsCaptureClient::CaptureAsync(
    WebContents* web_contents,
    const ImageDetails* image_details,
    base::OnceCallback<void(const SkBitmap&)> callback) {
  // ...
  gfx::Rect source_rect;

  if (image_details) {
    // ...
    // If `rect` parameter is set, use it to get the correct region to capture.
    if (image_details->rect) {
      const auto& rect = *image_details->rect;
      source_rect.SetRect(rect.x, rect.y, rect.width, rect.height);
      float scale = image_details->scale ? *image_details->scale
                    : view->GetDeviceScaleFactor();
      source_rect = gfx::ScaleToEnclosingRect(source_rect, scale);
    }
  }

  view->CopyFromSurface(
      source_rect,  // An empty rect will capture the entire surface.
      gfx::Size(),
      base::TimeDelta(),
      // ...
  );
}
```

This means `CopyFromSurface` already supports a capture rect natively. When `source_rect` is empty (no `rect` parameter), it captures the entire visible surface. When populated, it captures only that region.

#### Source: `extensions/browser/api/web_contents_capture_client.h`
**URL:** https://chromium.googlesource.com/chromium/src/+/refs/heads/main/extensions/browser/api/web_contents_capture_client.h

The header shows this class is the shared base for both `tabs.captureVisibleTab` and `webview.captureVisibleRegion`:

```cpp
// Base class for capturing visible area of a WebContents.
// This is used by both webview.captureVisibleRegion and tabs.captureVisibleTab.
class WebContentsCaptureClient {
  // ...
  CaptureResult CaptureAsync(
      content::WebContents* web_contents,
      const api::extension_types::ImageDetails* image_detail,
      base::OnceCallback<void(const SkBitmap&)> callback);
};
```

### Chromium Bug Tracker Confirms This

**Chromium Issue #423658618:** "Feature request — extend `chrome.tabs.captureVisibleTab` API with `rect` parameter"
**URL:** https://issues.chromium.org/issues/423658618

A Chromium team member responded:
> "This seems like a very reasonable feature request. The underlying method we use, CopyFromSurface, even supports a capture rect."

**W3C WebExtensions Proposal #850:** "Extend `chrome.tabs.captureVisibleTab()` API with rect parameter"
**URL:** https://github.com/w3c/webextensions/issues/850

This is an active proposal from June 2025 to make the `rect` parameter officially public.

### Risk Assessment: Using the Undocumented `rect` Parameter

| Factor | Assessment |
|--------|------------|
| **C++ implementation exists** | ✅ Yes — fully implemented in `web_contents_capture_client.cc` |
| **Type definition exists** | ✅ Yes — `Rect` type and `ImageDetails.rect` defined in `extension_types.json` |
| **Publicly documented** | ❌ No — `nodoc: true` means it's hidden from Chrome DevDocs |
| **Stability risk** | ⚠️ Medium — undocumented APIs can change without notice |
| **Active proposal to make public** | ✅ Yes — W3C WebExtensions #850, Chromium #423658618 |
| **TypeScript types include it** | ❓ Unknown — `@anthropic-ai/claude-code` chrome types may not include it |

### Recommendation

**Do NOT rely on the undocumented `rect` parameter.** Instead, use the reliable client-side crop approach (capture viewport → crop with OffscreenCanvas), which is:
- Guaranteed to work on all Chrome versions
- Independent of undocumented API surfaces
- Already proven by the existing tiling code

---

## 3. Feasible Implementation: Client-Side Element Crop

### Approach

1. **Accept a `selector` parameter** (CSS selector or `@ref` from `browser_scan_page`)
2. **`executeScript`** to get the element's bounding rect and scroll it into view
3. **`captureVisibleTab`** to grab the viewport (exactly as today)
4. **Crop on OffscreenCanvas** using the bounding rect coordinates
5. Apply existing JPEG conversion and size management

### Existing Infrastructure That Supports This

| Capability | Where | Status |
|-----------|-------|--------|
| `@ref` resolution | `resolveRefSelector()` in `browser-tools.ts` (line 558) | ✅ Ready |
| `executeScript` to page | Used by `browser_click_element`, `browser_fill_form`, etc. | ✅ Ready |
| `getBoundingClientRect()` | Already used by `scan-page-dom.ts` (line 296, 413) and `dom-scanner-helpers.ts` (line 172) | ✅ Ready |
| OffscreenCanvas crop | Tiling code at line 406 uses `drawImage(source, sx, sy, sw, sh, dx, dy, dw, dh)` | ✅ Ready |
| JPEG conversion | `canvasToJpegBase64()` helper at line 126 | ✅ Ready |
| Frame-aware script injection | `resolveRefSelector()` returns `frameId`, click/fill use `{ tabId, frameIds: [frameId] }` | ✅ Ready |

### Implementation Flow

```
Agent calls: browser_screenshot({ selector: "@e5" })
                    │
                    ▼
  ┌──────────────────────────────┐
  │ 1. resolveRefSelector("@e5") │  → CSS selector + frameId
  └──────────────────┬───────────┘
                     │
                     ▼
  ┌──────────────────────────────────────────────────────┐
  │ 2. executeScript in correct frame:                   │
  │    - el.scrollIntoView({ block: 'center' })          │
  │    - small delay for scroll/render                   │
  │    - return el.getBoundingClientRect()                │
  │    → { x, y, width, height } relative to viewport   │
  └──────────────────┬───────────────────────────────────┘
                     │
                     ▼
  ┌──────────────────────────────────────────────────────┐
  │ 3. captureVisibleTab(windowId, { format: 'png' })    │
  │    → full viewport data URL (unchanged)              │
  └──────────────────┬───────────────────────────────────┘
                     │
                     ▼
  ┌──────────────────────────────────────────────────────┐
  │ 4. Decode to ImageBitmap, create OffscreenCanvas     │
  │    sized to element rect, drawImage with crop params │
  │    → cropped element image                           │
  └──────────────────┬───────────────────────────────────┘
                     │
                     ▼
  ┌──────────────────────────────────────────────────────┐
  │ 5. Apply existing pipeline:                          │
  │    - Resize if > 1568px long edge                    │
  │    - JPEG @ 80% quality                              │
  │    - Tile if > 800KB                                 │
  │    → MCP image content block(s)                      │
  └──────────────────────────────────────────────────────┘
```

### Edge Cases to Handle

| Edge Case | Handling |
|-----------|----------|
| Element is off-screen / not in viewport | `scrollIntoView()` first, then capture |
| Element larger than viewport | Capture what's visible, or multiple captures with scrolling |
| Element in iframe | Use `frameId` from ref resolution for `executeScript` target |
| Element partially obscured by other elements | Accept this — we capture the rendered pixel output |
| Element has zero dimensions | Return error: "Element has no visible dimensions" |
| `selector` not found in DOM | Return error: "Element not found: {selector}" |
| DPR scaling (Retina displays) | `getBoundingClientRect()` returns CSS pixels; `captureVisibleTab` returns device pixels. Must multiply rect by `window.devicePixelRatio` |
| Selector resolves to multiple elements | Use first match (consistent with `browser_click_element`) |

### DPR Scaling Detail

`captureVisibleTab` returns a PNG at **device pixel** resolution (2x on Retina). `getBoundingClientRect()` returns coordinates in **CSS pixels**. The crop must account for this:

```typescript
// In executeScript:
const rect = el.getBoundingClientRect();
const dpr = window.devicePixelRatio;
return {
  x: Math.round(rect.x * dpr),
  y: Math.round(rect.y * dpr),
  width: Math.round(rect.width * dpr),
  height: Math.round(rect.height * dpr),
  dpr,
};
```

### Proposed Tool Schema Addition

```typescript
// browser_screenshot gains an optional `selector` parameter:
browser_screenshot({
  tabId?: number,       // existing — which tab (default: active)
  selector?: string,    // NEW — CSS selector or @ref (e.g., "@e5", "#main-content", ".hero-image")
})
```

When `selector` is omitted, behaviour is identical to today (full viewport capture). When provided, the element is scrolled into view and cropped from the viewport capture.

---

## 4. Alternative Approaches Considered

### 4A: Use Chrome's undocumented `rect` parameter in `captureVisibleTab`

```typescript
// RISKY — undocumented API
const dataUrl = await tabs.captureVisibleTab(windowId, {
  format: 'png',
  rect: { x: 100, y: 200, width: 300, height: 150 },
});
```

**Rejected:** While the C++ implementation handles this, the `nodoc: true` flag means it could be removed without notice. TypeScript types may not include it. Not worth the stability risk when client-side crop is trivial.

### 4B: Use `chrome.debugger` + CDP `Page.captureScreenshot` with `clip`

CDP's `Page.captureScreenshot` has a documented `clip` parameter with `{ x, y, width, height, scale }`. However:
- Requires `chrome.debugger` permission
- Shows a "Chrome is being debugged" banner
- Much more complex permission model
- Overkill when we already have `captureVisibleTab` + OffscreenCanvas

**Rejected:** UX degradation from debugger banner.

### 4C: Use `html2canvas` or similar library

Render-to-canvas libraries can capture specific elements, but:
- Don't capture actual rendered pixels (re-render the DOM)
- Miss CSS effects, custom fonts, canvas/WebGL content
- Large library dependency
- Won't match what the user actually sees

**Rejected:** Doesn't capture actual screen content.

---

## 5. Estimated Scope

| Component | Effort |
|-----------|--------|
| Add `selector` param handling to `browser_screenshot` | Small |
| `executeScript` for `scrollIntoView` + `getBoundingClientRect` | Small |
| DPR-aware crop on OffscreenCanvas | Small — reuses existing `drawImage` pattern |
| Frame-aware execution (iframe support) | Small — reuses `resolveRefSelector` |
| MCP tool schema update (add `selector` to tool definition) | Trivial |
| Tests for element screenshot scenarios | Medium |
| Edge case handling (zero-size, off-screen, not found) | Small |
| **Total** | **~3–5 story points** |

---

## 6. Source References

| File | Lines | What |
|------|-------|------|
| `extension/src/background/browser-tools.ts` | 340–425 | Current `browser_screenshot` handler |
| `extension/src/background/browser-tools.ts` | 558–573 | `resolveRefSelector()` — CSS/ref resolution |
| `extension/src/background/browser-tools.ts` | 126–137 | `canvasToJpegBase64()` helper |
| `extension/src/background/browser-tools.ts` | 406 | `drawImage` with crop params (tiling code) |
| `extension/src/background/browser-tools.ts` | 89 | `createBrowserTools` factory |
| `extension/src/background/browser-tools-types.ts` | 1–153 | Type definitions and MCP content types |
| `extension/src/background/scan-page-dom.ts` | 296, 413 | `getBoundingClientRect()` usage |
| `extension/src/background/dom-scanner-helpers.ts` | 172 | `getBoundingClientRect()` usage |
| Chromium `extension_types.json` | — | `Rect` type + `ImageDetails.rect` (nodoc) |
| Chromium `web_contents_capture_client.cc` | — | `CopyFromSurface` with `source_rect` |
| Chromium `web_contents_capture_client.h` | — | Base class for captureVisibleTab |
| Chromium Issue #423658618 | — | Feature request to make `rect` public |
| W3C WebExtensions Issue #850 | — | Proposal to standardise `rect` |
