/**
 * fspec Browser Agent - Browser Event Listeners
 *
 * Captures Chrome browser events (tab navigation, creation, removal,
 * load completion) and forwards them as MCP notifications via the
 * message router's forwardNotification pathway.
 *
 * Uses dependency injection for chrome.tabs so listeners are testable
 * without the Chrome runtime.
 *
 * Implemented by: EXT-007
 */

import { clearTabScanState } from './ref-state';

/** Minimal chrome.tabs event interface for dependency injection */
export interface ChromeTabsEvents {
  onUpdated: {
    addListener: (
      cb: (
        tabId: number,
        changeInfo: Record<string, unknown>,
        tab: Record<string, unknown>
      ) => void
    ) => void;
  };
  onCreated: {
    addListener: (cb: (tab: Record<string, unknown>) => void) => void;
  };
  onRemoved: {
    addListener: (
      cb: (tabId: number, removeInfo: Record<string, unknown>) => void
    ) => void;
  };
}

/** Tool registry interface for tab cleanup on close */
export interface BrowserEventsToolRegistry {
  getByTab: (tabId: number) => Array<{ name: string; tabId?: number }>;
  unregister: (name: string) => void;
}

/** Notification envelope passed to onNotify */
export interface NotificationEnvelope {
  notification: {
    jsonrpc: '2.0';
    method: string;
    params?: Record<string, unknown>;
  };
}

export interface BrowserEventListenerOptions {
  tabs: ChromeTabsEvents;
  onNotify: (envelope: NotificationEnvelope) => void;
  toolRegistry?: BrowserEventsToolRegistry;
  onToolsChanged?: () => void;
}

/** Build a JSON-RPC 2.0 notification envelope */
function buildNotification(
  method: string,
  params?: Record<string, unknown>
): NotificationEnvelope {
  return {
    notification: {
      jsonrpc: '2.0',
      method,
      params,
    },
  };
}

/**
 * Creates and registers Chrome browser event listeners that produce
 * MCP notification envelopes.
 *
 * Events:
 * - notifications/browser/navigation  — tab URL changed
 * - notifications/browser/load_complete — tab finished loading (status: 'complete')
 * - notifications/browser/tab_created  — new tab opened
 * - notifications/browser/tab_closed   — tab closed (+ tool cleanup)
 */
export function createBrowserEventListeners(
  options: BrowserEventListenerOptions
): void {
  const { tabs, onNotify, toolRegistry, onToolsChanged } = options;

  // --- chrome.tabs.onUpdated ---
  // Fires for URL changes (navigation) and status changes (load_complete).
  tabs.onUpdated.addListener(
    (
      tabId: number,
      changeInfo: Record<string, unknown>,
      tab: Record<string, unknown>
    ) => {
      // Navigation: changeInfo contains a new url
      if (typeof changeInfo.url === 'string') {
        clearTabScanState(tabId);
        onNotify(
          buildNotification('notifications/browser/navigation', {
            tabId,
            url: changeInfo.url,
            title: (tab.title as string) ?? '',
          })
        );
      }

      // Load complete: status transitioned to 'complete'
      if (changeInfo.status === 'complete') {
        onNotify(
          buildNotification('notifications/browser/load_complete', {
            tabId,
            url: (tab.url as string) ?? '',
            title: (tab.title as string) ?? '',
          })
        );
      }
    }
  );

  // --- chrome.tabs.onCreated ---
  tabs.onCreated.addListener((tab: Record<string, unknown>) => {
    onNotify(
      buildNotification('notifications/browser/tab_created', {
        tabId: tab.id as number,
        url: (tab.url as string) ?? '',
        title: (tab.title as string) ?? '',
      })
    );
  });

  // --- chrome.tabs.onRemoved ---
  tabs.onRemoved.addListener((tabId: number) => {
    // Clear ref state for closed tab
    clearTabScanState(tabId);

    // Clean up any WebMCP tools registered from this tab
    if (toolRegistry) {
      const tabTools = toolRegistry.getByTab(tabId);
      for (const tool of tabTools) {
        toolRegistry.unregister(tool.name);
      }
      if (tabTools.length > 0 && onToolsChanged) {
        onToolsChanged();
      }
    }

    onNotify(buildNotification('notifications/browser/tab_closed', { tabId }));
  });
}
