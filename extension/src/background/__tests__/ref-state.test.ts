/**
 * Feature: spec/features/ref-state-management.feature
 *
 * This test file validates the acceptance criteria for LOCATE-003:
 * Ref State Management Module.
 *
 * Tests the ref-state module that stores scan results per-tab
 * and provides ref resolution for the scan→interact→verify workflow.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
  setTabScanState,
  getTabScanState,
  clearTabScanState,
  resolveRef,
  _resetForTesting,
} from '../ref-state';
import type { TabScanState, RefEntry } from '../ref-state';
import { createBrowserEventListeners } from '../browser-events';
import type { NotificationEnvelope } from '../browser-events';
import { createMockChromeTabs } from './helpers/mock-chrome-tabs';

/** Helper: creates a TabScanState with the given number of refs */
function createScanState(refCount: number): TabScanState {
  const refs = new Map<string, RefEntry>();
  const roles = ['button', 'textbox', 'link', 'checkbox', 'combobox'];
  const names = ['Login', 'Email', 'Home', 'Remember me', 'Country'];
  const selectors = [
    '#login-btn',
    '#email',
    'a[href="/home"]',
    '#remember',
    '#country',
  ];

  for (let i = 0; i < refCount; i++) {
    refs.set(`e${i + 1}`, {
      selector: selectors[i % selectors.length],
      role: roles[i % roles.length],
      name: names[i % names.length],
      frameId: 0,
    });
  }

  return {
    refs,
    treeText: `- ${roles[0]} "${names[0]}" [ref=e1]`,
    timestamp: Date.now(),
  };
}

describe('Feature: Ref State Management Module', () => {
  beforeEach(() => {
    _resetForTesting();
  });

  describe('Scenario: Store and retrieve scan state for a tab', () => {
    it('should store and return the exact refs, treeText, and timestamp', () => {
      // @step Given no scan state exists for tab 42
      const existing = getTabScanState(42);
      expect(existing).toBeUndefined();

      // @step When I store scan state for tab 42 with 3 refs and tree text "- button Login [ref=e1]"
      const treeText = '- button Login [ref=e1]';
      const timestamp = Date.now();
      const refs = new Map<string, RefEntry>();
      refs.set('e1', {
        selector: '#login-btn',
        role: 'button',
        name: 'Login',
        frameId: 0,
      });
      refs.set('e2', {
        selector: '#email',
        role: 'textbox',
        name: 'Email',
        frameId: 0,
      });
      refs.set('e3', {
        selector: 'a[href="/home"]',
        role: 'link',
        name: 'Home',
        frameId: 0,
      });
      const state: TabScanState = { refs, treeText, timestamp };
      setTabScanState(42, state);

      // @step Then getTabScanState for tab 42 should return the stored refs map with 3 entries
      const retrieved = getTabScanState(42);
      expect(retrieved).toBeDefined();
      expect(retrieved!.refs.size).toBe(3);

      // @step And the returned state should include the exact tree text and a valid timestamp
      expect(retrieved!.treeText).toBe(treeText);
      expect(retrieved!.timestamp).toBe(timestamp);
    });
  });

  describe('Scenario: Resolve a known ref to its entry', () => {
    it('should return the RefEntry for a known ref', () => {
      // @step Given tab 42 has scan state with ref "e2" mapped to selector "#email" role "textbox" name "Email"
      const refs = new Map<string, RefEntry>();
      refs.set('e2', {
        selector: '#email',
        role: 'textbox',
        name: 'Email',
        frameId: 0,
      });
      setTabScanState(42, { refs, treeText: '', timestamp: Date.now() });

      // @step When I resolve ref "e2" for tab 42
      const entry = resolveRef(42, 'e2');

      // @step Then I should receive the RefEntry with selector "#email" role "textbox" name "Email"
      expect(entry).toBeDefined();
      expect(entry!.selector).toBe('#email');
      expect(entry!.role).toBe('textbox');
      expect(entry!.name).toBe('Email');
    });
  });

  describe('Scenario: Resolve an unknown ref returns undefined', () => {
    it('should return undefined for an unknown ref key', () => {
      // @step Given tab 42 has scan state with ref "e2" mapped to selector "#email" role "textbox" name "Email"
      const refs = new Map<string, RefEntry>();
      refs.set('e2', {
        selector: '#email',
        role: 'textbox',
        name: 'Email',
        frameId: 0,
      });
      setTabScanState(42, { refs, treeText: '', timestamp: Date.now() });

      // @step When I resolve ref "e99" for tab 42
      const entry = resolveRef(42, 'e99');

      // @step Then I should receive undefined
      expect(entry).toBeUndefined();
    });
  });

  describe('Scenario: Resolve ref for unknown tab returns undefined', () => {
    it('should return undefined when tab has no scan state', () => {
      // @step Given no scan state exists for tab 999
      const existing = getTabScanState(999);
      expect(existing).toBeUndefined();

      // @step When I resolve ref "e1" for tab 999
      const entry = resolveRef(999, 'e1');

      // @step Then I should receive undefined
      expect(entry).toBeUndefined();
    });
  });

  describe('Scenario: Clear scan state for one tab without affecting others', () => {
    it('should only remove the specified tab state', () => {
      // @step Given tab 42 has scan state with 3 refs
      setTabScanState(42, createScanState(3));

      // @step And tab 99 has scan state with 2 refs
      setTabScanState(99, createScanState(2));

      // @step When I clear scan state for tab 42
      clearTabScanState(42);

      // @step Then getTabScanState for tab 42 should return undefined
      expect(getTabScanState(42)).toBeUndefined();

      // @step And getTabScanState for tab 99 should still return its 2 refs
      const tab99State = getTabScanState(99);
      expect(tab99State).toBeDefined();
      expect(tab99State!.refs.size).toBe(2);
    });
  });

  describe('Scenario: Get scan state for never-scanned tab returns undefined', () => {
    it('should return undefined without error for unscanned tabs', () => {
      // @step Given no scan state exists for tab 999
      // (no setup needed — tab 999 was never scanned)

      // @step When I get scan state for tab 999
      const state = getTabScanState(999);

      // @step Then I should receive undefined without any error
      expect(state).toBeUndefined();
    });
  });

  describe('Scenario: Navigation event invalidates scan state', () => {
    it('should clear scan state when tab navigates to a new URL', () => {
      // @step Given tab 42 has scan state with 3 refs
      setTabScanState(42, createScanState(3));
      expect(getTabScanState(42)).toBeDefined();

      // @step And browser event listeners are registered with ref state invalidation
      const mockTabs = createMockChromeTabs();
      const notifications: NotificationEnvelope[] = [];
      createBrowserEventListeners({
        tabs: mockTabs,
        onNotify: envelope => {
          notifications.push(envelope);
        },
      });

      // @step When the tab 42 onUpdated event fires with a new URL
      mockTabs._fireUpdated(
        42,
        { url: 'https://example.com/new-page' },
        { id: 42, url: 'https://example.com/new-page', title: 'New Page' }
      );

      // @step Then getTabScanState for tab 42 should return undefined
      expect(getTabScanState(42)).toBeUndefined();
    });
  });

  describe('Scenario: Tab close event cleans up scan state', () => {
    it('should clear scan state when tab is closed', () => {
      // @step Given tab 42 has scan state with 3 refs
      setTabScanState(42, createScanState(3));
      expect(getTabScanState(42)).toBeDefined();

      // @step And browser event listeners are registered with ref state invalidation
      const mockTabs = createMockChromeTabs();
      const notifications: NotificationEnvelope[] = [];
      createBrowserEventListeners({
        tabs: mockTabs,
        onNotify: envelope => {
          notifications.push(envelope);
        },
      });

      // @step When the tab 42 onRemoved event fires
      mockTabs._fireRemoved(42, {});

      // @step Then getTabScanState for tab 42 should return undefined
      expect(getTabScanState(42)).toBeUndefined();
    });
  });
});
