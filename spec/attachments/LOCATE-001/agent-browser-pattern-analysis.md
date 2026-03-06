# Research: agent-browser Snapshot/Ref Pattern — Application to fspec Chrome Extension

## Date: 2026-03-06
## Work Unit: LOCATE-001
## Source: https://github.com/vercel-labs/agent-browser (cloned to /tmp/agent-browser)

---

## 1. What This Document Covers

This document analyses agent-browser's snapshot/ref system and maps exactly what needs to be implemented in the **fspec Chrome Extension** (`extension/` directory) to give AI agents structured page understanding. 

**Critical constraint:** The existing Rust `WebSearch` tool (`codelet/tools/src/web_search.rs`, `codelet/tools/src/chrome_browser.rs`, `codelet/tools/src/page_fetcher.rs`) is a completely separate system using rust-headless-chrome for headless browsing. It is **untouched** by this work. The scan/ref system is purely additive to the Chrome Extension's browser control tools.

---

## 2. agent-browser's Architecture (What We're Learning From)

### 2.1 The Snapshot/Ref Lifecycle

agent-browser's core insight: **AI agents should never construct CSS selectors**. Instead:

```
snapshot -i  →  ARIA accessibility tree with [ref=eN]  →  ref map stored in daemon memory
click @e3    →  lookup ref → getByRole('button', {name:"Submit"}) → Playwright executes
diff snapshot → new snapshot → Myers diff vs previous → +/- text changes
```

Refs are **ephemeral** — invalidated on any page navigation. The workflow is always: **scan → interact → re-scan**.

### 2.2 Key Source Files

| File | Lines | What It Does |
|------|-------|-------------|
| [`src/snapshot.ts`](https://github.com/vercel-labs/agent-browser/blob/main/src/snapshot.ts) | ~640 | Core snapshot engine: parses Playwright's `ariaSnapshot()`, adds `[ref=eN]` to interactive elements, stores ref→locator map, cursor-interactive detection, compact mode |
| [`src/diff.ts`](https://github.com/vercel-labs/agent-browser/blob/main/src/diff.ts) | ~340 | Myers diff algorithm for comparing snapshot text (line-level), pixel diff for screenshots |
| [`src/actions.ts`](https://github.com/vercel-labs/agent-browser/blob/main/src/actions.ts) | ~2800 | Command dispatcher: resolves `@e3` refs to Playwright locators before executing clicks/fills |
| [`src/browser.ts`](https://github.com/vercel-labs/agent-browser/blob/main/src/browser.ts) | ~2500 | BrowserManager: stores `lastSnapshot`/`lastRefs`, manages ref lifecycle, tab state |
| [`src/types.ts`](https://github.com/vercel-labs/agent-browser/blob/main/src/types.ts) | ~1290 | All command/response type definitions including SnapshotCommand, SnapshotData |

### 2.3 The Snapshot Output Format

```
- heading "Example Domain" [ref=e1] [level=1]
- paragraph: This domain is for use in illustrative examples.
- link "More information..." [ref=e2]
```

Interactive-only mode (`-i` flag) strips structural elements:
```
- link "More information..." [ref=e1]
```

### 2.4 The RefMap Data Structure

From `src/snapshot.ts:22-35`:
```typescript
export interface RefMap {
  [ref: string]: {
    selector: string;   // e.g. "getByRole('button', { name: \"Submit\", exact: true })"
    role: string;       // e.g. "button"
    name: string;       // e.g. "Submit"
    nth?: number;       // disambiguation index for duplicate role+name combos
  };
}
```

### 2.5 Role Classification

From `src/snapshot.ts:70-128`:
```typescript
// Get refs — these are what the AI interacts with
const INTERACTIVE_ROLES = new Set([
  'button', 'link', 'textbox', 'checkbox', 'radio', 'combobox', 'listbox',
  'menuitem', 'menuitemcheckbox', 'menuitemradio', 'option', 'searchbox',
  'slider', 'spinbutton', 'switch', 'tab', 'treeitem',
]);

// Get refs for text extraction (context)
const CONTENT_ROLES = new Set([
  'heading', 'cell', 'gridcell', 'columnheader', 'rowheader',
  'listitem', 'article', 'region', 'main', 'navigation',
]);

// Can be filtered in compact mode (noise reduction)
const STRUCTURAL_ROLES = new Set([
  'generic', 'group', 'list', 'table', 'row', 'rowgroup', 'grid',
  'treegrid', 'menu', 'menubar', 'toolbar', 'tablist', 'tree',
  'directory', 'document', 'application', 'presentation', 'none',
]);
```

### 2.6 Cursor-Interactive Detection

From `src/snapshot.ts:142-261` — finds clickable elements that lack proper ARIA roles:

```javascript
// Detects elements with:
// - cursor: pointer (but not inherited from parent)
// - onclick attribute
// - tabindex (not -1)
// Filters out: zero-size elements, elements with empty text,
//              elements that already have interactive ARIA roles
```

This is critical for modern SPAs where many interactive elements are plain `<div>` or `<span>` with click handlers.

### 2.7 Duplicate Role+Name Handling

From `src/snapshot.ts:340-383` — when multiple elements share the same role+name, agent-browser uses `nth` indices and shows `[nth=1]` in output. After processing, elements with unique role+name have their `nth` stripped for simplicity.

### 2.8 Myers Diff

From `src/diff.ts:17-158` — line-level diff producing unified format:

```
  - heading "Sign In" [level=1]
  - textbox "Email" [ref=e1]
+ - textbox "Password" [ref=e2]
- - button "Submit" [ref=e3]
+ - button "Signing in..." [ref=e3] [disabled]
```

Returns stats: `{ additions, removals, unchanged, changed: boolean }`

---

## 3. Critical Differences: Chrome Extension vs agent-browser

| Aspect | agent-browser | fspec Chrome Extension |
|--------|--------------|----------------------|
| **ARIA tree source** | Playwright's `ariaSnapshot()` API | Must build our own — no Playwright available |
| **Selector format** | Playwright locators: `getByRole('button', ...)` | CSS selectors: `#submit` or `button.login-btn` |
| **Interaction execution** | Playwright trusted events (dispatched at browser level) | `chrome.scripting.executeScript` → `element.click()` |
| **Browser instance** | Headless Playwright-controlled | User's real live browser |
| **Ref storage** | In-memory in Node.js daemon process | In service worker memory (scoped per tabId) |
| **Ref invalidation** | On navigation (tracked by daemon) | On `chrome.tabs.onUpdated` navigation events (already tracked by `browser-events.ts`) |
| **Extra capabilities** | None | WebMCP tool discovery, browser event notifications |
| **Script injection** | `page.evaluate()` in Playwright context | `chrome.scripting.executeScript()` in ISOLATED world |

### 3.1 Why We Can't Use Playwright's `ariaSnapshot()`

Playwright's `ariaSnapshot()` is an internal Playwright API that uses the browser's accessibility tree via CDP `Accessibility.getFullAXTree`. Our Chrome Extension doesn't have a Playwright `Page` object — we have `chrome.tabs` and `chrome.scripting`.

**Our approach:** Build an equivalent representation using standard DOM APIs available in the ISOLATED content script world:
- `document.createTreeWalker()` for O(n) traversal
- `element.checkVisibility()` for visibility filtering
- `element.getBoundingClientRect()` for zero-size detection
- `element.getAttribute('role')` / `element.getAttribute('aria-label')` for ARIA semantics
- `element.matches()` for interactability detection
- `getComputedStyle()` for cursor/pointer-events checks

All of these APIs are verified to work in Chrome's ISOLATED content script world (see `spec/attachments/EXT-012/dom-scanning-chromium-research.md`).

---

## 4. Exactly What Needs to Be Implemented

### 4.1 New Tool: `browser_scan_page`

**Purpose:** Scan the active tab's DOM for interactive elements, returning a structured tree with refs.

**Files to modify:**

| File | Change |
|------|--------|
| `extension/src/background/browser-tools.ts` | Add `browser_scan_page` handler |
| `extension/host/lib/mcp-server.mjs` | Add tool definition to `NATIVE_TOOLS` array |
| `extension/webmcp-skill.md` | Document the new tool |
| `extension/inject-webmcp-tools-skill.md` | Update if needed |

**Handler implementation in `browser-tools.ts`:**

The handler uses `chrome.scripting.executeScript()` to inject a scanning function into the ISOLATED world. The function:

1. Creates a `TreeWalker` rooted at `document.body` (with `NodeFilter.FILTER_REJECT` for script/style/noscript/svg)
2. For each element, checks interactability via `element.matches(INTERACTABLE_SELECTOR)` plus cursor/onclick/tabindex heuristics
3. For interactive elements, checks visibility via `element.checkVisibility({opacityProperty:true, visibilityProperty:true})` + `getBoundingClientRect()` zero-size check
4. Generates a unique CSS selector (data-testid > id > unique attribute combo > nth-child path)
5. Extracts: tag name, computed role (from `role` attribute or implicit HTML semantics), accessible name (from aria-label, textContent, placeholder, value, alt, title), element type classification
6. Returns a flat array of element descriptors + page metadata (url, title, viewport dimensions)

The service worker then:
1. Assigns refs (e1, e2, e3...) to each element
2. Stores the ref→selector map in memory, keyed by tabId
3. Stores the tree text for later diff comparison
4. Formats the output as an indented accessibility-tree-like string

**Injected scanning function (~200-300 lines JS):**

```javascript
// Key interactable selector set (from EXT-012 research)
const INTERACTABLE_SELECTOR = [
  'a[href]', 'button', 'input', 'textarea', 'select',
  '[role="button"]', '[role="link"]', '[role="checkbox"]',
  '[role="radio"]', '[role="tab"]', '[role="menuitem"]',
  '[role="option"]', '[role="switch"]', '[role="textbox"]',
  '[role="combobox"]', '[role="searchbox"]', '[role="slider"]',
  '[role="spinbutton"]', '[contenteditable="true"]',
  '[contenteditable=""]', '[tabindex]', 'summary', 'details',
].join(',');
```

**Output format (matching agent-browser style):**

```
Page: https://example.com/login — "Login - Example App"
Viewport: 1280x720 | Scroll: 0,0

- heading "Sign In" [level=1]
  - textbox "Email" [ref=e1]
  - textbox "Password" [ref=e2] [type=password]
  - checkbox "Remember me" [ref=e3]
  - button "Sign In" [ref=e4]
- region "Footer"
  - link "Forgot Password" [ref=e5]
  - link "Create Account" [ref=e6]
```

**Tool input schema:**

```json
{
  "type": "object",
  "properties": {
    "tabId": { "type": "number", "description": "Tab to scan (defaults to active tab)" },
    "interactive": { "type": "boolean", "description": "Only show interactive elements (default: true)" },
    "selector": { "type": "string", "description": "CSS selector to scope the scan to a subtree" }
  }
}
```

### 4.2 New Tool: `browser_diff_page`

**Purpose:** Show what changed on the page since the last `browser_scan_page` call.

**Files to modify:**

| File | Change |
|------|--------|
| `extension/src/background/browser-tools.ts` | Add `browser_diff_page` handler |
| `extension/host/lib/mcp-server.mjs` | Add tool definition to `NATIVE_TOOLS` array |
| `extension/webmcp-skill.md` | Document the new tool |

**Handler implementation:**

1. Runs the same scan function as `browser_scan_page`
2. Retrieves the previous scan tree text from service worker memory
3. Runs a line-level text diff (simplified Myers, or even simple line-by-line comparison)
4. Returns unified diff format with stats

**Output format:**

```
  - heading "Sign In" [level=1]
  - textbox "Email" [ref=e1]
  - textbox "Password" [ref=e2] [type=password]
- - button "Sign In" [ref=e3]
+ - button "Signing in..." [ref=e3] [disabled]

Changes: 1 addition, 1 removal, 3 unchanged
```

**Tool input schema:**

```json
{
  "type": "object",
  "properties": {
    "tabId": { "type": "number", "description": "Tab to diff (defaults to active tab)" }
  }
}
```

### 4.3 Ref Resolution in Existing Tools

**Purpose:** Let `browser_click_element` and `browser_fill_form` accept `@e1` syntax.

**Files to modify:**

| File | Change |
|------|--------|
| `extension/src/background/browser-tools.ts` | Add ref resolution at the top of `browser_click_element` and `browser_fill_form` handlers |

**Implementation:**

```typescript
// At the top of browser_click_element and browser_fill_form handlers:
let resolvedSelector = selector;
if (selector.startsWith('@')) {
  const ref = selector.slice(1); // e.g. "e3"
  const refEntry = refMaps.get(tabId)?.get(ref);
  if (!refEntry) {
    return errorResult(`Ref ${selector} not found. Run browser_scan_page first.`);
  }
  resolvedSelector = refEntry.selector;
}
```

**Backward compatibility:** If the selector does NOT start with `@`, it passes through as a raw CSS selector — existing behavior is completely preserved.

### 4.4 Ref Map Invalidation

**Purpose:** Clear stale refs when the page changes.

**Files to modify:**

| File | Change |
|------|--------|
| `extension/src/background/browser-events.ts` | Add ref map invalidation on navigation events |
| `extension/src/background/service-worker.ts` | Wire ref map to event listeners |

**Implementation:**

In `browser-events.ts`, when `changeInfo.url` fires (navigation), clear the ref map for that tabId. When `onRemoved` fires (tab closed), also clear.

This already partially exists — `browser-events.ts` lines 88-116 already fire notifications on navigation. We add ref map cleanup to the same callbacks.

### 4.5 Ref Map State Module (New File)

**New file:** `extension/src/background/ref-state.ts`

**Purpose:** Centralized ref map storage, shared between `browser-tools.ts` (write on scan, read on click/fill) and `browser-events.ts` (clear on navigation).

```typescript
interface RefEntry {
  selector: string;
  role: string;
  name: string;
}

interface TabScanState {
  refs: Map<string, RefEntry>;
  treeText: string;  // for diff comparison
  timestamp: number;
}

// Map<tabId, TabScanState>
const tabStates = new Map<number, TabScanState>();

export function setTabScanState(tabId: number, state: TabScanState): void { ... }
export function getTabScanState(tabId: number): TabScanState | undefined { ... }
export function clearTabScanState(tabId: number): void { ... }
export function resolveRef(tabId: number, ref: string): RefEntry | undefined { ... }
```

---

## 5. What We Borrow From agent-browser vs. What We Build Ourselves

### Borrow (design patterns):
- **Ref-based interaction model** — AI uses `@e1`, never constructs selectors
- **Ephemeral refs** — invalidated on navigation, re-scan required
- **Scan→Interact→Re-scan workflow** — proven AI agent pattern
- **Interactive-only mode** — only show elements the AI can interact with
- **Role classification** — interactive/content/structural role sets
- **Cursor-interactive detection** — find `cursor:pointer` + `onclick` elements without ARIA roles
- **Text diff for verification** — compare before/after to confirm action effects
- **Token stats** — report chars/tokens/ref count so AI knows the cost

### Build ourselves (because Chrome Extension ≠ Playwright):
- **DOM scanning function** — replaces Playwright's `ariaSnapshot()` with TreeWalker + ARIA heuristics
- **CSS selector generation** — replaces Playwright's `getByRole()` locators with CSS selectors
- **ISOLATED world injection** — `chrome.scripting.executeScript()` instead of `page.evaluate()`
- **Service worker ref storage** — in-memory Map instead of daemon-process state
- **Tab-scoped lifecycle** — ref maps keyed by tabId, cleared on `chrome.tabs.onUpdated`

### Explicitly NOT implementing (V1 scope):
- **Shadow DOM traversal** — complex, low ROI for most pages
- **Cross-navigation re-identification** — agent-browser doesn't do this either; refs are ephemeral
- **Annotated screenshots** — agent-browser's `--annotate` mode is nice but not essential for V1
- **Compact mode tree pruning** — start simple, add later if tree output is too noisy
- **Nth disambiguation display** — include in ref map but only show `[nth=N]` when duplicates exist (like agent-browser)

---

## 6. File Change Summary

### New files:
| File | Purpose | Est. Lines |
|------|---------|-----------|
| `extension/src/background/ref-state.ts` | Ref map storage + resolution + invalidation | ~60 |

### Modified files:
| File | Change | Est. Lines Changed |
|------|--------|-------------------|
| `extension/src/background/browser-tools.ts` | Add `browser_scan_page`, `browser_diff_page` handlers + ref resolution in click/fill | ~350 |
| `extension/src/background/browser-events.ts` | Add ref map invalidation on navigation/tab close | ~10 |
| `extension/src/background/service-worker.ts` | Import and wire ref-state module | ~5 |
| `extension/host/lib/mcp-server.mjs` | Add 2 new tool definitions to `NATIVE_TOOLS` | ~30 |
| `extension/webmcp-skill.md` | Document `browser_scan_page`, `browser_diff_page`, ref syntax | ~80 |

### Untouched (critical — explicitly preserved):
| File | Why |
|------|-----|
| `codelet/tools/src/web_search.rs` | Separate Rust headless browser — unrelated |
| `codelet/tools/src/chrome_browser.rs` | Separate Rust headless browser — unrelated |
| `codelet/tools/src/page_fetcher.rs` | Separate Rust headless browser — unrelated |
| `codelet/common/src/web_search.rs` | WebSearchAction enum — unrelated |
| All existing extension tools | `browser_navigate`, `browser_screenshot`, `browser_execute_script`, `browser_list_tabs`, `browser_switch_tab`, `browser_close_tab`, `browser_get_page_content`, `browser_click_element`, `browser_fill_form`, `browser_go_back`, `browser_go_forward`, `browser_create_tab` — all preserved, only click/fill gain ref resolution |

---

## 7. Interaction With Existing Extension Architecture

### 7.1 How tool calls flow (existing, unchanged):

```
AI agent
  → ConnectMCP(http://localhost:19876/mcp)
  → tools/call { name: "browser_scan_page", arguments: {} }
  → MCP server (mcp-server.mjs)
  → native messaging → service worker (service-worker.ts)
  → message-router.ts dispatches to browser-tools.ts handler
  → chrome.scripting.executeScript() injects scan into tab
  → results flow back the same path
```

### 7.2 Ref resolution path (new):

```
AI agent calls: browser_click_element { selector: "@e3" }
  → browser-tools.ts handler detects "@" prefix
  → resolveRef(tabId, "e3") from ref-state.ts
  → gets CSS selector "#submit-btn"
  → executes chrome.scripting.executeScript with resolved selector
  → existing click logic unchanged
```

### 7.3 Ref invalidation path (new):

```
User navigates in browser
  → chrome.tabs.onUpdated fires with changeInfo.url
  → browser-events.ts already catches this (line 95-103)
  → NEW: also calls clearTabScanState(tabId)
  → next browser_click_element with @ref → "Ref not found. Run browser_scan_page first."
```

---

## 8. agent-browser Code Reference Index

For implementation reference, here are the exact line ranges in agent-browser:

| Concept | File | Lines | Description |
|---------|------|-------|-------------|
| RefMap interface | `src/snapshot.ts` | 22-35 | Data structure for ref→locator mapping |
| Interactive roles set | `src/snapshot.ts` | 70-88 | Which ARIA roles get refs |
| Content roles set | `src/snapshot.ts` | 93-104 | Roles for text extraction context |
| Structural roles set | `src/snapshot.ts` | 109-128 | Roles filtered in compact mode |
| Selector builder | `src/snapshot.ts` | 133-136 | Builds Playwright locator strings |
| Cursor-interactive detection | `src/snapshot.ts` | 142-261 | Finds cursor:pointer / onclick / tabindex elements |
| Enhanced snapshot entry point | `src/snapshot.ts` | 266-336 | Main function: scan + ref assignment + cursor detection |
| ARIA tree processing | `src/snapshot.ts` | 388-448 | Parse ariaSnapshot lines, add refs, apply filters |
| Duplicate handling | `src/snapshot.ts` | 340-383 | RoleNameTracker for nth disambiguation |
| Compact tree pruning | `src/snapshot.ts` | 558-600 | Remove empty structural branches |
| Ref parsing from CLI | `src/snapshot.ts` | 604-616 | Parse `@e1`, `ref=e1`, `e1` formats |
| Snapshot stats | `src/snapshot.ts` | 621-640 | Token count estimation |
| Myers diff algorithm | `src/diff.ts` | 17-118 | Line-level diff with edit script |
| Diff stats computation | `src/diff.ts` | 123-158 | additions/removals/unchanged/changed |
| Screenshot pixel diff | `src/diff.ts` | 178-339 | Canvas-based visual diff (not needed for V1) |
| Ref resolution in actions | `src/actions.ts` | (search for `parseRef`) | Resolves @ref before executing |
| Last snapshot storage | `src/browser.ts` | (search for `lastSnapshot`) | Stores previous snapshot for diff |

---

## 9. Proposed AI Workflow (Extension Skill Guide)

This would be documented in `extension/webmcp-skill.md`:

### Scan a page for interactive elements
```
1. browser_navigate → URL
2. browser_scan_page → get tree with @e1, @e2, @e3 refs
```

### Interact using refs
```
3. browser_fill_form → { selector: "@e1", value: "user@test.com" }
4. browser_fill_form → { selector: "@e2", value: "password123" }
5. browser_click_element → { selector: "@e3" }
```

### Verify the action worked
```
6. browser_diff_page → see what changed
   Output: + heading "Dashboard" / - button "Sign In"
```

### Re-scan after navigation
```
7. browser_scan_page → get fresh tree with new refs
```

### Fallback: CSS selectors still work
```
browser_click_element → { selector: "#legacy-button" }  // still works
browser_scan_page is optional — all existing tools work without it
```

---

## 10. Estimation Notes

Based on the implementation scope:

| Component | Complexity | Points |
|-----------|-----------|--------|
| Scanning JS function (TreeWalker + interactivity + visibility + selectors + tree formatting) | Complex — core novelty | 5 |
| ref-state.ts module + service worker wiring + event invalidation | Simple — straightforward state management | 2 |
| Ref resolution in browser_click_element / browser_fill_form | Simple — string prefix check + map lookup | 1 |
| browser_diff_page tool (simplified Myers diff) | Moderate — text processing | 2 |
| MCP tool definitions + skill doc updates | Simple — JSON + markdown | 1 |
| Tests for all of the above | Moderate — unit tests for scan logic, ref resolution, diff | 3 |

**Total: ~13 points** — at the upper edge. Could be split into:
- **LOCATE-001a**: Scanning function + ref map + ref resolution in click/fill (8 pts)
- **LOCATE-001b**: Diff tool + skill documentation + test coverage (5 pts)
