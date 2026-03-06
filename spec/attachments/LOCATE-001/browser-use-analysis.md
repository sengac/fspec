# Research: browser-use DOM Architecture Analysis — Applicability to LOCATE-001

## Date: 2026-03-06
## Work Unit: LOCATE-001
## Source: https://github.com/browser-use/browser-use (~80k GitHub stars)

---

## 1. Summary

browser-use is the most mature open-source AI browser automation framework. It was evaluated as a potential approach or dependency for LOCATE-001's DOM scanning and element location system.

**Verdict:** browser-use is a **design reference, not a dependency**. Its Python + CDP architecture is incompatible with our Chrome Extension context, but its interactivity detection heuristics and serialization patterns are the most battle-tested in the ecosystem and should be adopted as design patterns.

---

## 2. browser-use Architecture (Current — Post-buildDomTree.js)

browser-use originally injected `buildDomTree.js` (~1500 LOC) via Playwright's `page.evaluate()`, similar to nanobrowser. They have since **completely replaced** this with a pure CDP approach using their `cdp-use` library.

### 2.1 Four Parallel CDP Data Sources

Their `DomService.get_state()` fires four CDP commands in parallel:

| CDP Command | What It Returns | Used For |
|-------------|----------------|----------|
| `DOMSnapshot.captureSnapshot` | Layout tree, bounding boxes, paint order, computed styles | Visibility, occlusion, geometry |
| `Accessibility.getFullAXTree` | Full accessibility tree with roles, names, properties | Semantic role/name extraction |
| `DOM.getDocument` | DOM tree structure with attributes | Tag names, attributes, hierarchy |
| `DOMDebugger.getEventListeners` | JS event listeners attached to elements | Detecting React onClick, Vue @click, etc. |

These are merged into `EnhancedDOMTreeNode` objects, then serialized by `DOMTreeSerializer`.

### 2.2 Key Source Files

| File | Purpose |
|------|---------|
| `browser_use/dom/service.py` | Orchestrates CDP calls, builds enhanced tree |
| `browser_use/dom/views.py` | Data model: EnhancedDOMTreeNode, DOMRect, SimplifiedNode, etc. |
| `browser_use/dom/enhanced_snapshot.py` | Parses CDP DOMSnapshot data for visibility/bounds/styles |
| `browser_use/dom/serializer/serializer.py` | Serializes tree to string for LLM consumption |
| `browser_use/dom/serializer/clickable_elements.py` | Interactivity detection heuristics |
| `browser_use/dom/serializer/paint_order.py` | Occlusion detection via paint order + rectangle union |
| `browser_use/skill_cli/` | CLI for AI agents (`browser-use state`, `browser-use click 5`) |

---

## 3. Why browser-use CANNOT Be Used Directly

| Constraint | Impact |
|-----------|--------|
| **Python library** | LOCATE-001 is TypeScript in a Chrome Extension |
| **Requires full CDP session** | Extension content scripts don't have CDP access without `chrome.debugger` (which shows an intrusive warning banner) |
| **Playwright dependency** | Not available in Chrome Extension context |
| **External process model** | browser-use spawns/connects to Chrome externally; our extension runs *inside* Chrome |
| **`DOMDebugger.getEventListeners` is CDP-only** | Content scripts can't detect JS event handlers |
| **`DOMSnapshot.captureSnapshot` is CDP-only** | Paint order, computed styles via snapshot not available from content scripts |

---

## 4. Design Patterns Worth Adopting

### 4.1 ClickableElementDetector (from `clickable_elements.py`)

The most comprehensive interactivity detection in the ecosystem. Key patterns:

**Label wrapper detection:**
```python
def has_form_control_descendant(element, max_depth=2):
    # Detects: <label><span><input></span></label> (Ant Design pattern)
    # Returns True if input/select/textarea found within 2 levels
    
# Labels with "for" attribute → skip (they proxy to external input)
# Labels wrapping form controls → interactive (they ARE the click target)
```

**AX property signals:**
```python
# Direct interactiveness indicators
if prop.name in ['focusable', 'editable', 'settable'] and prop.value:
    return True
# Interactive state properties (presence = interactive widget)
if prop.name in ['checked', 'expanded', 'pressed', 'selected']:
    return True
```

**Search element heuristics:**
```python
search_indicators = {'search', 'magnify', 'glass', 'lookup', 'find', 'query',
                     'search-icon', 'search-btn', 'search-button', 'searchbox'}
# Checked against: class names, id, data-* attributes
```

**Icon-size heuristics:**
```python
# 10-50px elements with interactive attributes → likely clickable icons
if 10 <= width <= 50 and 10 <= height <= 50:
    if has_class or has_role or has_onclick or has_data_action or has_aria_label:
        return True
```

**JS click listener detection (CDP-only, unavailable to us):**
```python
# This is the one thing we can't replicate in content scripts
if node.has_js_click_listener:  # Detected via DOMDebugger.getEventListeners
    return True
```

**Our fallback for JS listeners:** `onclick`/`onmousedown`/`onkeydown` attributes + `cursor:pointer` computed style + `tabindex` attribute. This covers ~90% of cases.

### 4.2 Bounding Box Propagation (from `serializer.py`)

```python
PROPAGATING_ELEMENTS = [
    {'tag': 'a', 'role': None},
    {'tag': 'button', 'role': None},
    {'tag': 'div', 'role': 'button'},
    {'tag': 'div', 'role': 'combobox'},
    {'tag': 'span', 'role': 'button'},
    {'tag': 'span', 'role': 'combobox'},
    {'tag': 'input', 'role': 'combobox'},
]
DEFAULT_CONTAINMENT_THRESHOLD = 0.99  # 99% containment
```

When an interactive parent element (from the list above) fully contains a child element (by bounding box), the child is excluded from the interactive element list. This prevents:
- `<a><span><img></span>text</a>` → 3 refs instead of 1
- `<button><svg>...</svg>Save</button>` → 2 refs instead of 1

### 4.3 Paint Order Filtering (from `paint_order.py`)

Uses CDP's `paintOrders` data with a rectangle union algorithm (`RectUnionPure`) to detect occluded elements. Elements painted under higher-paint-order elements that have opaque backgrounds are marked `ignored_by_paint_order = True`.

**Our adaptation:** Since paint order isn't available from content scripts, we approximate with `elementFromPoint()` at the element's center, which catches modal overlays and z-index stacking but misses partial occlusion.

### 4.4 Dynamic Class Filtering for Stable Hashing (from `views.py`)

```python
DYNAMIC_CLASS_PATTERNS = frozenset({
    'focus', 'hover', 'active', 'selected', 'disabled',
    'animation', 'transition', 'loading', 'open', 'closed',
    'expanded', 'collapsed', 'visible', 'hidden', 'pressed',
    'checked', 'highlighted', 'current', 'entering', 'leaving',
})

def filter_dynamic_classes(class_str):
    classes = class_str.split()
    stable = [c for c in classes if not any(p in c.lower() for p in DYNAMIC_CLASS_PATTERNS)]
    return ' '.join(sorted(stable))
```

This is used for element re-identification across page state changes. Classes containing these patterns are stripped before hashing, so `btn btn-primary active focus` and `btn btn-primary` produce the same hash.

### 4.5 Attribute Inclusion for LLM Context (from `views.py`)

browser-use carefully curates which attributes are included in the serialized output:

**Always include (automation-critical):**
- `type`, `checked`, `selected`, `expanded`, `pressed`, `disabled`, `required`
- `placeholder`, `value`, `aria-label`, `aria-expanded`, `aria-checked`
- `min`, `max`, `minlength`, `maxlength`, `step`, `pattern`
- `accept`, `multiple`, `inputmode`, `autocomplete`
- `level` (for headings)

**Exclude (noise):**
- `class` (too verbose, often meaningless utility classes)
- `style` (too verbose)
- Most `data-*` (except `data-testid`, `data-state`)
- Event handler attributes (already detected for interactivity, not needed in output)

### 4.6 Compound Control Handling (from `serializer.py`)

HTML5 compound controls are represented as single interactive elements:

```python
# For date/time inputs: DON'T expose internal spinners
# HTML5 date inputs ALWAYS require ISO format (YYYY-MM-DD)
# The placeholder shows the format, the compound parts just confuse the model
if input_type in ['date', 'time', 'datetime-local', 'month', 'week']:
    pass  # Skip compound components

# For range/number/color/file: synthetic compound info only
if input_type == 'range':
    node._compound_children.append({'role': 'slider', 'name': 'Value', ...})
```

### 4.7 CLI Pattern (from `skill_cli/`)

browser-use's CLI provides a `state` command that outputs numbered interactive elements:

```
browser-use state
# → URL: https://example.com/login
# → Title: Login - Example App
# → Elements:
# [1] input#email (type=email, placeholder="Email")
# [2] input#password (type=password, placeholder="Password")
# [3] button#submit ("Sign In")
# [4] a.forgot-password ("Forgot Password")
```

Then: `browser-use click 3` to interact by index.

This is essentially the same scan→interact pattern we're implementing, but via CLI + Playwright rather than Chrome Extension + MCP tools.

---

## 5. What We Adopt vs. What We Don't

### Adopt (design patterns → implementation guidance):

| Pattern | Source | Our Adaptation |
|---------|--------|---------------|
| Label wrapper detection | `clickable_elements.py` | `has_form_control_descendant()` with depth=2 in scan function |
| Bounding box propagation | `serializer.py` | `getBoundingClientRect()` containment check, 99% threshold |
| Dynamic class filtering | `views.py` | Filter before hashing in re-identification |
| Attribute inclusion list | `views.py` | Include validation attrs (min/max/pattern/etc.) in tree output |
| Compound control collapsing | `serializer.py` | Don't expose date/time internal shadow DOM parts |
| Search element heuristics | `clickable_elements.py` | Check class/id for search keywords |
| Icon-size heuristics | `clickable_elements.py` | 10-50px elements with interactive attributes |
| Aria-disabled/hidden early exit | `clickable_elements.py` | Skip immediately in interactivity check |
| Scan→interact→verify pattern | `skill_cli/` | `browser_scan_page` → `@ref` interaction → `browser_diff_page` |

### Explicitly NOT adopting:

| What | Why Not |
|------|---------|
| CDP-based DOM extraction | Requires `chrome.debugger`, shows banner, user-hostile |
| `getEventListeners()` | CDP-only; use onclick attr + cursor:pointer fallback |
| Paint order filtering | CDP-only; use `elementFromPoint()` approximation |
| Python/Playwright dependency | We're TypeScript in a Chrome Extension |
| Their data model (EnhancedDOMTreeNode etc.) | Way too heavy for a content script; build lightweight equivalent |
| Cloud browser integration | Irrelevant to our use case |

---

## 6. Reference Source Links

| Concept | URL |
|---------|-----|
| ClickableElementDetector | https://github.com/browser-use/browser-use/blob/main/browser_use/dom/serializer/clickable_elements.py |
| DOMTreeSerializer | https://github.com/browser-use/browser-use/blob/main/browser_use/dom/serializer/serializer.py |
| Paint order filtering | https://github.com/browser-use/browser-use/blob/main/browser_use/dom/serializer/paint_order.py |
| Enhanced snapshot (CDP parsing) | https://github.com/browser-use/browser-use/blob/main/browser_use/dom/enhanced_snapshot.py |
| Data model (views) | https://github.com/browser-use/browser-use/blob/main/browser_use/dom/views.py |
| DomService (orchestrator) | https://github.com/browser-use/browser-use/blob/main/browser_use/dom/service.py |
| CLI skill | https://github.com/browser-use/browser-use/tree/main/browser_use/skill_cli |
| SKILL.md (Claude Code integration) | https://github.com/browser-use/browser-use/blob/main/skills/browser-use/SKILL.md |
