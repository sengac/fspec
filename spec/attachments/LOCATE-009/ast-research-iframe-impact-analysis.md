# AST Research: Iframe-Aware DOM Scanning Impact Analysis

## Files Requiring Changes

### 1. ref-state.ts — RefEntry & TabScanState
**Location:** `extension/src/background/ref-state.ts`

```
RefEntry interface at line 16:
  Current fields: selector, role, name
  Required change: Add `frameId: number` field (0 for main frame)

TabScanState interface at line 26:
  Current fields: refs (Map<string, RefEntry>), treeText, timestamp
  No structural change needed — frame-prefixed ref keys (f5e1) naturally coexist with main frame keys (e1)
```

`resolveRef` function needs no change — Map lookup with frame-prefixed keys works transparently.

### 2. browser-tools-types.ts — BrowserToolsDeps
**Location:** `extension/src/background/browser-tools-types.ts:83`

```
BrowserToolsDeps interface:
  Current: { tabs, scripting, windows, userScripts? }
  Required change: Add `webNavigation: ChromeWebNavigationForTools` field
```

New interface needed:
```typescript
interface ChromeWebNavigationForTools {
  getAllFrames: (details: { tabId: number }) => Promise<FrameInfo[] | null>;
}

interface FrameInfo {
  frameId: number;
  parentFrameId: number;
  url: string;
  documentId: string;
  documentLifecycle: string;
  frameType: string;
  errorOccurred: boolean;
}
```

### 3. browser-tools.ts — Handler Changes
**Location:** `extension/src/background/browser-tools.ts`

**14 handlers registered (AST grep: `handlers.set` calls):**
- browser_navigate (L194) — no change
- browser_screenshot (L209) — no change
- browser_list_tabs (L224) — no change
- browser_execute_script (L248) — no change
- browser_switch_tab (L292) — no change
- browser_close_tab (L311) — no change
- browser_get_page_content (L327) — no change
- **browser_click_element (L372) — CHANGE: resolveRefSelector must parse f{frameId}e{N} refs**
- **browser_fill_form (L406) — CHANGE: resolveRefSelector must parse f{frameId}e{N} refs**
- browser_go_back (L446) — no change
- browser_go_forward (L453) — no change
- browser_create_tab (L460) — no change
- **browser_scan_page (L499) — CHANGE: multi-frame scanning orchestration**
- **browser_diff_page (L526) — CHANGE: diff on merged multi-frame tree**

**`executeScanAndStore` function (L156-191):**
- Currently scans only top-level frame: `scripting.executeScript({ target: { tabId }, ... })`
- Must be refactored to: (1) discover frames via getAllFrames, (2) inject markers per frame, (3) scan all frames, (4) merge results
- Ref assignment at L174-182 needs frame-prefixed logic: main frame = `e{N}`, iframe = `f{frameId}e{N}`

**`resolveRefSelector` function (L354-369):**
- Currently only strips `@` prefix and looks up in refs Map
- Must parse frame-prefixed refs: `@f5e3` → `{ frameId: 5, elementRef: 'e3' }`
- Must target correct frame when calling executeScript for click/fill

### 4. dom-scanner.ts — formatAccessibilityTree
**Location:** `extension/src/background/dom-scanner.ts:173`

```
formatAccessibilityTree function:
  Current: Takes flat RawElement[] array, formats indented tree
  Required change: Accept optional iframe nesting data to splice iframe subtrees at correct positions
```

RawElement interface at L16 may need an optional `iframeContent` field or the merge happens before calling formatAccessibilityTree.

### 5. scan-page-dom.ts — iframe element detection
**Location:** `extension/src/background/scan-page-dom.ts:40`

TreeWalker in scanPageDOM currently uses `FILTER_REJECT` for SCRIPT/STYLE/NOSCRIPT (L366-370). Iframe elements should NOT be rejected — they should be included in results so the service worker knows where to splice iframe subtrees.

The scanPageDOM function currently doesn't detect iframe elements at all. Needs:
- Detection of `<iframe>` elements during traversal
- Recording iframe position, src, name, and attempt to read `contentWindow.__fspec_frameId`
- Output these as special elements in the result array for merge

### 6. manifest.json — webNavigation permission
**Location:** `extension/manifest.json`

```
Current permissions: ["activeTab", "tabs", "scripting", "storage", "offscreen", "nativeMessaging", "userScripts"]
Required change: Add "webNavigation"
```

## Backward Compatibility Analysis

- Main frame refs (e1, e2, e3) remain unchanged — `f{frameId}e{N}` format only for non-main frames
- RefEntry gains `frameId` field — new field, no breakage
- Pages without iframes produce identical output — confirmed by example [1] in work unit

## Test Files to Update

```
extension/src/background/__tests__/ — existing scan tests need frame-aware scenarios
```
