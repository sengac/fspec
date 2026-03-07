/**
 * fspec Browser Agent - DOM Scanner Advanced Heuristics
 *
 * Exported helper functions for advanced interactivity detection.
 * These are the testable versions of heuristics that are also inlined
 * in scanPageDOM (scan-page-dom.ts) for chrome.scripting.executeScript().
 *
 * Separated from dom-scanner-helpers.ts to stay under 300 lines per file.
 *
 * Implemented by: LOCATE-007
 */

/* ── Label Wrapper Detection ──────────────────────────────────── */

/** Check if an element has a form control descendant within maxDepth levels */
export function hasFormControlDescendant(el: Element, maxDepth = 2): boolean {
  if (maxDepth <= 0) {
    return false;
  }
  for (const child of Array.from(el.children)) {
    if (child.matches('input, select, textarea')) {
      return true;
    }
    if (hasFormControlDescendant(child, maxDepth - 1)) {
      return true;
    }
  }
  return false;
}

/* ── Bounding Box Containment ─────────────────────────────────── */

interface RectLike {
  left: number;
  top: number;
  right: number;
  bottom: number;
  width: number;
  height: number;
}

const CONTAINMENT_THRESHOLD = 0.99;

/** Check if a child rect is 99%+ contained within a parent rect */
export function isFullyContainedBy(
  childRect: RectLike,
  parentRect: RectLike
): boolean {
  const childArea = childRect.width * childRect.height;
  if (childArea === 0) {
    return true;
  }

  const overlapX = Math.max(
    0,
    Math.min(childRect.right, parentRect.right) -
      Math.max(childRect.left, parentRect.left)
  );
  const overlapY = Math.max(
    0,
    Math.min(childRect.bottom, parentRect.bottom) -
      Math.max(childRect.top, parentRect.top)
  );
  const overlapArea = overlapX * overlapY;

  return overlapArea / childArea >= CONTAINMENT_THRESHOLD;
}

/** Selectors for elements that propagate their interactivity to children */
export const PROPAGATING_SELECTORS = [
  'a',
  'button',
  'div[role="button"]',
  'div[role="combobox"]',
  'span[role="button"]',
  'span[role="combobox"]',
  'input[role="combobox"]',
];

/* ── Compound Input Types ─────────────────────────────────────── */

export const COMPOUND_INPUT_TYPES = new Set([
  'date',
  'time',
  'datetime-local',
  'month',
  'week',
  'range',
  'number',
  'color',
  'file',
]);

/* ── Search Element Detection ─────────────────────────────────── */

const SEARCH_INDICATORS = new Set([
  'search',
  'magnify',
  'glass',
  'lookup',
  'find',
  'query',
  'search-icon',
  'search-btn',
  'search-button',
  'searchbox',
]);

/** Detect non-semantic search controls via class/id/data-* heuristics (div/span only) */
export function isSearchElement(el: Element): boolean {
  const tag = el.tagName;
  if (tag !== 'DIV' && tag !== 'SPAN') {
    return false;
  }

  const classNames =
    el.className && typeof el.className === 'string'
      ? el.className.toLowerCase()
      : '';
  const id = (el.id || '').toLowerCase();

  for (const indicator of SEARCH_INDICATORS) {
    if (classNames.includes(indicator) || id.includes(indicator)) {
      return true;
    }
  }

  // Check data-* attributes for search-related values
  for (const attr of Array.from(el.attributes)) {
    if (attr.name.startsWith('data-')) {
      const val = attr.value.toLowerCase();
      for (const indicator of SEARCH_INDICATORS) {
        if (val.includes(indicator)) {
          return true;
        }
      }
    }
  }

  return false;
}

/* ── Icon-Size Detection ──────────────────────────────────────── */

/** Detect small interactive icons (10-50px) with meaningful metadata */
export function isLikelyInteractiveIcon(
  el: Element,
  rect?: DOMRect | RectLike
): boolean {
  const r = rect ?? el.getBoundingClientRect();
  const w = r.width;
  const h = r.height;

  if (w < 10 || w > 50 || h < 10 || h > 50) {
    return false;
  }

  return !!(
    (el.className &&
      typeof el.className === 'string' &&
      el.className.length > 0) ||
    el.getAttribute('role') ||
    el.getAttribute('data-action') ||
    el.getAttribute('aria-label')
  );
}

/* ── Dynamic Class Filtering ──────────────────────────────────── */

const DYNAMIC_CLASS_PATTERNS = new Set([
  'focus',
  'hover',
  'active',
  'selected',
  'disabled',
  'animation',
  'transition',
  'loading',
  'open',
  'closed',
  'expanded',
  'collapsed',
  'visible',
  'hidden',
  'pressed',
  'checked',
  'highlighted',
  'current',
  'entering',
  'leaving',
]);

/** Filter state-related CSS classes for stable element hashing */
export function filterDynamicClasses(classStr: string): string {
  return classStr
    .split(/\s+/)
    .filter(c => {
      if (!c) {
        return false;
      }
      const lower = c.toLowerCase();
      // Exact match
      if (DYNAMIC_CLASS_PATTERNS.has(lower)) {
        return false;
      }
      // Substring match (e.g., "is-loading", "menu-expanded")
      for (const pattern of DYNAMIC_CLASS_PATTERNS) {
        if (lower.includes(pattern)) {
          return false;
        }
      }
      return true;
    })
    .sort()
    .join(' ');
}
