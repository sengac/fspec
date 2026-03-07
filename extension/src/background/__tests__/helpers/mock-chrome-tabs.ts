/**
 * Shared test helper: mock chrome.tabs with event emitters.
 *
 * Used by browser-events.test.ts and ref-state.test.ts to avoid
 * duplicating the ~60-line mock factory.
 */

import type { ChromeTabsEvents } from '../../browser-events';

/** Mock chrome.tabs with fire helpers for test control */
export interface MockChromeTabs extends ChromeTabsEvents {
  _fireUpdated: (
    tabId: number,
    changeInfo: Record<string, unknown>,
    tab: Record<string, unknown>
  ) => void;
  _fireCreated: (tab: Record<string, unknown>) => void;
  _fireRemoved: (tabId: number, removeInfo: Record<string, unknown>) => void;
}

/** Creates a mock chrome.tabs with event emitters for all three event types */
export function createMockChromeTabs(): MockChromeTabs {
  const onUpdatedListeners: Array<
    (
      tabId: number,
      changeInfo: Record<string, unknown>,
      tab: Record<string, unknown>
    ) => void
  > = [];
  const onCreatedListeners: Array<(tab: Record<string, unknown>) => void> = [];
  const onRemovedListeners: Array<
    (tabId: number, removeInfo: Record<string, unknown>) => void
  > = [];

  return {
    onUpdated: {
      addListener: (
        cb: (
          tabId: number,
          changeInfo: Record<string, unknown>,
          tab: Record<string, unknown>
        ) => void
      ) => {
        onUpdatedListeners.push(cb);
      },
    },
    onCreated: {
      addListener: (cb: (tab: Record<string, unknown>) => void) => {
        onCreatedListeners.push(cb);
      },
    },
    onRemoved: {
      addListener: (
        cb: (tabId: number, removeInfo: Record<string, unknown>) => void
      ) => {
        onRemovedListeners.push(cb);
      },
    },
    _fireUpdated: (
      tabId: number,
      changeInfo: Record<string, unknown>,
      tab: Record<string, unknown>
    ) => {
      for (const listener of onUpdatedListeners) {
        listener(tabId, changeInfo, tab);
      }
    },
    _fireCreated: (tab: Record<string, unknown>) => {
      for (const listener of onCreatedListeners) {
        listener(tab);
      }
    },
    _fireRemoved: (tabId: number, removeInfo: Record<string, unknown>) => {
      for (const listener of onRemovedListeners) {
        listener(tabId, removeInfo);
      }
    },
  };
}
