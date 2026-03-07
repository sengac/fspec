# AST Research: DOM Scanner Codebase Analysis for LOCATE-007

## Files Analyzed

### 1. extension/src/background/dom-scanner-helpers.ts (246 lines)
**Current interactivity detection and helper functions.**

Key exports:
- `isDynamicId(id: string): boolean` — Framework ID pattern detection
- `generateSelector(el: Element): string` — CSS selector generation (data-testid > id > attrs > nth-child)
- `isVisible(el: Element): boolean` — Visibility via checkVisibility() + getBoundingClientRect()
- `isInteractiveElement(el: Element): boolean` — Combined selector + cursor:pointer + onclick heuristics
- `shouldClaimChildren(el: Element): boolean` — Simple tag/role check (a[href], button, role=button/link)
- `getRelevantAttributes(el: Element): Record<string, string>` — Validation attributes extraction

Current `isInteractiveElement()` already handles:
- aria-disabled="true" → return false
- el.closest('[aria-hidden="true"]') → return false
- pointer-events: none → return false
- INTERACTABLE_SELECTOR match → true
- cursor:pointer → true
- onclick/onmousedown/onkeydown → true

**Missing from LOCATE-007 scope:**
- NO `inert` attribute check
- NO label wrapper detection (hasFormControlDescendant)
- NO search element heuristic
- NO icon-size heuristic
- NO dynamic class filtering

Current `shouldClaimChildren()` is simple tag-based — needs replacement with bounding box propagation.

### 2. extension/src/background/scan-page-dom.ts (359 lines)
**Injected scanning function — fully self-contained, no imports.**

Key function: `scanPageDOM(interactiveMode: boolean, scope?: string): ScanResult`

Inlined duplicates of all helpers from dom-scanner-helpers.ts:
- `isDynId()`, `getRole()`, `getName()`, `isVis()`, `isInteractive()`, `claimsChildren()`, `getAttrs()`, `genSelector()`, `depthOf()`

Two-pass TreeWalker approach:
1. First pass: Find claiming parents, mark all descendants as claimed
2. Second pass: Build element list, skip claimed elements

**Areas to modify:**
- Add `hasFormControl()` inlined function for label wrapper detection
- Add `isSearchEl()` inlined function for search detection
- Add `isIconSize()` inlined function for icon detection
- Add `inert` check to `isInteractive()`
- Add `filterDynClasses()` for stable hashing
- Replace `claimsChildren()` with bounding box propagation post-pass
- Add compound input type check

### 3. extension/src/background/dom-scanner.ts (195 lines)
**Pure helper functions + tree formatting.**

Re-exports from dom-scanner-helpers.ts. Contains:
- `getImplicitRole()` — Role mapping
- `getAccessibleName()` — Name extraction priority chain
- `formatAccessibilityTree()` — Tree text formatting

No changes needed for LOCATE-007.

### 4. extension/src/background/browser-tools.ts (578 lines)
**Tool handler with browser_scan_page handler.**

The `browser_scan_page` handler:
- Injects `scanPageDOM` via `chrome.scripting.executeScript()`
- Assigns refs (e1, e2, ...) to interactive elements
- Stores refs via `setTabScanState()`
- Formats tree via `formatAccessibilityTree()`

No changes needed — ref assignment happens after scan returns.

### 5. extension/src/background/__tests__/dom-scanning.test.ts (563 lines)
**Existing test file for LOCATE-004.**

Tests all 13 scenarios from dom-scanning.feature.
Uses jsdom with real `scanPageDOM` calls.
Mock factory pattern for `BrowserToolsDeps`.

New tests for LOCATE-007 should go in a separate test file to maintain < 300 line limit.

## Impact Analysis

| File | Lines | Changes Needed |
|------|-------|---------------|
| dom-scanner-helpers.ts | 246 | Add inert check, export new heuristic functions (may exceed 300 → extract to new file) |
| scan-page-dom.ts | 359 | Add inlined versions of all new heuristics, replace claimsChildren with bbox propagation |
| dom-scanner.ts | 195 | Re-export new functions from heuristics file |
| browser-tools.ts | 578 | No changes |
| NEW: dom-scanner-heuristics.ts | ~150 | New file for exported heuristic functions |
| NEW: interactivity-heuristics.test.ts | ~300 | New test file |

## Key Design Decision

`dom-scanner-helpers.ts` is at 246 lines. Adding 5+ new functions would push it over 300.
**Solution:** Create `dom-scanner-heuristics.ts` for the new exported functions, re-export from `dom-scanner.ts`.
