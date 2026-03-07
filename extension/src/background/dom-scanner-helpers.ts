/**
 * fspec Browser Agent - DOM Scanner Selectors & Detection
 *
 * Helper functions for CSS selector generation, dynamic ID detection,
 * visibility filtering, interactability detection, parent claiming,
 * and attribute extraction.
 *
 * Separated from dom-scanner.ts to stay under 300 lines per file.
 *
 * Implemented by: LOCATE-004, LOCATE-007
 */

import {
  hasFormControlDescendant,
  isSearchElement,
  isLikelyInteractiveIcon,
  filterDynamicClasses,
  PROPAGATING_SELECTORS,
  COMPOUND_INPUT_TYPES,
} from './dom-scanner-heuristics';

/* ── Dynamic ID Detection ─────────────────────────────────────── */

const FRAMEWORK_ID_PATTERNS = [
  /^r-/, // React/Radix
  /^ember\d/, // Ember
  /^react-/, // React
  /^:r[0-9]/, // React 18 useId
];

/** Check if an ID looks dynamically generated */
export function isDynamicId(id: string): boolean {
  for (const pattern of FRAMEWORK_ID_PATTERNS) {
    if (pattern.test(id)) {
      return true;
    }
  }

  // >30% digits
  const digitCount = (id.match(/\d/g) ?? []).length;
  if (id.length > 0 && digitCount / id.length > 0.3) {
    return true;
  }

  // >8 chars that look hex-like
  if (id.length > 8) {
    const hexChars = id.replace(/[^a-f0-9]/gi, '');
    if (hexChars.length > 8) {
      return true;
    }
  }

  return false;
}

/* ── CSS Selector Generation ──────────────────────────────────── */

/** Generate a reliable CSS selector for an element */
export function generateSelector(el: Element): string {
  // 1. data-testid
  const testId = el.getAttribute('data-testid');
  if (testId) {
    return `[data-testid="${testId}"]`;
  }

  // 2. ID (if not dynamic)
  if (el.id && !isDynamicId(el.id)) {
    return `#${el.id}`;
  }

  // 3. Unique attribute combo
  const attrSelector = buildAttributeSelector(el);
  if (attrSelector) {
    return attrSelector;
  }

  // 4. nth-child path
  return buildNthChildPath(el);
}

function buildAttributeSelector(el: Element): string | undefined {
  const tag = el.tagName.toLowerCase();
  const parts: string[] = [tag];

  const attrs = ['type', 'name', 'role', 'aria-label'];
  for (const attr of attrs) {
    const value = el.getAttribute(attr);
    if (value) {
      parts.push(`[${attr}="${value}"]`);
    }
  }

  // Use filtered (stable) classes for selector stability
  if (el.className && typeof el.className === 'string') {
    const stable = filterDynamicClasses(el.className);
    if (stable) {
      parts.push(`.${stable.split(' ').join('.')}`);
    }
  }

  if (parts.length > 1) {
    const selector = parts.join('');
    try {
      const matches = el.ownerDocument.querySelectorAll(selector);
      if (matches.length === 1) {
        return selector;
      }
    } catch {
      // Invalid selector — fall through
    }
  }
  return undefined;
}

function buildNthChildPath(el: Element): string {
  const parts: string[] = [];
  let current: Element | null = el;

  while (current && current !== el.ownerDocument.body) {
    const parent: Element | null = current.parentElement;
    if (!parent) {
      break;
    }
    const children = Array.from(parent.children);
    const index = children.indexOf(current) + 1;
    const tag = current.tagName.toLowerCase();
    parts.unshift(`${tag}:nth-child(${index})`);
    current = parent;
  }

  return parts.length > 0
    ? `body > ${parts.join(' > ')}`
    : el.tagName.toLowerCase();
}

/* ── Visibility Filtering ─────────────────────────────────────── */

/** Check if an element is visible (not display:none, not zero-size) */
export function isVisible(el: Element): boolean {
  const htmlEl = el as HTMLElement;

  if (typeof htmlEl.checkVisibility === 'function') {
    if (
      !htmlEl.checkVisibility({
        opacityProperty: true,
        visibilityProperty: true,
      } as CheckVisibilityOptions)
    ) {
      return false;
    }
  } else {
    // Fallback: check computed style (jsdom compatibility)
    const style = getComputedStyle(htmlEl);
    if (
      style.display === 'none' ||
      style.visibility === 'hidden' ||
      style.opacity === '0'
    ) {
      return false;
    }
    let parent = htmlEl.parentElement;
    while (parent) {
      const parentStyle = getComputedStyle(parent);
      if (parentStyle.display === 'none') {
        return false;
      }
      parent = parent.parentElement;
    }
  }

  // Zero-size check (skip in environments without layout, e.g. jsdom)
  const rect = el.getBoundingClientRect();
  const hasLayout =
    rect.width !== 0 || rect.height !== 0 || rect.top !== 0 || rect.left !== 0;
  if (hasLayout && (rect.width === 0 || rect.height === 0)) {
    return false;
  }

  return true;
}

/* ── Interactability Detection ────────────────────────────────── */

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
].join(',');

/** Check if an element should be treated as interactive */
export function isInteractiveElement(el: Element): boolean {
  // Early exit: aria-disabled, aria-hidden, inert
  if (el.getAttribute('aria-disabled') === 'true') {
    return false;
  }
  if (el.closest('[aria-hidden="true"]')) {
    return false;
  }
  if (el.closest('[inert]')) {
    return false;
  }

  const style = getComputedStyle(el);
  if (style.pointerEvents === 'none') {
    return false;
  }

  // Label wrapper detection
  if (el.tagName === 'LABEL') {
    if (el.hasAttribute('for')) {
      return false;
    }
    return hasFormControlDescendant(el, 2);
  }

  if (el.matches(INTERACTABLE_SELECTOR)) {
    return true;
  }
  if (style.cursor === 'pointer') {
    return true;
  }
  if (
    el.hasAttribute('onclick') ||
    el.hasAttribute('onmousedown') ||
    el.hasAttribute('onkeydown')
  ) {
    return true;
  }

  // Search element heuristic
  if (isSearchElement(el)) {
    return true;
  }

  // Icon-size heuristic
  if (isLikelyInteractiveIcon(el)) {
    return true;
  }

  return false;
}

/* ── Parent Claiming (Bounding Box Propagation) ───────────────── */

const PROPAGATING_JOINED = PROPAGATING_SELECTORS.join(',');

/** Check if an interactive parent should propagate to children via containment */
export function shouldClaimChildren(el: Element): boolean {
  if (el.matches(PROPAGATING_JOINED)) {
    return true;
  }
  // Label wrapping form controls claims the inner input
  if (
    el.tagName === 'LABEL' &&
    !el.hasAttribute('for') &&
    hasFormControlDescendant(el, 2)
  ) {
    return true;
  }
  // Compound HTML5 inputs claim their shadow DOM sub-components
  if (el.tagName === 'INPUT') {
    const t = (el.getAttribute('type') ?? '').toLowerCase();
    if (COMPOUND_INPUT_TYPES.has(t)) {
      return true;
    }
  }
  return false;
}

/* ── Relevant Attributes ──────────────────────────────────────── */

const RELEVANT_ATTRS = [
  'type',
  'checked',
  'selected',
  'expanded',
  'pressed',
  'disabled',
  'required',
  'placeholder',
  'min',
  'max',
  'minlength',
  'maxlength',
  'step',
  'pattern',
  'accept',
  'multiple',
  'inputmode',
  'autocomplete',
  'aria-expanded',
  'aria-checked',
  'contenteditable',
];

/** Extract validation-relevant attributes from an element */
export function getRelevantAttributes(el: Element): Record<string, string> {
  const attrs: Record<string, string> = {};

  const headingMatch = /^H([1-6])$/.exec(el.tagName);
  if (headingMatch) {
    attrs.level = headingMatch[1];
  }

  for (const attr of RELEVANT_ATTRS) {
    if (el.hasAttribute(attr)) {
      const value = el.getAttribute(attr);
      attrs[attr] = value ?? '';
    }
  }

  const htmlEl = el as HTMLInputElement;
  if (htmlEl.required && !attrs.required) {
    attrs.required = '';
  }
  if (htmlEl.checked && !attrs.checked) {
    attrs.checked = '';
  }

  return attrs;
}
