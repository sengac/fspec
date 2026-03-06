# LOCATE-004: DOM Scanning Core — Architecture Document

## Overview

The `browser_scan_page` tool injects a scanning function into the active tab via `chrome.scripting.executeScript()` in the ISOLATED world. The function traverses the DOM, detects interactive elements, generates CSS selectors, and returns a structured accessibility-tree-like output.

## Architecture

```
AI Agent
  → tools/call { name: "browser_scan_page", arguments: { interactive: true } }
  → MCP server → native messaging → service worker
  → browser-tools.ts: browser_scan_page handler
  → chrome.scripting.executeScript({ target: { tabId }, func: scanPageDOM })
  → Returns: { elements: [...], metadata: {...} }
  → Service worker: assigns refs (e1, e2, ...), stores in ref-state.ts
  → Formats accessibility tree text
  → Returns tree text + metadata to AI
```

## Injected Scanning Function (~250 LOC)

### 1. TreeWalker Setup

```javascript
const walker = document.createTreeWalker(
  document.body,
  NodeFilter.SHOW_ELEMENT,
  {
    acceptNode(node) {
      const tag = node.tagName;
      if (tag === 'SCRIPT' || tag === 'STYLE' || tag === 'NOSCRIPT') {
        return NodeFilter.FILTER_REJECT;  // Skip node AND children
      }
      return NodeFilter.FILTER_ACCEPT;
    }
  }
);
```

### 2. Interactability Detection

```javascript
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

Plus heuristic checks:
- `cursor: pointer` via `getComputedStyle(el).cursor`
- `onclick` / `onmousedown` / `onkeydown` attributes
- NOT `pointer-events: none`
- NOT `aria-disabled="true"` or `aria-hidden="true"`

### 3. Visibility Filtering

```javascript
function isVisible(el) {
  // Fast path: checkVisibility API
  if (!el.checkVisibility({ opacityProperty: true, visibilityProperty: true })) {
    return false;
  }
  // Zero-size check
  const rect = el.getBoundingClientRect();
  if (rect.width === 0 || rect.height === 0) {
    return false;
  }
  return true;
}
```

### 4. Role Extraction

Map HTML elements to ARIA roles:

| Tag | Implicit Role |
|-----|-------------|
| `button` | `button` |
| `a[href]` | `link` |
| `input[type=text]` | `textbox` |
| `input[type=checkbox]` | `checkbox` |
| `input[type=radio]` | `radio` |
| `input[type=email]` | `textbox` |
| `input[type=password]` | `textbox` |
| `input[type=search]` | `searchbox` |
| `input[type=number]` | `spinbutton` |
| `input[type=range]` | `slider` |
| `textarea` | `textbox` |
| `select` | `combobox` |
| `h1-h6` | `heading` |
| `nav` | `navigation` |
| `main` | `main` |
| `aside` | `complementary` |
| `footer` | `contentinfo` |
| `header` | `banner` |
| `section[aria-label]` | `region` |

Explicit `role` attribute always overrides implicit role.

### 5. Accessible Name Extraction

Priority order:
1. `aria-label` attribute
2. `aria-labelledby` → resolve to referenced element's text
3. `placeholder` (for inputs)
4. `value` (for inputs with value)
5. `alt` (for images)
6. `title` attribute
7. Direct text content (trimmed, max 80 chars)

### 6. CSS Selector Generation

Ranked by reliability:

1. **data-testid**: `[data-testid="submit-btn"]`
2. **ID** (if not dynamic): `#email-input`
3. **Unique attribute combo**: `input[type="email"][name="email"]`
4. **nth-child path**: `body > div:nth-child(2) > form > button:nth-child(3)`

Dynamic ID detection: skip IDs with >30% digits, >8 chars that look hex-like, or matching patterns like `r-[hash]`, `ember[digits]`, `react-[hash]`.

### 7. Tree Building

For structural context (not just flat list):
- Track depth via `element.parentElement` chain to `document.body`
- Include structural elements (headings, regions, navigation) without refs
- Interactive elements get refs

### 8. Output Format

```
Page: https://example.com/login — "Login - Example App"
Viewport: 1280x720 | Elements: 147 | Interactive: 12

- heading "Sign In" [level=1]
  - textbox "Email" [ref=e1] [type=email] [required]
  - textbox "Password" [ref=e2] [type=password] [required] [minlength=8]
  - checkbox "Remember me" [ref=e3]
  - button "Sign In" [ref=e4]
- region "Footer"
  - link "Forgot Password" [ref=e5]
  - link "Create Account" [ref=e6]
```

### 9. Attribute Inclusion

Include in tree notation ONLY when present:
- `type`, `checked`, `selected`, `expanded`, `pressed`, `disabled`, `required`
- `placeholder`, `min`, `max`, `minlength`, `maxlength`, `step`, `pattern`
- `accept`, `multiple`, `inputmode`, `autocomplete`
- `aria-expanded`, `aria-checked`, `contenteditable`
- `level` (for headings)

Exclude (too noisy):
- `class`, `style`, `data-*` (except data-testid), event handlers

## Service Worker Handler

```typescript
handlers.set('browser_scan_page', async (args) => {
  const tabId = await resolveTabId(args.tabId);
  const interactive = args.interactive !== false;  // default true
  const scopeSelector = args.selector as string | undefined;
  
  // 1. Inject scanning function
  const results = await scripting.executeScript({
    target: { tabId },
    args: [interactive, scopeSelector],
    func: scanPageDOM,  // The injected function
  });
  
  const scanResult = results[0]?.result;
  
  // 2. Assign refs
  let refCounter = 1;
  const refs = new Map<string, RefEntry>();
  for (const element of scanResult.elements) {
    if (element.interactive) {
      const refKey = `e${refCounter++}`;
      refs.set(refKey, {
        selector: element.selector,
        role: element.role,
        name: element.name,
      });
      element.ref = refKey;
    }
  }
  
  // 3. Format tree text
  const treeText = formatAccessibilityTree(scanResult.elements);
  
  // 4. Store in ref-state
  setTabScanState(tabId, { refs, treeText, timestamp: Date.now() });
  
  // 5. Return to AI
  return textResult(treeText + `\n\n${refs.size} interactive elements`);
});
```

## Token Efficiency

Target: <2000 tokens for a typical page with 30-50 interactive elements.

Strategy:
- Only include interactive elements + structural context (headings, regions)
- 2-space indentation (not 4)
- Abbreviate long text content (max 80 chars with "...")
- Skip zero-content structural elements

## Files Modified

| File | Change |
|------|--------|
| `extension/src/background/browser-tools.ts` | Add `browser_scan_page` handler (~200 lines) |
| `extension/src/background/ref-state.ts` | Import and use (from LOCATE-003) |

## Testing Notes

The scanning function runs in a browser environment (content script ISOLATED world), so unit testing requires DOM mocking. Key test scenarios:

1. Simple page with known elements → correct ref assignment
2. Page with hidden elements → filtered out
3. Page with dynamic IDs → fallback selector used
4. Interactive mode vs full mode
5. Scoped scan via selector parameter
6. Error handling for invalid tab / restricted pages
