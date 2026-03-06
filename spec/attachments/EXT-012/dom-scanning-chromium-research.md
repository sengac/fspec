# Research: AI-Optimized DOM Element Location — Chromium Source References

## Date: 2026-03-06
## Work Unit: EXT-012

---

## Overview

This document maps each DOM API and Chrome Extension API used by the `browser_scan_page` tool to its implementation in the Chromium/Blink source code, proving accuracy and documenting behavior guarantees.

---

## 1. Element Visibility: `element.checkVisibility()`

### What We Use It For
Filtering out invisible elements (display:none, visibility:hidden, opacity:0, content-visibility:auto) before including them in scan results.

### API Surface
```typescript
element.checkVisibility({
  opacityProperty: true,      // Check if opacity is 0
  visibilityProperty: true,   // Check if visibility is hidden
  contentVisibilityAuto: true  // Check if content-visibility: auto is skipping rendering
})
```

### Chromium Source References

| File | URL | What It Shows |
|------|-----|---------------|
| **element.idl** (WebIDL definition) | [source.chromium.org/.../element.idl](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/dom/element.idl) | Lines 22-28: `CheckVisibilityOptions` dictionary with `checkOpacity`, `checkVisibilityCSS`, `contentVisibilityAuto`, `opacityProperty`, `visibilityProperty` |
| **element.cc** (C++ implementation) | [chromium.googlesource.com/.../element.cc](https://chromium.googlesource.com/chromium/src/+/HEAD/third_party/blink/renderer/core/dom/element.cc) | Contains `Element::checkVisibility()` — checks LayoutObject existence (display:none has no layout object), then checks computed style for visibility/opacity/content-visibility |
| **v8_check_visibility_options.h** (V8 binding) | included from element.cc | Auto-generated from IDL, maps JS options dict to C++ struct |

### MDN Reference
- https://developer.mozilla.org/en-US/docs/Web/API/Element/checkVisibility

### Browser Support
- Chrome 105+ (base), Chrome 113+ (extra properties: opacityProperty, visibilityProperty, contentVisibilityAuto)
- Our extension runs IN Chrome, so these are always available.

### Key Behavior Notes
- Returns `false` if element has no associated box (display:none, display:contents)
- Returns `false` if element or ancestor has visibility:hidden (when visibilityProperty=true)
- Returns `false` if element or ancestor has opacity:0 (when opacityProperty=true)
- Does NOT check if element is in viewport — that requires `getBoundingClientRect()` intersection check

---

## 2. Element Geometry: `element.getBoundingClientRect()`

### What We Use It For
- Checking if element has zero dimensions (width=0, height=0 means non-interactive)
- Checking if element is within the viewport (for filtering off-screen elements)
- Reporting element position in the scan result for AI spatial understanding

### Chromium Source References

| File | URL | What It Shows |
|------|-----|---------------|
| **element.idl** | [source.chromium.org/.../element.idl](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/dom/element.idl) | `DOMRect getBoundingClientRect()` declaration |
| **element.cc** | [chromium.googlesource.com/.../element.cc](https://chromium.googlesource.com/chromium/src/+/HEAD/third_party/blink/renderer/core/dom/element.cc) | `Element::getBoundingClientRect()` — calls `GetBoundingClientRect()` which computes layout rect relative to viewport |
| **dom_rect.idl** | [source.chromium.org/.../dom_rect.idl](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/geometry/dom_rect.idl) | DOMRect interface: x, y, width, height, top, right, bottom, left |

### MDN Reference
- https://developer.mozilla.org/en-US/docs/Web/API/Element/getBoundingClientRect

### Key Behavior Notes
- Returns viewport-relative coordinates (accounts for scroll position)
- Returns `{x:0, y:0, width:0, height:0}` for elements with display:none
- Includes CSS transforms, padding, border in the rect
- On inline elements, returns the bounding box of all line boxes

---

## 3. DOM Traversal: `document.createTreeWalker()`

### What We Use It For
Efficient O(n) traversal of all DOM elements to find interactable ones. This is faster than `querySelectorAll('*')` because:
1. TreeWalker doesn't create an intermediate NodeList
2. We can skip subtrees (FILTER_REJECT) for elements like `<script>`, `<style>`, `<noscript>`
3. Memory-efficient: one iterator vs. collecting all nodes

### Chromium Source References

| File | URL | What It Shows |
|------|-----|---------------|
| **document.idl** | [source.chromium.org/.../document.idl](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/dom/document.idl) | `TreeWalker createTreeWalker(Node root, optional unsigned long whatToShow = 0xFFFFFFFF, optional NodeFilter? filter = null)` |
| **tree_walker.cc** | [chromium.googlesource.com/.../tree_walker.cc](https://chromium.googlesource.com/chromium/src/+/HEAD/third_party/blink/renderer/core/dom/tree_walker.cc) | C++ implementation of TreeWalker traversal (nextNode, parentNode, etc.) |
| **node_filter.idl** | [source.chromium.org/.../node_filter.idl](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/dom/node_filter.idl) | Constants: SHOW_ELEMENT (1), FILTER_ACCEPT (1), FILTER_REJECT (2), FILTER_SKIP (3) |

### MDN Reference
- https://developer.mozilla.org/en-US/docs/Web/API/Document/createTreeWalker

### Usage Pattern
```javascript
const walker = document.createTreeWalker(
  document.body,
  NodeFilter.SHOW_ELEMENT,
  {
    acceptNode(node) {
      const tag = node.tagName;
      // Skip script/style/noscript subtrees entirely
      if (tag === 'SCRIPT' || tag === 'STYLE' || tag === 'NOSCRIPT' || tag === 'SVG') {
        return NodeFilter.FILTER_REJECT; // skip node AND children
      }
      return NodeFilter.FILTER_ACCEPT;
    }
  }
);
while (walker.nextNode()) {
  // Process walker.currentNode
}
```

---

## 4. Computed Style: `window.getComputedStyle()`

### What We Use It For
- Checking `pointer-events: none` (element cannot receive clicks)
- Checking `cursor` style for visual interactivity hints
- Fallback visibility checks when `checkVisibility()` is insufficient

### Chromium Source References

| File | URL | What It Shows |
|------|-----|---------------|
| **window.idl** | [source.chromium.org/.../window.idl](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/frame/window.idl) | `CSSStyleDeclaration getComputedStyle(Element elt, optional DOMString? pseudoElt = null)` |
| **css_computed_style_declaration.cc** | [chromium.googlesource.com/.../css_computed_style_declaration.cc](https://chromium.googlesource.com/chromium/src/+/HEAD/third_party/blink/renderer/core/css/css_computed_style_declaration.cc) | Full implementation of computed style resolution |

### MDN Reference
- https://developer.mozilla.org/en-US/docs/Web/API/Window/getComputedStyle

### Key Behavior Notes
- Returns the RESOLVED style (after cascade, inheritance, and defaults)
- Triggers layout if needed (potential perf cost — use sparingly)
- `pointer-events: none` check is critical for elements that look clickable but aren't

---

## 5. Element Matching: `element.matches()`

### What We Use It For
Testing whether an element matches our interactable selectors (e.g., `a[href]`, `button`, `input`, `[role="button"]`, `[contenteditable="true"]`).

### Chromium Source References

| File | URL | What It Shows |
|------|-----|---------------|
| **element.idl** | [source.chromium.org/.../element.idl](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/dom/element.idl) | `boolean matches(DOMString selectors)` |
| **element.cc** | [chromium.googlesource.com/.../element.cc](https://chromium.googlesource.com/chromium/src/+/HEAD/third_party/blink/renderer/core/dom/element.cc) | `Element::matches()` — delegates to SelectorQuery for matching |

### MDN Reference
- https://developer.mozilla.org/en-US/docs/Web/API/Element/matches

### Interactable Selector Set
```javascript
const INTERACTABLE_SELECTOR = [
  'a[href]',
  'button',
  'input',
  'textarea',
  'select',
  '[role="button"]',
  '[role="link"]',
  '[role="checkbox"]',
  '[role="radio"]',
  '[role="tab"]',
  '[role="menuitem"]',
  '[role="option"]',
  '[role="switch"]',
  '[role="textbox"]',
  '[role="combobox"]',
  '[role="searchbox"]',
  '[role="slider"]',
  '[role="spinbutton"]',
  '[contenteditable="true"]',
  '[contenteditable=""]',
  '[tabindex]',
  'summary',
  'details',
  'label',
].join(',');
```

---

## 6. ARIA Properties

### What We Use It For
Extracting semantic meaning from elements: role, aria-label, aria-labelledby, aria-describedby, aria-expanded, aria-checked, aria-disabled, etc.

### Chromium Source References

| File | URL | What It Shows |
|------|-----|---------------|
| **aria_properties.idl** | [source.chromium.org/.../accessibility_properties.idl](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/html/html_element.idl) | ARIA attribute declarations on HTMLElement |
| **ax_object.cc** | [chromium.googlesource.com/.../ax_object.cc](https://chromium.googlesource.com/chromium/src/+/HEAD/third_party/blink/renderer/modules/accessibility/ax_object.cc) | How accessibility tree computes roles and labels |

### W3C ARIA Spec
- https://www.w3.org/TR/wai-aria-1.2/
- Roles that imply interactivity: `button`, `link`, `checkbox`, `radio`, `tab`, `menuitem`, `option`, `switch`, `textbox`, `combobox`, `searchbox`, `slider`, `spinbutton`

---

## 7. Script Injection: `chrome.scripting.executeScript()`

### What We Use It For
Injecting the DOM scanning function into the active tab. Runs in the ISOLATED world (default), which shares the page's DOM but has a separate JavaScript execution context.

### Chrome Extension API References

| Resource | URL | What It Shows |
|----------|-----|---------------|
| **chrome.scripting API docs** | [developer.chrome.com/.../scripting](https://developer.chrome.com/docs/extensions/reference/api/scripting) | Full API reference for executeScript |
| **scripting.idl** (Chromium source) | [source.chromium.org/.../scripting.idl](https://source.chromium.org/chromium/chromium/src/+/main:chrome/common/extensions/api/scripting.idl) | IDL definition of chrome.scripting namespace |
| **scripting_api.cc** | [chromium.googlesource.com/.../scripting_api.cc](https://chromium.googlesource.com/chromium/src/+/HEAD/chrome/browser/extensions/api/scripting/scripting_api.cc) | C++ implementation of chrome.scripting.executeScript |

### Key Behavior: ISOLATED vs MAIN world
- **ISOLATED** (default): Content script world. Shares DOM, separate JS context. Can access `document.*`, `element.*`, `window.getComputedStyle()` — but NOT page-defined JS variables.
- **MAIN**: Page's JS context. Can access page variables but subject to CSP restrictions.
- For DOM scanning, ISOLATED world is perfect — we only need DOM APIs.

### InjectionResult
```typescript
interface InjectionResult<Result> {
  result: Result;     // The return value of the injected function
  documentId: string; // ID of the document where the script ran
  frameId: number;    // Frame ID (0 for main frame)
}
```

### Why ISOLATED Works for Scanning
Per Chromium's content script isolation model:
- Content scripts share the DOM with the page
- `document.querySelector()`, `element.checkVisibility()`, `getBoundingClientRect()`, `getComputedStyle()`, `createTreeWalker()` — all operate on the shared DOM
- ARIA attributes (`getAttribute('role')`, `getAttribute('aria-label')`) are DOM attributes, accessible from ISOLATED world
- Only page-defined JavaScript variables/functions are isolated

---

## 8. CSS Selector Generation

### What We Use It For
Generating a unique CSS selector for each interactable element, so the AI can use it with `browser_click_element` or `browser_fill_form`.

### Strategy (ordered by reliability)

1. **ID-based**: `#myButton` — most reliable, but not all elements have IDs
2. **Data attribute**: `[data-testid="submit"]` — common in React/test-friendly apps
3. **Unique attribute combo**: `input[type="email"][name="email"]` — often unique
4. **nth-child path**: `body > div:nth-child(2) > form > button:nth-child(3)` — always works but fragile

### Chromium Source for `querySelector` (validation)

| File | URL | What It Shows |
|------|-----|---------------|
| **selector_query.cc** | [chromium.googlesource.com/.../selector_query.cc](https://chromium.googlesource.com/chromium/src/+/HEAD/third_party/blink/renderer/core/css/selector_query.cc) | How CSS selectors are parsed and matched |
| **css_selector_parser.cc** | [chromium.googlesource.com/.../css_selector_parser.cc](https://chromium.googlesource.com/chromium/src/+/HEAD/third_party/blink/renderer/core/css/parser/css_selector_parser.cc) | CSS selector parsing implementation |

---

## 9. Prior Art: DOM Scanning in AI Browser Agents

### Tarsier (reworkd/tarsier)
- **Approach**: Tags interactable elements with numbered brackets `[1]`, `[2]`, etc.
- **Source**: https://github.com/reworkd/tarsier
- **Key insight**: LLMs work best with numbered references, not CSS selectors
- **Our adaptation**: We use numeric `ref` indices + CSS selectors (best of both worlds)

### Nanobrowser DOM Abstraction
- **Approach**: Full DOM tree representation with element hashing and history tracking
- **Source**: https://github.com/nanobrowser/nanobrowser (see `DOMElementNode`, `DOMBaseNode`)
- **Key insight**: CSS selector generation uses safe attribute filtering and nth-child fallback
- **DeepWiki**: https://deepwiki.com/reindent/nanobrowser/2.3-dom-abstraction

### dom-engine (The-Agentic-Intelligence-Co)
- **Approach**: Turns website DOMs into actionable context for browser agents
- **Source**: https://github.com/The-Agentic-Intelligence-Co/dom-engine

### browser-use
- **Approach**: Screenshot highlighting with numbered DOM element overlays
- **Source**: https://github.com/browser-use/browser-use

### Key Design Decisions Informed by Prior Art
1. **Numeric refs** (from Tarsier) — AI models reference elements by number
2. **CSS selector generation** (from Nanobrowser) — id > data-attr > nth-child path
3. **Visibility filtering** (from browser-use) — only include what's actually visible
4. **Structured JSON output** (from dom-engine) — not visual overlays, but structured data
5. **ARIA semantics** (our addition) — extract role/label for semantic understanding

---

## 10. Extension Permissions

### Already Declared in manifest.json
```json
{
  "permissions": ["activeTab", "tabs", "scripting", "storage", "offscreen", "nativeMessaging", "userScripts"],
  "host_permissions": ["<all_urls>"]
}
```

### What browser_scan_page Requires
- `scripting` — for `chrome.scripting.executeScript()` ✅ Already declared
- `activeTab` or `host_permissions` — for executing on the active tab ✅ Already declared
- **No additional permissions needed**

### Chrome Permissions Reference
- https://developer.chrome.com/docs/extensions/reference/api/scripting#permissions
- `"scripting"` permission is required to use `chrome.scripting.executeScript()`

---

## Summary of Chromium Source Verification

| API | Verified In Source | Status |
|-----|--------------------|--------|
| `element.checkVisibility()` | element.idl (lines 22-28), element.cc | ✅ Confirmed |
| `element.getBoundingClientRect()` | element.idl, element.cc | ✅ Confirmed |
| `document.createTreeWalker()` | document.idl, tree_walker.cc | ✅ Confirmed |
| `window.getComputedStyle()` | window.idl, css_computed_style_declaration.cc | ✅ Confirmed |
| `element.matches()` | element.idl, element.cc | ✅ Confirmed |
| `element.getAttribute()` (ARIA) | element.idl | ✅ Confirmed |
| `chrome.scripting.executeScript()` | scripting.idl, scripting_api.cc | ✅ Confirmed |
| ISOLATED world DOM access | Content script isolation model docs | ✅ Confirmed |
| No additional permissions needed | manifest.json already has scripting + host_permissions | ✅ Confirmed |

All DOM APIs used by the scanning function are standard web APIs implemented in Blink (Chromium's rendering engine). They work in the ISOLATED content script world because they operate on the shared DOM, not page-specific JavaScript.
