# LOCATE-007: Advanced Interactivity Heuristics — Reference Guide

## Overview

This card enhances the DOM scanning function (from LOCATE-004) with battle-tested heuristics from browser-use (~80k stars) and nanobrowser. These handle the long tail of edge cases that simple CSS selector matching misses.

## Heuristic 1: Label Wrapper Detection

**Source:** browser-use `clickable_elements.py` → `has_form_control_descendant()`

**Problem:** Ant Design and similar libraries wrap form controls in labels:
```html
<label class="ant-checkbox-wrapper">
  <span class="ant-checkbox">
    <input type="checkbox">
  </span>
  <span>Remember me</span>
</label>
```

Without this heuristic, the scanner would create refs for both the label AND the input — double-counting.

**Implementation:**
```javascript
function hasFormControlDescendant(el, maxDepth = 2) {
  if (maxDepth <= 0) return false;
  for (const child of el.children) {
    if (child.matches('input, select, textarea')) return true;
    if (hasFormControlDescendant(child, maxDepth - 1)) return true;
  }
  return false;
}

// In interactivity check:
if (el.tagName === 'LABEL') {
  // Labels with 'for' attribute → skip (they proxy click to target input)
  if (el.hasAttribute('for')) return false;
  // Labels wrapping form controls → interactive (they ARE the click target)
  if (hasFormControlDescendant(el)) return true;
  return false;
}
```

## Heuristic 2: Bounding Box Propagation

**Source:** browser-use `serializer.py` → PROPAGATING_ELEMENTS

**Problem:** Links and buttons containing child elements create multiple refs:
```html
<a href="/home">
  <span class="icon"><svg>...</svg></span>
  <span>Home</span>
</a>
```
Without propagation: 3 refs (a, span+svg, span). With propagation: 1 ref (a).

**Implementation:**
```javascript
const PROPAGATING_SELECTORS = [
  'a', 'button',
  'div[role="button"]', 'div[role="combobox"]',
  'span[role="button"]', 'span[role="combobox"]',
  'input[role="combobox"]',
];
const CONTAINMENT_THRESHOLD = 0.99;  // 99%

function isFullyContainedBy(childRect, parentRect) {
  const childArea = childRect.width * childRect.height;
  if (childArea === 0) return true;
  
  const overlapX = Math.max(0,
    Math.min(childRect.right, parentRect.right) - Math.max(childRect.left, parentRect.left));
  const overlapY = Math.max(0,
    Math.min(childRect.bottom, parentRect.bottom) - Math.max(childRect.top, parentRect.top));
  const overlapArea = overlapX * overlapY;
  
  return (overlapArea / childArea) >= CONTAINMENT_THRESHOLD;
}

// Post-processing after initial scan:
// For each propagating parent, remove child refs that are fully contained
```

## Heuristic 3: Compound Control Collapsing

**Source:** browser-use `serializer.py`

**Problem:** HTML5 `<input type="date">` renders with internal shadow DOM spinners (year/month/day). The scanner should NOT expose these as separate refs.

**Implementation:**
```javascript
const COMPOUND_INPUT_TYPES = new Set([
  'date', 'time', 'datetime-local', 'month', 'week',
  'range', 'number', 'color', 'file',
]);

// During TreeWalker traversal:
if (el.tagName === 'INPUT' && COMPOUND_INPUT_TYPES.has(el.type)) {
  // Don't traverse into shadow DOM children
  // Represent as single element with type attribute
}
```

## Heuristic 4: Search Element Detection

**Source:** browser-use `clickable_elements.py`

**Problem:** Many search controls are non-semantic divs/spans:
```html
<div class="search-icon magnify" data-action="toggle-search">🔍</div>
```

**Implementation:**
```javascript
const SEARCH_INDICATORS = new Set([
  'search', 'magnify', 'glass', 'lookup', 'find', 'query',
  'search-icon', 'search-btn', 'search-button', 'searchbox',
]);

function isSearchElement(el) {
  const classNames = (el.className || '').toLowerCase();
  const id = (el.id || '').toLowerCase();
  
  for (const indicator of SEARCH_INDICATORS) {
    if (classNames.includes(indicator) || id.includes(indicator)) return true;
  }
  
  // Check data-* attributes
  for (const attr of el.attributes) {
    if (attr.name.startsWith('data-') && SEARCH_INDICATORS.has(attr.value.toLowerCase())) {
      return true;
    }
  }
  
  return false;
}
```

## Heuristic 5: Icon-Size Detection

**Source:** browser-use `clickable_elements.py`

**Problem:** Small interactive icons (10-50px) often lack ARIA roles:
```html
<span class="close-btn" style="width:24px;height:24px">✕</span>
```

**Implementation:**
```javascript
function isLikelyInteractiveIcon(el) {
  const rect = el.getBoundingClientRect();
  const w = rect.width;
  const h = rect.height;
  
  if (w < 10 || w > 50 || h < 10 || h > 50) return false;
  
  return (
    el.className ||
    el.getAttribute('role') ||
    el.getAttribute('data-action') ||
    el.getAttribute('aria-label')
  );
}
```

## Heuristic 6: Early Exit Checks

**Implementation:**
```javascript
// At the very top of interactivity check:
if (el.getAttribute('aria-disabled') === 'true') return false;
if (el.getAttribute('aria-hidden') === 'true') return false;
if (el.hasAttribute('inert')) return false;
```

## Heuristic 7: Dynamic Class Filtering

**Source:** browser-use `views.py` → DYNAMIC_CLASS_PATTERNS

**Purpose:** For stable element hashing across UI state changes.

```javascript
const DYNAMIC_CLASS_PATTERNS = new Set([
  'focus', 'hover', 'active', 'selected', 'disabled',
  'animation', 'transition', 'loading', 'open', 'closed',
  'expanded', 'collapsed', 'visible', 'hidden', 'pressed',
  'checked', 'highlighted', 'current', 'entering', 'leaving',
]);

function filterDynamicClasses(classStr) {
  return classStr.split(/\s+/)
    .filter(c => !DYNAMIC_CLASS_PATTERNS.has(c.toLowerCase()) &&
                 ![...DYNAMIC_CLASS_PATTERNS].some(p => c.toLowerCase().includes(p)))
    .sort()
    .join(' ');
}
```

## Heuristic 8: Validation Attribute Inclusion

```javascript
const INCLUDE_ATTRIBUTES = [
  'type', 'checked', 'selected', 'expanded', 'pressed', 'disabled', 'required',
  'placeholder', 'min', 'max', 'minlength', 'maxlength', 'step', 'pattern',
  'accept', 'multiple', 'inputmode', 'autocomplete',
  'aria-expanded', 'aria-checked', 'contenteditable',
];

// For heading elements:
// Include level from tag name: h1 → [level=1]
```

## Testing Strategy

Each heuristic should have dedicated test cases:

1. **Label wrapper**: Ant Design checkbox pattern → single ref
2. **Label with for**: `<label for="email">` → no ref on label
3. **Bounding box**: `<a><span><img></span>text</a>` → single ref on `<a>`
4. **Compound date**: `<input type="date">` → single ref, includes min/max
5. **Search div**: `<div class="search-icon">` → gets ref
6. **Icon button**: 24x24 span with class → gets ref
7. **aria-disabled**: Element with `aria-disabled="true"` → skipped
8. **Dynamic classes**: `"btn active focus"` → hash as `"btn"` only
9. **Validation attrs**: `<input required minlength=8>` → attributes in output
