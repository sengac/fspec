/**
 * Feature: spec/features/webmcp-chrome-extension.feature
 *
 * This test file validates the acceptance criteria for EXT-007:
 * Bidirectional Browser Event Notifications.
 *
 * Tests the browser-events module that captures Chrome browser events
 * and forwards them as MCP notifications via the message router.
 */

import { describe, it, expect, vi } from 'vitest';
import { createBrowserEventListeners } from '../browser-events';
import type {
  BrowserEventListenerOptions,
  NotificationEnvelope,
  ChromeTabsEvents,
} from '../browser-events';
import { MESSAGE_TYPES } from '../../types';

/** Mock chrome.tabs with fire helpers for test control */
interface MockChromeTabs extends ChromeTabsEvents {
  _fireUpdated: (
    tabId: number,
    changeInfo: Record<string, unknown>,
    tab: Record<string, unknown>
  ) => void;
  _fireCreated: (tab: Record<string, unknown>) => void;
  _fireRemoved: (tabId: number, removeInfo: Record<string, unknown>) => void;
}

/** Mock tool registry with test helpers */
interface MockToolRegistry {
  getByTab: (tabId: number) => Array<{ name: string; tabId?: number }>;
  unregister: ReturnType<typeof vi.fn>;
  _addTool: (name: string, tabId: number) => void;
}

/**
 * Helper: creates a mock chrome.tabs-like object with event emitters.
 */
function createMockChromeTabs(): MockChromeTabs {
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

/**
 * Helper: creates a mock tool registry.
 */
function createMockToolRegistry(): MockToolRegistry {
  const tools = new Map<string, { name: string; tabId?: number }>();
  return {
    getByTab: (tabId: number) => {
      return Array.from(tools.values()).filter(t => t.tabId === tabId);
    },
    unregister: vi.fn<(name: string) => void>((name: string) => {
      tools.delete(name);
    }),
    _addTool: (name: string, tabId: number) => {
      tools.set(name, { name, tabId });
    },
  };
}

/** Helper: sets up listeners with standard options, returns onNotify spy */
function setupListeners(
  mockTabs: MockChromeTabs,
  overrides?: {
    toolRegistry?: MockToolRegistry;
    onToolsChanged?: () => void;
  }
): ReturnType<typeof vi.fn<(envelope: NotificationEnvelope) => void>> {
  const onNotify = vi.fn<(envelope: NotificationEnvelope) => void>();
  createBrowserEventListeners({
    tabs: mockTabs,
    onNotify,
    toolRegistry:
      overrides?.toolRegistry as BrowserEventListenerOptions['toolRegistry'],
    onToolsChanged: overrides?.onToolsChanged,
  });
  return onNotify;
}

describe('Feature: fspec Browser Agent - Browser Event Notifications', () => {
  describe('Scenario: Receive navigation event when user navigates to new URL', () => {
    it('should fire notifications/browser/navigation with tabId, url, and title', () => {
      // @step Given the agent has an active MCP connection to the extension
      const mockTabs = createMockChromeTabs();

      // @step And a GET /mcp SSE stream is open for notifications
      const onNotify = setupListeners(mockTabs);

      // @step When the user clicks a link that navigates tab 123 to "https://new-page.com"
      mockTabs._fireUpdated(
        123,
        { url: 'https://new-page.com' },
        {
          id: 123,
          url: 'https://new-page.com',
          title: 'New Page',
        }
      );

      // @step Then the extension fires an MCP notification with method "notifications/browser/navigation"
      expect(onNotify).toHaveBeenCalledWith(
        expect.objectContaining({
          notification: expect.objectContaining({
            jsonrpc: '2.0',
            method: 'notifications/browser/navigation',
          }),
        })
      );

      // @step And the notification params include tabId 123, url "https://new-page.com", and title "New Page"
      const call = onNotify.mock.calls[0][0];
      expect(call.notification.params).toEqual(
        expect.objectContaining({
          tabId: 123,
          url: 'https://new-page.com',
          title: 'New Page',
        })
      );

      // @step And the agent receives the notification via the SSE stream
      // SSE delivery verified at MCP server level; notification format validated above
      expect(onNotify).toHaveBeenCalledTimes(1);
    });
  });

  describe('Scenario: Receive tab created event when user opens a new tab', () => {
    it('should fire notifications/browser/tab_created with tabId and url', () => {
      // @step Given the agent has an active MCP connection to the extension
      const mockTabs = createMockChromeTabs();
      const onNotify = setupListeners(mockTabs);

      // @step When the user opens a new browser tab
      mockTabs._fireCreated({
        id: 456,
        url: 'chrome://newtab',
        title: 'New Tab',
      });

      // @step Then the extension fires an MCP notification with method "notifications/browser/tab_created"
      expect(onNotify).toHaveBeenCalledWith(
        expect.objectContaining({
          notification: expect.objectContaining({
            jsonrpc: '2.0',
            method: 'notifications/browser/tab_created',
          }),
        })
      );

      // @step Given a GET /mcp SSE stream is open for notifications
      // SSE stream setup is handled by MCP server; we verify notification format

      // @step Then the notification params include the new tabId and url
      const call = onNotify.mock.calls[0][0];
      expect(call.notification.params).toEqual(
        expect.objectContaining({
          tabId: 456,
          url: 'chrome://newtab',
          title: 'New Tab',
        })
      );

      // @step Then the agent receives the notification via the SSE stream
      expect(onNotify).toHaveBeenCalledTimes(1);
    });
  });

  describe('Scenario: Receive tab closed event when user closes a tab', () => {
    it('should fire notifications/browser/tab_closed with tabId', () => {
      // @step Given the agent has an active MCP connection to the extension
      const mockTabs = createMockChromeTabs();
      const onNotify = setupListeners(mockTabs);

      // @step When the user closes browser tab 123
      mockTabs._fireRemoved(123, { windowId: 1, isWindowClosing: false });

      // @step Then the extension fires an MCP notification with method "notifications/browser/tab_closed"
      expect(onNotify).toHaveBeenCalledWith(
        expect.objectContaining({
          notification: expect.objectContaining({
            jsonrpc: '2.0',
            method: 'notifications/browser/tab_closed',
          }),
        })
      );

      // @step Given a GET /mcp SSE stream is open for notifications
      // SSE stream setup is handled by MCP server; we verify notification format

      // @step Then the notification params include tabId 123
      const call = onNotify.mock.calls[0][0];
      expect(call.notification.params).toEqual(
        expect.objectContaining({
          tabId: 123,
        })
      );

      // @step Then the agent receives the notification via the SSE stream
      expect(onNotify).toHaveBeenCalledTimes(1);
    });
  });

  describe('Scenario: Receive load complete event when page finishes loading', () => {
    it('should fire notifications/browser/load_complete when tab status is complete', () => {
      // @step Given the agent has an active MCP connection to the extension
      const mockTabs = createMockChromeTabs();
      const onNotify = setupListeners(mockTabs);

      // @step When tab 123 finishes loading the page at "https://example.com"
      mockTabs._fireUpdated(
        123,
        { status: 'complete' },
        {
          id: 123,
          url: 'https://example.com',
          title: 'Example',
        }
      );

      // @step Then the extension fires an MCP notification with method "notifications/browser/load_complete"
      expect(onNotify).toHaveBeenCalledWith(
        expect.objectContaining({
          notification: expect.objectContaining({
            jsonrpc: '2.0',
            method: 'notifications/browser/load_complete',
          }),
        })
      );

      // @step Given a GET /mcp SSE stream is open for notifications
      // SSE stream setup is handled by MCP server; we verify notification format

      // @step Then the notification params include tabId 123, url "https://example.com", and title
      const call = onNotify.mock.calls[0][0];
      expect(call.notification.params).toEqual(
        expect.objectContaining({
          tabId: 123,
          url: 'https://example.com',
          title: 'Example',
        })
      );

      // @step Then the agent receives the notification via the SSE stream
      expect(onNotify).toHaveBeenCalledTimes(1);
    });
  });

  describe('Scenario: Receive tool list changed notification when WebMCP tools are discovered', () => {
    it('should verify TOOLS_CHANGED notification pathway exists in message types', () => {
      // @step Given the agent has an active MCP connection to the extension
      // @step And a GET /mcp SSE stream is open for notifications
      // notifications/tools/list_changed is delivered via the TOOLS_CHANGED → NOTIFICATION
      // pathway already implemented in message-router.ts and mcp-server.mjs.

      // @step When a website registers a new WebMCP tool via navigator.modelContext.registerTool
      // The content script sends TOOL_REGISTERED; message-router calls notifyToolsChanged()
      expect(MESSAGE_TYPES.TOOL_REGISTERED).toBe(
        'FSPEC_WEBMCP_TOOL_REGISTERED'
      );

      // @step Then the extension sends a "notifications/tools/list_changed" MCP notification via SSE
      // message-router sends TOOLS_CHANGED to native host; mcp-server translates to SSE notification
      expect(MESSAGE_TYPES.TOOLS_CHANGED).toBe('TOOLS_CHANGED');
      expect(MESSAGE_TYPES.NOTIFICATION).toBe('NOTIFICATION');

      // @step And the agent's next tools/list call includes the newly discovered WebMCP tool
      // Full pathway tested in extension-webmcp-discovery.test.ts and native-messaging-host.test.ts
    });
  });

  describe('Scenario: Closing tab with WebMCP tools triggers both tab closed and tools changed notifications', () => {
    it('should unregister tools and fire both tab_closed and trigger tools changed', () => {
      // @step Given the agent has an active MCP connection to the extension
      const mockTabs = createMockChromeTabs();
      const mockToolRegistry = createMockToolRegistry();
      const onToolsChanged = vi.fn();

      // @step Given a GET /mcp SSE stream is open for notifications
      // SSE stream is managed by MCP server
      const onNotify = setupListeners(mockTabs, {
        toolRegistry: mockToolRegistry,
        onToolsChanged,
      });

      // @step Given tab 456 has WebMCP tools registered
      mockToolRegistry._addTool('webmcp__example-com__searchFlights', 456);
      mockToolRegistry._addTool('webmcp__example-com__bookFlight', 456);

      // @step When the user closes browser tab 456
      mockTabs._fireRemoved(456, { windowId: 1, isWindowClosing: false });

      // @step Then the WebMCP tools from tab 456 are unregistered from the tool registry
      expect(mockToolRegistry.unregister).toHaveBeenCalledWith(
        'webmcp__example-com__searchFlights'
      );
      expect(mockToolRegistry.unregister).toHaveBeenCalledWith(
        'webmcp__example-com__bookFlight'
      );

      // @step Then the agent receives a "notifications/browser/tab_closed" notification
      expect(onNotify).toHaveBeenCalledWith(
        expect.objectContaining({
          notification: expect.objectContaining({
            method: 'notifications/browser/tab_closed',
            params: expect.objectContaining({ tabId: 456 }),
          }),
        })
      );

      // @step Then the agent receives a "notifications/tools/list_changed" notification
      expect(onToolsChanged).toHaveBeenCalled();
    });
  });

  describe('Edge case: simultaneous URL change and load complete', () => {
    it('should fire both navigation and load_complete when changeInfo has url and status:complete', () => {
      const mockTabs = createMockChromeTabs();
      const onNotify = setupListeners(mockTabs);

      // Chrome can fire onUpdated with both url and status:'complete' in one event
      mockTabs._fireUpdated(
        789,
        { url: 'https://instant.com', status: 'complete' },
        {
          id: 789,
          url: 'https://instant.com',
          title: 'Instant Load',
        }
      );

      expect(onNotify).toHaveBeenCalledTimes(2);

      const methods = onNotify.mock.calls.map(
        call => (call[0] as NotificationEnvelope).notification.method
      );
      expect(methods).toContain('notifications/browser/navigation');
      expect(methods).toContain('notifications/browser/load_complete');

      // Both should reference the same tab and URL
      for (const call of onNotify.mock.calls) {
        const envelope = call[0] as NotificationEnvelope;
        expect(envelope.notification.params).toEqual(
          expect.objectContaining({
            tabId: 789,
            url: 'https://instant.com',
            title: 'Instant Load',
          })
        );
      }
    });
  });
});
