/**
 * Feature: spec/features/webmcp-chrome-extension.feature
 *
 * This test file validates the acceptance criteria for EXT-005:
 * Native Browser Control Tools.
 * Scenarios map directly to Gherkin scenarios tagged @EXT-005.
 *
 * Chrome APIs are mocked since these run in Vitest, not a real browser.
 * The browser-tools module uses dependency injection for all Chrome APIs.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { ToolRegistryAPI } from '../../../extension/src/background/tool-registry';
import type { NativeConnectionAPI } from '../../../extension/src/background/native-connection';
import type { MessageRouterAPI } from '../../../extension/src/background/message-router';

/**
 * Mock Chrome APIs for testing browser tools.
 * These mirror the DI interfaces that browser-tools.ts will define.
 */
interface MockTab {
  id: number;
  url: string;
  title: string;
  active: boolean;
  windowId: number;
}

interface MockInjectionResult {
  result: unknown;
}

function createMockChromeTabs() {
  return {
    query: vi.fn<(query: Record<string, unknown>) => Promise<MockTab[]>>(),
    update:
      vi.fn<
        (tabId: number, props: Record<string, unknown>) => Promise<MockTab>
      >(),
    remove: vi.fn<(tabId: number) => Promise<void>>(),
    captureVisibleTab:
      vi.fn<
        (windowId: number, opts: Record<string, unknown>) => Promise<string>
      >(),
    goBack: vi.fn<(tabId: number) => Promise<void>>(),
    goForward: vi.fn<(tabId: number) => Promise<void>>(),
    get: vi.fn<(tabId: number) => Promise<MockTab>>(),
    sendMessage: vi.fn(),
    onUpdated: {
      addListener: vi.fn(),
      removeListener: vi.fn(),
    },
  };
}

function createMockChromeScripting() {
  return {
    executeScript:
      vi.fn<
        (injection: Record<string, unknown>) => Promise<MockInjectionResult[]>
      >(),
  };
}

function createMockChromeWindows() {
  return {
    update:
      vi.fn<
        (windowId: number, props: Record<string, unknown>) => Promise<void>
      >(),
  };
}

interface MockPort {
  name: string;
  postMessage: ReturnType<typeof vi.fn>;
  onMessage: {
    addListener: ReturnType<typeof vi.fn>;
    removeListener: ReturnType<typeof vi.fn>;
  };
  onDisconnect: {
    addListener: ReturnType<typeof vi.fn>;
    removeListener: ReturnType<typeof vi.fn>;
  };
  disconnect: ReturnType<typeof vi.fn>;
}

function createMockPort(name: string): MockPort {
  return {
    name,
    postMessage: vi.fn(),
    onMessage: { addListener: vi.fn(), removeListener: vi.fn() },
    onDisconnect: { addListener: vi.fn(), removeListener: vi.fn() },
    disconnect: vi.fn(),
  };
}

function createMockChromeRuntime() {
  return {
    connectNative: vi.fn(),
    onMessage: { addListener: vi.fn(), removeListener: vi.fn() },
    sendMessage: vi.fn(),
    lastError: null as { message: string } | null,
  };
}

const activeTab: MockTab = {
  id: 1,
  url: 'https://current-page.com',
  title: 'Current Page',
  active: true,
  windowId: 1,
};

describe('Feature: fspec Browser Agent Chrome Extension — EXT-005 Native Browser Control Tools', () => {
  let mockTabs: ReturnType<typeof createMockChromeTabs>;
  let mockScripting: ReturnType<typeof createMockChromeScripting>;
  let mockWindows: ReturnType<typeof createMockChromeWindows>;
  let mockRuntime: ReturnType<typeof createMockChromeRuntime>;
  let mockPort: MockPort;

  beforeEach(() => {
    mockTabs = createMockChromeTabs();
    mockScripting = createMockChromeScripting();
    mockWindows = createMockChromeWindows();
    mockRuntime = createMockChromeRuntime();
    mockPort = createMockPort('com.fspec.browser.agent');
    mockRuntime.connectNative.mockReturnValue(mockPort);

    // Default: active tab query returns our activeTab
    mockTabs.query.mockResolvedValue([activeTab]);

    vi.clearAllMocks();

    // Re-set defaults after clear
    mockRuntime.connectNative.mockReturnValue(mockPort);
    mockTabs.query.mockResolvedValue([activeTab]);
  });

  describe('Scenario: Navigate browser tab to URL', () => {
    it('should navigate the active tab to the specified URL and return the final URL and title after load completes', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step When the agent calls mcp__ext__browser_navigate with url "https://example.com"
      const navigatedTab: MockTab = {
        id: 1,
        url: 'https://example.com',
        title: 'Example Domain',
        active: true,
        windowId: 1,
      };
      mockTabs.update.mockResolvedValue(navigatedTab);
      mockTabs.get.mockResolvedValue(navigatedTab);

      // Mock onUpdated: when addListener is called, immediately fire the callback
      // simulating the tab completing its load
      mockTabs.onUpdated.addListener.mockImplementation(
        (
          callback: (
            tabId: number,
            changeInfo: { status?: string },
            tab: MockTab
          ) => void
        ) => {
          // Simulate async load completion
          setTimeout(() => {
            callback(1, { status: 'complete' }, navigatedTab);
          }, 5);
        }
      );

      const handler = browserTools.getHandler('browser_navigate');
      expect(handler).toBeDefined();
      const result = await handler!({ url: 'https://example.com' });

      // @step Then the extension navigates the active tab to "https://example.com"
      expect(mockTabs.update).toHaveBeenCalledWith(
        expect.any(Number),
        expect.objectContaining({ url: 'https://example.com' })
      );

      // Verify it waited for onUpdated
      expect(mockTabs.onUpdated.addListener).toHaveBeenCalled();
      expect(mockTabs.onUpdated.removeListener).toHaveBeenCalled();

      // @step And the tool returns the final URL after any redirects
      expect(result.content[0].text).toContain('https://example.com');

      // @step And the tool returns the page title
      expect(result.content[0].text).toContain('Example Domain');
    });
  });

  describe('Scenario: Capture full-page screenshot', () => {
    it('should capture a screenshot and return base64-encoded PNG', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And tab 123 is displaying a web page
      const tab123: MockTab = {
        id: 123,
        url: 'https://page.com',
        title: 'Page',
        active: true,
        windowId: 1,
      };
      mockTabs.get.mockResolvedValue(tab123);

      // @step When the agent calls mcp__ext__browser_screenshot with tabId 123 and fullPage true
      mockTabs.captureVisibleTab.mockResolvedValue(
        'data:image/png;base64,iVBORFAKEDATA'
      );
      const handler = browserTools.getHandler('browser_screenshot');
      expect(handler).toBeDefined();
      const result = await handler!({ tabId: 123, fullPage: true });

      // @step Then the extension captures a screenshot using chrome.tabs.captureVisibleTab
      expect(mockTabs.captureVisibleTab).toHaveBeenCalled();

      // @step And the tool returns a base64-encoded PNG image
      expect(result.content).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            type: 'image',
            mimeType: 'image/png',
          }),
        ])
      );
    });
  });

  describe('Scenario: List all open browser tabs', () => {
    it('should return a list of all tabs with their IDs, URLs, and titles', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the browser has multiple tabs open
      const multipleTabs: MockTab[] = [
        {
          id: 1,
          url: 'https://example.com',
          title: 'Example',
          active: true,
          windowId: 1,
        },
        {
          id: 2,
          url: 'about:blank',
          title: 'New Tab',
          active: false,
          windowId: 1,
        },
      ];
      mockTabs.query.mockResolvedValue(multipleTabs);

      // @step When the agent calls mcp__ext__browser_list_tabs
      const handler = browserTools.getHandler('browser_list_tabs');
      expect(handler).toBeDefined();
      const result = await handler!({});

      // @step Then the tool returns a list of all open tabs with their IDs, URLs, and titles
      const parsed = JSON.parse(result.content[0].text);
      expect(parsed).toHaveLength(2);
      expect(parsed[0]).toEqual(
        expect.objectContaining({
          id: 1,
          url: 'https://example.com',
          title: 'Example',
          active: true,
        })
      );
      expect(parsed[1]).toEqual(
        expect.objectContaining({
          id: 2,
          url: 'about:blank',
          title: 'New Tab',
          active: false,
        })
      );
    });
  });

  describe('Scenario: Execute JavaScript in a browser tab', () => {
    it('should execute the script and return the result', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const mockUserScripts = {
        configureWorld: vi.fn().mockResolvedValue(undefined),
        execute: vi.fn().mockResolvedValue([{ result: 'My Page Title' }]),
      };
      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
        userScripts: mockUserScripts,
      });

      // @step When the agent calls mcp__ext__browser_execute_script with code "document.title"
      const handler = browserTools.getHandler('browser_execute_script');
      expect(handler).toBeDefined();
      const result = await handler!({ code: 'document.title' });

      // @step Then the extension executes the script in the active tab via userScripts API
      expect(mockUserScripts.execute).toHaveBeenCalledWith(
        expect.objectContaining({
          target: expect.objectContaining({ tabId: expect.any(Number) }),
          world: 'USER_SCRIPT',
        })
      );

      // @step And the tool returns the script result
      expect(result.content[0].text).toContain('My Page Title');
    });
  });

  describe('Scenario: Switch to a specific browser tab', () => {
    it('should activate the tab and focus its window', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And tab 42 exists in the browser
      const tab42: MockTab = {
        id: 42,
        url: 'https://tab42.com',
        title: 'Tab 42',
        active: false,
        windowId: 2,
      };
      mockTabs.update.mockResolvedValue({ ...tab42, active: true });
      mockTabs.get.mockResolvedValue(tab42);

      // @step When the agent calls mcp__ext__browser_switch_tab with tabId 42
      const handler = browserTools.getHandler('browser_switch_tab');
      expect(handler).toBeDefined();
      const result = await handler!({ tabId: 42 });

      // @step Then the extension activates tab 42 and focuses its window
      expect(mockTabs.update).toHaveBeenCalledWith(42, { active: true });
      expect(mockWindows.update).toHaveBeenCalledWith(tab42.windowId, {
        focused: true,
      });

      // @step And the tool returns confirmation with the tab info
      const parsed = JSON.parse(result.content[0].text);
      expect(parsed).toEqual(expect.objectContaining({ tabId: 42 }));
    });
  });

  describe('Scenario: Close a browser tab', () => {
    it('should close the tab and return confirmation with its URL', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And tab 42 is open with url "https://example.com"
      const tab42: MockTab = {
        id: 42,
        url: 'https://example.com',
        title: 'Example',
        active: false,
        windowId: 1,
      };
      mockTabs.get.mockResolvedValue(tab42);
      mockTabs.remove.mockResolvedValue(undefined);

      // @step When the agent calls mcp__ext__browser_close_tab with tabId 42
      const handler = browserTools.getHandler('browser_close_tab');
      expect(handler).toBeDefined();
      const result = await handler!({ tabId: 42 });

      // @step Then the extension closes tab 42
      expect(mockTabs.remove).toHaveBeenCalledWith(42);

      // @step And the tool returns confirmation with the closed tab's URL
      const parsed = JSON.parse(result.content[0].text);
      expect(parsed).toEqual(
        expect.objectContaining({
          closed: true,
          tabId: 42,
          url: 'https://example.com',
        })
      );
    });
  });

  describe('Scenario: Get page content as text', () => {
    it('should return the page title, URL, and innerText content', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the active tab is displaying a web page
      // (default mock active tab is set up in beforeEach)

      // @step When the agent calls mcp__ext__browser_get_page_content with format "text"
      mockScripting.executeScript.mockResolvedValue([
        {
          result: {
            title: 'Current Page',
            url: 'https://current-page.com',
            content: 'This is the page text content',
          },
        },
      ]);
      const handler = browserTools.getHandler('browser_get_page_content');
      expect(handler).toBeDefined();
      const result = await handler!({ format: 'text' });

      // @step Then the tool returns the page title, URL, and inner text content
      const parsed = JSON.parse(result.content[0].text);
      expect(parsed.title).toBe('Current Page');
      expect(parsed.url).toBe('https://current-page.com');
      expect(parsed.content).toContain('page text content');
    });
  });

  describe('Scenario: Get page content as HTML', () => {
    it('should return the page title, URL, and outerHTML content', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the active tab is displaying a web page
      // (default mock active tab is set up in beforeEach)

      // @step When the agent calls mcp__ext__browser_get_page_content with format "html"
      mockScripting.executeScript.mockResolvedValue([
        {
          result: {
            title: 'Current Page',
            url: 'https://current-page.com',
            content: '<html><body>Hello</body></html>',
          },
        },
      ]);
      const handler = browserTools.getHandler('browser_get_page_content');
      expect(handler).toBeDefined();
      const result = await handler!({ format: 'html' });

      // @step Then the tool returns the page title, URL, and outer HTML content
      const parsed = JSON.parse(result.content[0].text);
      expect(parsed.title).toBe('Current Page');
      expect(parsed.url).toBe('https://current-page.com');
      expect(parsed.content).toContain('<html>');
    });
  });

  describe('Scenario: Click an element on the page', () => {
    it('should find and click the element matching the selector', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the active tab contains an element matching selector "#submit-btn"
      mockScripting.executeScript.mockResolvedValue([
        {
          result: { clicked: true, selector: '#submit-btn' },
        },
      ]);

      // @step When the agent calls mcp__ext__browser_click_element with selector "#submit-btn"
      const handler = browserTools.getHandler('browser_click_element');
      expect(handler).toBeDefined();
      const result = await handler!({ selector: '#submit-btn' });

      // @step Then the extension clicks the element matching the selector
      expect(mockScripting.executeScript).toHaveBeenCalled();

      // @step And the tool returns confirmation that the element was clicked
      const parsed = JSON.parse(result.content[0].text);
      expect(parsed).toEqual(
        expect.objectContaining({ clicked: true, selector: '#submit-btn' })
      );
    });
  });

  describe('Scenario: Click element fails when selector not found', () => {
    it('should return an error indicating the element was not found', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the active tab does not contain an element matching selector "#nonexistent"
      mockScripting.executeScript.mockResolvedValue([
        {
          result: { error: 'Element not found: #nonexistent' },
        },
      ]);

      // @step When the agent calls mcp__ext__browser_click_element with selector "#nonexistent"
      const handler = browserTools.getHandler('browser_click_element');
      expect(handler).toBeDefined();
      const result = await handler!({ selector: '#nonexistent' });

      // @step Then the tool returns an error indicating the element was not found
      expect(result.isError).toBe(true);
      expect(result.content[0].text).toContain('Element not found');
      expect(result.content[0].text).toContain('#nonexistent');
    });
  });

  describe('Scenario: Fill a form field on the page', () => {
    it('should set the input value and dispatch events', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step And the active tab contains an input element matching selector "#email"
      mockScripting.executeScript.mockResolvedValue([
        {
          result: {
            filled: true,
            selector: '#email',
            value: 'test@example.com',
          },
        },
      ]);

      // @step When the agent calls mcp__ext__browser_fill_form with selector "#email" and value "test@example.com"
      const handler = browserTools.getHandler('browser_fill_form');
      expect(handler).toBeDefined();
      const result = await handler!({
        selector: '#email',
        value: 'test@example.com',
      });

      // @step Then the extension sets the input value and dispatches input and change events
      expect(mockScripting.executeScript).toHaveBeenCalled();

      // @step And the tool returns confirmation with the selector and value
      const parsed = JSON.parse(result.content[0].text);
      expect(parsed).toEqual(
        expect.objectContaining({
          filled: true,
          selector: '#email',
          value: 'test@example.com',
        })
      );
    });
  });

  describe('Scenario: Navigate browser history backward', () => {
    it('should navigate the active tab back in history', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step When the agent calls mcp__ext__browser_go_back
      mockTabs.goBack.mockResolvedValue(undefined);
      const handler = browserTools.getHandler('browser_go_back');
      expect(handler).toBeDefined();
      const result = await handler!({});

      // @step Then the extension navigates the active tab back in history
      expect(mockTabs.goBack).toHaveBeenCalledWith(expect.any(Number));

      // @step And the tool returns confirmation of the navigation direction
      const parsed = JSON.parse(result.content[0].text);
      expect(parsed).toEqual(
        expect.objectContaining({ navigated: true, direction: 'back' })
      );
    });
  });

  describe('Scenario: Navigate browser history forward', () => {
    it('should navigate the active tab forward in history', async () => {
      // @step Given the agent has an active MCP connection to the extension
      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step When the agent calls mcp__ext__browser_go_forward
      mockTabs.goForward.mockResolvedValue(undefined);
      const handler = browserTools.getHandler('browser_go_forward');
      expect(handler).toBeDefined();
      const result = await handler!({});

      // @step Then the extension navigates the active tab forward in history
      expect(mockTabs.goForward).toHaveBeenCalledWith(expect.any(Number));

      // @step And the tool returns confirmation of the navigation direction
      const parsed = JSON.parse(result.content[0].text);
      expect(parsed).toEqual(
        expect.objectContaining({ navigated: true, direction: 'forward' })
      );
    });
  });

  describe('Scenario: All native browser tools are listed in tools/list response', () => {
    it('should include all 11 native browser control tools with proper schemas', async () => {
      // @step Given the agent has an active MCP connection to the extension
      // Validate both: the handler registry AND the NATIVE_TOOLS definitions in mcp-server

      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });

      // @step When the agent calls tools/list
      const allToolNames = browserTools.getToolNames();

      // @step Then the response includes all 12 native browser control tools
      const expectedTools = [
        'browser_navigate',
        'browser_screenshot',
        'browser_list_tabs',
        'browser_execute_script',
        'browser_switch_tab',
        'browser_close_tab',
        'browser_get_page_content',
        'browser_click_element',
        'browser_fill_form',
        'browser_go_back',
        'browser_go_forward',
        'browser_create_tab',
      ];
      for (const toolName of expectedTools) {
        expect(allToolNames).toContain(toolName);
      }
      expect(allToolNames).toHaveLength(12);

      // @step And each tool has a name, description, and inputSchema
      // Validate the NATIVE_TOOLS array from mcp-server matches handlers
      const { readFileSync } = await import('fs');
      const { join } = await import('path');
      const mcpServerPath = join(
        process.cwd(),
        'extension/host/lib/mcp-server.mjs'
      );
      const mcpSource = readFileSync(mcpServerPath, 'utf-8');

      // Verify every handler has a corresponding NATIVE_TOOLS entry
      for (const toolName of expectedTools) {
        const handler = browserTools.getHandler(toolName);
        expect(handler, `Handler missing for ${toolName}`).toBeDefined();
        expect(
          mcpSource,
          `NATIVE_TOOLS missing entry for ${toolName}`
        ).toContain(`name: '${toolName}'`);
      }

      // Verify NATIVE_TOOLS entries have description and inputSchema
      for (const toolName of expectedTools) {
        // Find the tool block in the source (between its name and the next tool or closing bracket)
        const nameIndex = mcpSource.indexOf(`name: '${toolName}'`);
        expect(
          nameIndex,
          `Could not find ${toolName} in NATIVE_TOOLS`
        ).toBeGreaterThan(-1);

        // Look for 'description:' and 'inputSchema:' after the name
        const blockEnd = mcpSource.indexOf('},\n  {', nameIndex);
        const toolBlock = mcpSource.substring(
          nameIndex,
          blockEnd > -1 ? blockEnd : nameIndex + 500
        );
        expect(toolBlock, `${toolName} missing description`).toContain(
          'description:'
        );
        expect(toolBlock, `${toolName} missing inputSchema`).toContain(
          'inputSchema:'
        );
      }
    });
  });

  describe('Integration: Message router dispatches to native browser tool handlers', () => {
    it('should route native tool calls to browser-tools handlers instead of returning errors', async () => {
      // Setup all components
      const { createNativeConnection } = await import(
        /* @vite-ignore */ '../../../extension/src/background/native-connection'
      );
      const { createMessageRouter } = await import(
        /* @vite-ignore */ '../../../extension/src/background/message-router'
      );
      const { createToolRegistry } = await import(
        /* @vite-ignore */ '../../../extension/src/background/tool-registry'
      );
      const { createBrowserTools } = await import(
        /* @vite-ignore */ '../../../extension/src/background/browser-tools'
      );

      const connection: NativeConnectionAPI = createNativeConnection({
        runtime: mockRuntime,
      });
      const toolRegistry: ToolRegistryAPI = createToolRegistry();
      const browserTools = createBrowserTools({
        tabs: mockTabs,
        scripting: mockScripting,
        windows: mockWindows,
      });
      const router: MessageRouterAPI = createMessageRouter({
        runtime: mockRuntime,
        tabs: mockTabs,
        connection,
        toolRegistry,
        browserTools,
      });

      connection.connect();

      // Call browser_list_tabs through the router
      mockTabs.query.mockResolvedValue([activeTab]);
      router.handleNativeMessage({
        type: 'TOOL_CALL',
        correlationId: 'test-123',
        params: { name: 'browser_list_tabs', arguments: {} },
      });

      // Wait for async handler
      await new Promise(resolve => setTimeout(resolve, 50));

      // Should NOT return error — should return the tool result
      expect(mockPort.postMessage).toHaveBeenCalledWith(
        expect.objectContaining({
          correlationId: 'test-123',
          result: expect.objectContaining({
            content: expect.arrayContaining([
              expect.objectContaining({ type: 'text' }),
            ]),
          }),
        })
      );
    });
  });
});
