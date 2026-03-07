/**
 * fspec Browser Agent - DOM Scanner
 *
 * Pure helper functions for scanning page DOM and building
 * an accessibility-tree-like output with ephemeral refs.
 *
 * These functions are used by the browser_scan_page handler
 * in browser-tools.ts. The actual DOM scanning is injected
 * via chrome.scripting.executeScript() — these helpers are
 * also importable for testing in jsdom.
 *
 * Implemented by: LOCATE-004
 */

/** Raw element data returned by the injected scanning function */
export interface RawElement {
  tagName: string;
  role: string;
  name: string;
  selector: string;
  interactive: boolean;
  depth: number;
  attributes: Record<string, string>;
  ref?: string;
}

/* ── Re-exports from helpers ──────────────────────────────────── */

export {
  isDynamicId,
  generateSelector,
  isVisible,
  isInteractiveElement,
  shouldClaimChildren,
  getRelevantAttributes,
} from './dom-scanner-helpers';

export {
  hasFormControlDescendant,
  isFullyContainedBy,
  isSearchElement,
  isLikelyInteractiveIcon,
  filterDynamicClasses,
  PROPAGATING_SELECTORS,
  COMPOUND_INPUT_TYPES,
} from './dom-scanner-heuristics';

/* ── Implicit Role Mapping ────────────────────────────────────── */

const INPUT_TYPE_ROLES: Record<string, string> = {
  text: 'textbox',
  email: 'textbox',
  password: 'textbox',
  tel: 'textbox',
  url: 'textbox',
  search: 'searchbox',
  checkbox: 'checkbox',
  radio: 'radio',
  number: 'spinbutton',
  range: 'slider',
  submit: 'button',
  reset: 'button',
  button: 'button',
  image: 'button',
};

const TAG_ROLES: Record<string, string> = {
  BUTTON: 'button',
  TEXTAREA: 'textbox',
  SELECT: 'combobox',
  NAV: 'navigation',
  MAIN: 'main',
  ASIDE: 'complementary',
  FOOTER: 'contentinfo',
  HEADER: 'banner',
  SUMMARY: 'button',
  DETAILS: 'group',
};

/** Get the implicit ARIA role for an element. Explicit role always overrides. */
export function getImplicitRole(el: Element): string {
  const explicit = el.getAttribute('role');
  if (explicit) {
    return explicit;
  }

  const tag = el.tagName;

  const headingMatch = /^H([1-6])$/.exec(tag);
  if (headingMatch) {
    return 'heading';
  }

  if (tag === 'A' && el.hasAttribute('href')) {
    return 'link';
  }

  if (tag === 'INPUT') {
    const type = (el.getAttribute('type') ?? 'text').toLowerCase();
    return INPUT_TYPE_ROLES[type] ?? 'textbox';
  }

  if (tag === 'SECTION' && el.hasAttribute('aria-label')) {
    return 'region';
  }

  return TAG_ROLES[tag] ?? '';
}

/* ── Accessible Name Extraction ───────────────────────────────── */

/** Extract the accessible name for an element using priority chain */
export function getAccessibleName(el: Element): string {
  // 1. aria-label
  const ariaLabel = el.getAttribute('aria-label');
  if (ariaLabel) {
    return ariaLabel.trim();
  }

  // 2. aria-labelledby
  const labelledBy = el.getAttribute('aria-labelledby');
  if (labelledBy) {
    const refEl = el.ownerDocument.getElementById(labelledBy);
    if (refEl) {
      const text = (refEl.textContent ?? '').trim();
      if (text) {
        return truncateName(text);
      }
    }
  }

  // 3. placeholder (for inputs)
  const placeholder = el.getAttribute('placeholder');
  if (placeholder) {
    return placeholder.trim();
  }

  // 4. value (for inputs with value)
  if ('value' in el && typeof (el as HTMLInputElement).value === 'string') {
    const val = (el as HTMLInputElement).value.trim();
    if (val) {
      return truncateName(val);
    }
  }

  // 5. alt (for images)
  const alt = el.getAttribute('alt');
  if (alt) {
    return alt.trim();
  }

  // 6. title
  const title = el.getAttribute('title');
  if (title) {
    return title.trim();
  }

  // 7. Direct text content
  const text = (el.textContent ?? '').trim();
  return truncateName(text);
}

function truncateName(text: string): string {
  if (text.length <= 80) {
    return text;
  }
  return text.slice(0, 80) + '...';
}

/* ── Tree Formatting ──────────────────────────────────────────── */

/** Format raw elements into an indented accessibility tree text */
export function formatAccessibilityTree(elements: RawElement[]): string {
  const lines: string[] = [];

  for (const el of elements) {
    if (!el.role && !el.interactive) {
      continue; // Skip elements with no role and not interactive
    }

    const indent = '  '.repeat(el.depth);
    let line = `${indent}- ${el.role}`;

    if (el.name) {
      line += ` "${el.name}"`;
    }

    if (el.ref) {
      line += ` [ref=${el.ref}]`;
    }

    // Append attributes
    for (const [key, value] of Object.entries(el.attributes)) {
      if (value === '') {
        line += ` [${key}]`;
      } else {
        line += ` [${key}=${value}]`;
      }
    }

    lines.push(line);
  }

  return lines.join('\n');
}
