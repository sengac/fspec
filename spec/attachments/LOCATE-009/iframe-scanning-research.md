# Iframe Scanning Research — Chrome Extension Context

## Problem Statement

The current `browser_scan_page` implementation injects `scanPageDOM` via `chrome.scripting.executeScript()` targeting only the top-level frame (frameId 0). Elements inside `<iframe>` elements are completely invisible to the AI agent — it cannot see, click, or fill form fields within iframes.

This is a significant gap because iframes are extremely common:
- **Payment forms** (Stripe, PayPal, Braintree) embed checkout in cross-origin iframes
- **OAuth/login flows** (Google Sign-In, Auth0, Okta) use iframe-based auth widgets
- **Embedded content** (YouTube, maps, social media embeds)
- **Advertising** (ad frames)
- **CMS editors** (TinyMCE, CKEditor use iframes for the editing surface)
- **Captchas** (reCAPTCHA, hCaptcha)

## How agent-browser Handles Iframes (Comparison)

agent-browser does **NOT** automatically scan iframes. They provide manual frame switching:

```typescript
// Switch to iframe by CSS selector, name, or URL
handleFrame(command: FrameCommand, browser: BrowserManager)
  → browser.switchToFrame({ selector: '#iframe' })
  → await frameElement.contentFrame()  // Playwright API

// Switch back
handleMainFrame() → browser.switchToMainFrame()
```

This means the AI must:
1. Know an iframe exists
2. Explicitly switch to it
3. Run snapshot/actions within that frame
4. Switch back

This is a poor UX for AI agents — they need to discover frames exist first, which requires scanning.

Their `snapshot.ts` uses `locator.ariaSnapshot()` which is Playwright's built-in and operates on the current active frame only.

## Chrome Extension APIs Available

### 1. `chrome.webNavigation.getAllFrames()`

Discovers all frames in a tab:

```typescript
const frames = await chrome.webNavigation.getAllFrames({ tabId });
// Returns: Array<FrameInfo> — one entry per frame in the tab
// frameId 0 = main frame
// frameId > 0 = subframes (iframes)
```

**Requires**: `"webNavigation"` permission in manifest.json.

Each frame object includes (per Chromium IDL `web_navigation.json`):
- `frameId` (integer) — 0 for main frame, positive for subframes
- `parentFrameId` (integer) — -1 for main frame, parent's frameId for subframes
- `url` (string) — the frame's URL
- `processId` (integer) — renderer process ID
- `documentId` (string) — UUID of the loaded document (**key for correlation**)
- `parentDocumentId` (string, optional) — UUID of the parent document
- `documentLifecycle` (enum) — `"prerender"`, `"active"`, `"cached"`, `"pending_deletion"`
- `frameType` (enum) — `"outermost_frame"`, `"fenced_frame"`, `"sub_frame"`
- `errorOccurred` (boolean) — whether the last navigation errored

### 2. `chrome.scripting.executeScript()` with `frameIds`

Can target specific frames:

```typescript
// Scan a specific iframe
const results = await chrome.scripting.executeScript({
  target: { tabId, frameIds: [frameId] },
  func: scanPageDOM,
  args: [true, undefined],
});
```

**Key insight**: This works for both same-origin AND cross-origin iframes, because `chrome.scripting.executeScript` checks host permissions (`<all_urls>` in our case). The extension can inject into any frame regardless of origin.

**InjectionResult type** (per Chromium IDL `scripting.idl`):
```typescript
interface InjectionResult {
  result: any;       // The return value from the injected function
  frameId: number;   // Which frame produced this result
  documentId: string; // UUID matching getAllFrames().documentId
}
```

The `documentId` field enables precise correlation between `executeScript` results and `getAllFrames()` frame metadata.

### 3. `chrome.scripting.executeScript()` with `allFrames: true`

Injects into ALL frames at once:

```typescript
const results = await chrome.scripting.executeScript({
  target: { tabId, allFrames: true },
  func: scanPageDOM,
  args: [true, undefined],
});
// Returns: Array<InjectionResult> — one per frame, each with frameId + documentId + result
```

**Important**: The main frame is guaranteed to be the first element in the results array. All other frames are in **non-deterministic order** — always use `frameId`/`documentId` to identify results, never array position.

### 4. Chrome 133+ Behavior Change (match_origin_as_fallback)

Since Chrome 133 (Jan 2025), `chrome.scripting.executeScript` uses `match_origin_as_fallback` by default. This means it now injects into MORE frames automatically, including `about:blank` and sandboxed `srcdoc` frames that previously required explicit `match_origin_as_fallback`. This improves iframe scanning coverage.

Source: [Chrome Extensions DevRel PSA](https://groups.google.com/a/chromium.org/g/chromium-extensions/c/D8DcJARVM90)

## Proposed Architecture

### Approach: Multi-Frame Scan with Frame-Prefixed Refs

#### Phase 1: Frame Discovery

```typescript
// Add "webNavigation" to manifest.json permissions
const frames = await chrome.webNavigation.getAllFrames({ tabId });
// Filter to scannable frames
const scannable = frames.filter(f =>
  f.documentLifecycle === 'active' && (
    f.url.startsWith('http://') ||
    f.url.startsWith('https://') ||
    f.url === 'about:blank' ||  // may have JS-populated content
    f.url === 'about:srcdoc'    // inline HTML content
  )
);
```

#### Phase 2: Frame-to-DOM Correlation (Two-Pass Injection)

**The problem**: `getAllFrames()` returns frameIds but there's no Chrome API to directly map a frameId to its `<iframe>` DOM element in the parent. We need this mapping to nest iframe content under the correct `<iframe>` element in the accessibility tree.

**Solution — two-pass injection**:

**Pass 1: Inject frameId markers into each frame**
```typescript
// Inject into all subframes simultaneously
await chrome.scripting.executeScript({
  target: { tabId, frameIds: scannable.filter(f => f.frameId !== 0).map(f => f.frameId) },
  func: (fid: number) => { (window as any).__fspec_frameId = fid; },
  args: [/* each frame gets its own frameId via separate call */],
});
// Note: since args are shared across all frameIds, we need individual calls
// per frame to pass the correct frameId:
for (const frame of subframes) {
  await chrome.scripting.executeScript({
    target: { tabId, frameIds: [frame.frameId] },
    func: (fid: number) => { (window as any).__fspec_frameId = fid; },
    args: [frame.frameId],
  });
}
```

**Pass 2: Parent frame scan reads markers**
```typescript
// Inside scanPageDOM running in the parent frame:
const iframes = document.querySelectorAll('iframe');
for (const iframe of iframes) {
  let frameId: number | null = null;
  try {
    // Same-origin: direct read from contentWindow
    frameId = (iframe.contentWindow as any)?.__fspec_frameId ?? null;
  } catch (e) {
    // Cross-origin: SOP blocks contentWindow access
    // Fall back to URL matching against getAllFrames() data
    frameId = null; // service worker matches by iframe.src vs frame.url
  }
  // Record: { iframeIndex, src: iframe.src, name: iframe.name, frameId }
}
```

For cross-origin iframes where `contentWindow` is blocked, the service worker matches by comparing `iframe.src` from the parent scan against `frame.url` from `getAllFrames()`. Multiple cross-origin iframes with identical URLs are rare; document order is the tiebreaker.

#### Phase 3: Multi-Frame Scanning

**Strategy A — Single batch call** (preferred when possible):
```typescript
const results = await chrome.scripting.executeScript({
  target: { tabId, frameIds: scannable.map(f => f.frameId) },
  func: scanPageDOM,
  args: [true, undefined],
});
// Each result has frameId + documentId for correlation
```

This returns one result per frame. Each result's `frameId` identifies which frame it came from. Frames that fail (e.g., chrome-extension:// URLs) are silently excluded from results.

**Strategy B — Individual calls per frame** (more control, needed for per-frame args):
```typescript
const results = await Promise.all(
  scannable.map(f =>
    chrome.scripting.executeScript({
      target: { tabId, frameIds: [f.frameId] },
      func: scanPageDOM,
      args: [true, undefined],
    }).catch(() => null) // gracefully skip failed frames
  )
);
```

#### Phase 4: Ref Namespace

Refs must encode the frame to enable click/fill targeting:

```
Main frame refs:  e1, e2, e3        (backward compatible — no prefix)
Frame 5 refs:     f5e1, f5e2, f5e3  (frame 5, element 1/2/3)
Frame 12 refs:    f12e1, f12e2      (frame 12, element 1/2)
```

The `@` prefix still works: `@f5e3` → frame 5, element 3.

`RefEntry` needs a new field:
```typescript
interface RefEntry {
  selector: string;
  role: string;
  name: string;
  frameId: number;  // NEW — 0 for main frame
}
```

#### Phase 5: Tree Output

Iframe content appears nested under the iframe element:

```
- heading "My Page" [level=1]
- textbox "Email" [ref=e1]
- iframe "Payment Form" [src=https://stripe.com/...]
  - textbox "Card Number" [ref=f5e1]
  - textbox "Expiry" [ref=f5e2]
  - textbox "CVC" [ref=f5e3]
  - button "Pay" [ref=f5e4]
- button "Continue" [ref=e2]
```

The top-level scanner detects `<iframe>` elements and records their position in the element list. The service worker then splices per-frame scan results at the correct tree positions using the frame-to-DOM correlation from Phase 2.

#### Phase 6: Click/Fill with Frame-Aware Refs

The `resolveRefSelector` function parses the ref to extract frameId:

```typescript
function parseRef(ref: string): { frameId: number; elementRef: string } | null {
  // "e5"    → { frameId: 0, elementRef: "e5" }
  // "f5e3"  → { frameId: 5, elementRef: "e3" }
  const frameMatch = /^f(\d+)e(\d+)$/.exec(ref);
  if (frameMatch) {
    return { frameId: parseInt(frameMatch[1]), elementRef: `e${frameMatch[2]}` };
  }
  const mainMatch = /^e(\d+)$/.exec(ref);
  if (mainMatch) {
    return { frameId: 0, elementRef: ref };
  }
  return null;
}
```

Then `executeScript` targets the correct frame:

```typescript
await chrome.scripting.executeScript({
  target: { tabId, frameIds: [parsed.frameId] },
  func: (sel) => { document.querySelector(sel)?.click(); },
  args: [entry.selector],
});
```

#### Phase 7: Diff Integration

`browser_diff_page` stores tree text per frame. Diff operates on the merged tree (same as scan output). Frame additions/removals (e.g., iframe dynamically added/removed) show up naturally.

## Edge Cases and Limitations

### Cross-Origin Iframes
✅ **Fully supported** — `chrome.scripting.executeScript` with `<all_urls>` host permission can inject into any frame regardless of origin. This is the key advantage of the Chrome extension approach over page-level JavaScript. Verified in Chromium source: `scripting_utils.cc` checks `HasPermissionToInjectIntoFrame()` using `CanAccessPage()` with the frame's effective URL.

### Sandboxed Iframes
✅ **Fully supported** in ISOLATED world — The `sandbox` attribute without `allow-scripts` only blocks scripts in the **MAIN** world. Extension content scripts running in the **ISOLATED** world bypass this restriction. Confirmed by:
- Chromium bug [355256366](https://issues.chromium.org/issues/355256366) (regression in Chrome 127 for srcdoc+sandbox, fixed Chrome 130)
- The [SO question](https://stackoverflow.com/questions/77775156) that reported blocking used `world: "MAIN"` explicitly
- `executeScript` defaults to `ISOLATED` world (per `scripting.idl`: `ExecutionWorld? world;` defaults to ISOLATED)

**Important**: Our `scanPageDOM` injection uses `chrome.scripting.executeScript` which runs in ISOLATED world by default. No sandbox issues.

### Dynamically Added Iframes
Iframes added after initial page load (e.g., payment modals that lazy-load) won't appear in the initial scan. A re-scan picks them up. This is consistent with the current ephemeral ref model.

### Nested Iframes (Iframe in Iframe)
`getAllFrames()` returns ALL frames at all nesting levels, each with `parentFrameId`. The tree builder reconstructs nesting using the `parentFrameId` chain. Each frame's refs use its direct frameId (f12e1), not the parent chain.

### about:blank and about:srcdoc Iframes
- `about:blank` iframes report `url: "about:blank"` — scan if they have content (JS-populated same-origin frames)
- `srcdoc` iframes report `url: "about:srcdoc"` (**NOT** `about:blank`) — always scan (inline HTML)
- Chrome 133+ injects into both by default via `match_origin_as_fallback`

### Frame Count Explosion
Some pages (especially ad-heavy) can have 20+ iframes. A `maxFrames` parameter (default: 10) should limit scanning to prevent timeout. Frames can be prioritized: same-origin first, then cross-origin by size (larger iframes are more likely to contain interactive content).

## Required Changes Summary

| File | Change |
|------|--------|
| `manifest.json` | Add `"webNavigation"` to permissions |
| `browser-tools-types.ts` | Update `BrowserToolsDeps` to include `webNavigation` |
| `ref-state.ts` | Add `frameId` to `RefEntry`; update `TabScanState` to hold per-frame data |
| `browser-tools.ts` | Update `executeScanAndStore` for multi-frame; update `resolveRefSelector` for frame-prefixed refs; update click/fill executeScript to target specific frameIds |
| `dom-scanner.ts` | Update `formatAccessibilityTree` to handle frame nesting |
| `scan-page-dom.ts` | Add iframe detection (record iframe positions in element list) |
| `webmcp-skill.md` | Document frame-aware refs (f5e3 syntax) |
| `mcp-server.mjs` | No changes needed (inputSchema unchanged) |

## Chromium Source References

All API claims verified against Chromium source (March 2026):
- **getAllFrames IDL**: `chrome/common/extensions/api/web_navigation.json` — confirmed all return fields
- **executeScript IDL**: `chrome/common/extensions/api/scripting.idl` — confirmed `InjectionResult { result, frameId, documentId }`
- **InjectionTarget**: `scripting.idl` — confirmed `frameIds: long[]?`, `allFrames: boolean?`, `documentIds: DOMString[]?`
- **Permission checking**: `extensions/browser/scripting_utils.cc` `HasPermissionToInjectIntoFrame()` — uses `CanAccessPage()` with effective URL
- **Frame injection**: `extensions/browser/script_executor.cc` Handler — dispatches to all frames, collects results per-frame
- **Sandbox behavior**: ISOLATED world scripts bypass `sandbox` without `allow-scripts` — confirmed by bug 355256366 and Blink execution path

## Performance Considerations

- `getAllFrames()` is fast (~1ms)
- Per-frame `executeScript` calls are dispatched in parallel by Chrome when using `frameIds: [f1, f2, f3]`
- For a typical page with 3-5 iframes, total scan time should be ~50-100ms (vs ~30ms for single frame)
- The `maxFrames` parameter prevents pathological cases
- Results come back in **non-deterministic order** (except main frame is always first) — use `frameId` to identify
