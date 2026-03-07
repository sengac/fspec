/**
 * Feature: spec/features/advanced-interactivity-heuristics.feature
 *
 * This test file validates the acceptance criteria for LOCATE-007:
 * Advanced Interactivity Heuristics.
 *
 * Tests the new heuristic functions added to dom-scanner-heuristics.ts
 * and their inlined counterparts in scanPageDOM (scan-page-dom.ts).
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { scanPageDOM } from '../scan-page-dom';
import {
  hasFormControlDescendant,
  isFullyContainedBy,
  isSearchElement,
  isLikelyInteractiveIcon,
  filterDynamicClasses,
} from '../dom-scanner-heuristics';
import { isInteractiveElement } from '../dom-scanner-helpers';

/* ── DOM helpers ──────────────────────────────────────────────── */

function clearBody(): void {
  document.body.innerHTML = '';
}

/* ── Tests ────────────────────────────────────────────────────── */

describe('Feature: Advanced Interactivity Heuristics', () => {
  beforeEach(() => {
    clearBody();
  });

  afterEach(() => {
    clearBody();
  });

  describe('Scenario: Label wrapping a form control gets the ref instead of the inner input', () => {
    it('should give the ref to the label, not the inner input', () => {
      // @step Given a page with an Ant Design checkbox pattern where a label wraps a span wrapping an input
      clearBody();
      const label = document.createElement('label');
      label.className = 'ant-checkbox-wrapper';
      const span = document.createElement('span');
      span.className = 'ant-checkbox';
      const input = document.createElement('input');
      input.type = 'checkbox';
      span.appendChild(input);
      label.appendChild(span);
      const textSpan = document.createElement('span');
      textSpan.textContent = 'Remember me';
      label.appendChild(textSpan);
      document.body.appendChild(label);

      // @step When I call browser_scan_page
      const result = scanPageDOM(true);
      const interactiveEls = result.elements.filter(e => e.interactive);

      // @step Then the label element should receive a ref as the interactive element
      const labelEl = interactiveEls.find(e => e.tagName === 'LABEL');
      expect(labelEl).toBeDefined();

      // @step And the inner input should not receive a separate ref
      const inputEl = interactiveEls.find(e => e.tagName === 'INPUT');
      expect(inputEl).toBeUndefined();
    });

    it('should detect form controls within max depth of 2', () => {
      const div = document.createElement('div');
      const span = document.createElement('span');
      const input = document.createElement('input');
      span.appendChild(input);
      div.appendChild(span);
      document.body.appendChild(div);

      expect(hasFormControlDescendant(div, 2)).toBe(true);
    });

    it('should NOT detect form controls beyond max depth', () => {
      const div = document.createElement('div');
      const span1 = document.createElement('span');
      const span2 = document.createElement('span');
      const input = document.createElement('input');
      span2.appendChild(input);
      span1.appendChild(span2);
      div.appendChild(span1);
      document.body.appendChild(div);

      expect(hasFormControlDescendant(div, 1)).toBe(false);
    });
  });

  describe('Scenario: Label with for attribute is skipped', () => {
    it('should skip the label and only ref the input', () => {
      // @step Given a page with a label having a for attribute pointing to an input
      clearBody();
      const label = document.createElement('label');
      label.setAttribute('for', 'email');
      label.textContent = 'Email';
      document.body.appendChild(label);

      const input = document.createElement('input');
      input.id = 'email';
      input.type = 'email';
      input.setAttribute('aria-label', 'Email');
      document.body.appendChild(input);

      // @step When I call browser_scan_page
      const result = scanPageDOM(true);
      const interactiveEls = result.elements.filter(e => e.interactive);

      // @step Then the label should not receive a ref
      const labelEl = interactiveEls.find(e => e.tagName === 'LABEL');
      expect(labelEl).toBeUndefined();

      // @step And only the input should receive a ref
      const inputEl = interactiveEls.find(e => e.tagName === 'INPUT');
      expect(inputEl).toBeDefined();
    });
  });

  describe('Scenario: Bounding box propagation excludes fully contained children', () => {
    it('should only include the parent link, not contained children', () => {
      // @step Given a page with a link containing spans and images that are fully contained within its bounding box
      clearBody();
      const link = document.createElement('a');
      link.href = '/home';
      const iconSpan = document.createElement('span');
      const img = document.createElement('img');
      img.src = 'icon.png';
      img.alt = 'icon';
      iconSpan.appendChild(img);
      link.appendChild(iconSpan);
      const textSpan = document.createElement('span');
      textSpan.textContent = 'Home';
      link.appendChild(textSpan);
      document.body.appendChild(link);

      // @step When I call browser_scan_page
      const result = scanPageDOM(true);
      const interactiveEls = result.elements.filter(e => e.interactive);

      // @step Then only the parent link should receive a ref
      expect(interactiveEls.length).toBe(1);
      expect(interactiveEls[0].role).toBe('link');
      expect(interactiveEls[0].name).toBe('Home');

      // @step And the contained children should be excluded by the 99% area containment check
      const spanEls = result.elements.filter(e => e.tagName === 'SPAN');
      expect(spanEls.length).toBe(0);
    });

    it('should use 99% containment threshold for area check', () => {
      const parentRect = {
        left: 0,
        top: 0,
        right: 100,
        bottom: 50,
        width: 100,
        height: 50,
      };
      const childRect = {
        left: 1,
        top: 1,
        right: 99,
        bottom: 49,
        width: 98,
        height: 48,
      };
      expect(isFullyContainedBy(childRect, parentRect)).toBe(true);

      const outsideRect = {
        left: 50,
        top: 0,
        right: 200,
        bottom: 50,
        width: 150,
        height: 50,
      };
      expect(isFullyContainedBy(outsideRect, parentRect)).toBe(false);
    });
  });

  describe('Scenario: Compound HTML5 inputs are represented as single elements', () => {
    it('should represent a date input as a single element with attributes', () => {
      // @step Given a page with a date input having min and max attributes
      clearBody();
      const input = document.createElement('input');
      input.type = 'date';
      input.setAttribute('min', '2024-01-01');
      input.setAttribute('max', '2025-12-31');
      input.setAttribute('aria-label', 'Date');
      document.body.appendChild(input);

      // @step When I call browser_scan_page
      const result = scanPageDOM(true);
      const interactiveEls = result.elements.filter(e => e.interactive);

      // @step Then the date input should receive exactly one ref
      expect(interactiveEls.length).toBe(1);

      // @step And the tree output should include the type, min, and max attributes
      expect(interactiveEls[0].attributes.type).toBe('date');
      expect(interactiveEls[0].attributes.min).toBe('2024-01-01');
      expect(interactiveEls[0].attributes.max).toBe('2025-12-31');
    });
  });

  describe('Scenario: Non-semantic search elements are detected as interactive', () => {
    it('should detect a search div with search-related classes', () => {
      // @step Given a page with a div having search-related class names and a data-action attribute
      clearBody();
      const div = document.createElement('div');
      div.className = 'search-icon magnify';
      div.setAttribute('data-action', 'toggle-search');
      div.textContent = '🔍';
      document.body.appendChild(div);

      // @step When I call browser_scan_page
      const result = scanPageDOM(true);
      const interactiveEls = result.elements.filter(e => e.interactive);

      // @step Then the search div should be detected as interactive and receive a ref
      expect(interactiveEls.length).toBeGreaterThanOrEqual(1);
      const searchEl = interactiveEls.find(e => e.name === '🔍');
      expect(searchEl).toBeDefined();

      // @step And the detection should work even without ARIA roles or onclick handlers
      expect(isSearchElement(div)).toBe(true);

      // Non-div/span elements are NOT detected by the search heuristic (per rule [3])
      const p = document.createElement('p');
      p.className = 'search-icon';
      expect(isSearchElement(p)).toBe(false);
    });
  });

  describe('Scenario: Small icon-sized elements with metadata are detected as interactive', () => {
    it('should detect a 24x24 icon with aria-label as interactive', () => {
      // @step Given a page with a 24x24 pixel span having a class name and aria-label
      clearBody();
      const span = document.createElement('span');
      span.className = 'close-btn';
      span.setAttribute('aria-label', 'Close');
      span.textContent = '✕';
      span.style.width = '24px';
      span.style.height = '24px';
      span.style.display = 'inline-block';
      document.body.appendChild(span);

      // @step When I call browser_scan_page
      // jsdom has no layout engine — getBoundingClientRect() returns zeros,
      // making the icon-size codepath unreachable via scanPageDOM.
      // We test the heuristic function directly with a mock rect to verify
      // the detection logic that runs inside scanPageDOM in a real browser.
      const mockRect = { width: 24, height: 24 };

      // @step Then the icon-sized element should be detected as interactive and receive a ref
      expect(isLikelyInteractiveIcon(span, mockRect as DOMRect)).toBe(true);
    });
  });

  describe('Scenario: Elements beyond icon-size range are not detected by icon heuristic', () => {
    it('should NOT detect a 60x60 element as an icon', () => {
      // @step Given a page with a 60x60 pixel span having a class name
      clearBody();
      const span = document.createElement('span');
      span.className = 'close-btn';
      span.textContent = '✕';
      document.body.appendChild(span);

      // @step When I call browser_scan_page
      // jsdom has no layout — test heuristic directly (see note in icon-size scenario above)
      const oversizedRect = { width: 60, height: 60 };

      // @step Then the oversized element should not be detected by the icon-size heuristic
      expect(isLikelyInteractiveIcon(span, oversizedRect as DOMRect)).toBe(
        false
      );

      // Also verify undersized elements are rejected (below 10px minimum)
      const tooSmall = { width: 5, height: 5 };
      expect(isLikelyInteractiveIcon(span, tooSmall as DOMRect)).toBe(false);
    });
  });

  describe('Scenario: Inert elements are skipped immediately', () => {
    it('should not include buttons inside inert containers', () => {
      // @step Given a page with a div having the inert attribute containing a button
      clearBody();
      const div = document.createElement('div');
      div.setAttribute('inert', '');
      const btn = document.createElement('button');
      btn.textContent = 'Disabled by inert';
      div.appendChild(btn);
      document.body.appendChild(div);

      // Also add a visible button outside
      const visibleBtn = document.createElement('button');
      visibleBtn.textContent = 'Active';
      document.body.appendChild(visibleBtn);

      // @step When I call browser_scan_page
      const result = scanPageDOM(true);

      // @step Then the button inside the inert container should not appear in the results
      const inertBtn = result.elements.find(
        e => e.name === 'Disabled by inert' && e.interactive
      );
      expect(inertBtn).toBeUndefined();

      // The non-inert button should still be present
      const activeBtn = result.elements.find(
        e => e.name === 'Active' && e.interactive
      );
      expect(activeBtn).toBeDefined();
    });

    it('should detect inert attribute via isInteractiveElement', () => {
      const btn = document.createElement('button');
      btn.textContent = 'Test';
      const container = document.createElement('div');
      container.setAttribute('inert', '');
      container.appendChild(btn);
      document.body.appendChild(container);

      expect(isInteractiveElement(btn)).toBe(false);
    });
  });

  describe('Scenario: Dynamic state classes are filtered for stable hashing', () => {
    it('should remove state classes and sort remaining', () => {
      // @step Given an element with classes including state-related classes like active, focus, and hover
      const classStr = 'btn active focus hover-highlight z-index primary';

      // @step When the dynamic class filter is applied
      const filtered = filterDynamicClasses(classStr);

      // @step Then only stable non-state classes should remain
      expect(filtered).toBe('btn primary z-index');

      // @step And the remaining classes should be sorted alphabetically
      const parts = filtered.split(' ').filter(Boolean);
      const sorted = [...parts].sort();
      expect(parts).toEqual(sorted);
    });

    it('should filter classes containing dynamic patterns', () => {
      expect(filterDynamicClasses('btn is-loading menu-expanded')).toBe('btn');
    });

    it('should return empty string when all classes are dynamic', () => {
      expect(filterDynamicClasses('active focus hover')).toBe('');
    });
  });
});
