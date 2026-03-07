/**
 * Feature: spec/features/webmcp-dynamic-tool-discovery.feature
 *
 * This test file validates the acceptance criteria for EXT-009:
 * WebMCP dynamic tool registration not detected — polyfill libraries
 * bypass navigator.modelContext.
 *
 * Tests the layered discovery strategy in webmcp-discovery.ts:
 * Layer 1: navigator.modelContext interception (existing)
 * Layer 2: WebMCP class prototype interception
 * Layer 3: Post-load snapshot for already-registered tools
 * Layer 4: ModelContextTesting API (opportunistic)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { webmcpDiscoveryFunction } from '../webmcp-discovery';
import {
  createWebMCPInjector,
  type ChromeScriptingForInjector,
  type ChromeTabsForInjector,
} from '../../background/webmcp-injector';

describe('Feature: WebMCP Dynamic Tool Discovery', () => {
  /**
   * Track message event listeners added by webmcpDiscoveryFunction()
   * so they can be removed between tests for proper isolation.
   * Without this, old listeners from previous tests fire on new messages,
   * causing cross-test contamination (e.g. error results from stale handlers).
   */
  const trackedMessageListeners: Array<EventListenerOrEventListenerObject> = [];
  let originalAddEventListener: typeof window.addEventListener;

  beforeEach(() => {
    // Intercept addEventListener to track message listeners for cleanup
    originalAddEventListener = window.addEventListener.bind(window);
    const origAdd = window.addEventListener.bind(window);
    window.addEventListener = ((
      type: string,
      listener: EventListenerOrEventListenerObject,
      options?: boolean | AddEventListenerOptions
    ) => {
      if (type === 'message') {
        trackedMessageListeners.push(listener);
      }
      origAdd(type, listener, options);
    }) as typeof window.addEventListener;

    // Clean up any discovery flag from previous tests
    if (
      typeof window !== 'undefined' &&
      (window as unknown as Record<string, unknown>)
        .__fspec_webmcp_discovery_active
    ) {
      delete (window as unknown as Record<string, unknown>)
        .__fspec_webmcp_discovery_active;
    }
  });

  afterEach(() => {
    // Remove all tracked message listeners to prevent cross-test contamination
    for (const listener of trackedMessageListeners) {
      window.removeEventListener('message', listener);
    }
    trackedMessageListeners.length = 0;

    // Restore original addEventListener
    if (originalAddEventListener) {
      window.addEventListener =
        originalAddEventListener as typeof window.addEventListener;
    }

    // Clean up discovery flag
    if (
      typeof window !== 'undefined' &&
      (window as unknown as Record<string, unknown>)
        .__fspec_webmcp_discovery_active
    ) {
      delete (window as unknown as Record<string, unknown>)
        .__fspec_webmcp_discovery_active;
    }
    // Clean up any WebMCP globals
    if (typeof window !== 'undefined') {
      delete (window as unknown as Record<string, unknown>).WebMCP;
      delete (window as unknown as Record<string, unknown>).webMCP;
      delete (window as unknown as Record<string, unknown>).mcp;
    }
  });

  describe('Scenario: Discover tools registered via WebMCP library class', () => {
    it('should intercept WebMCP.prototype.registerTool and post a FSPEC_WEBMCP_TOOL_REGISTERED message', () => {
      // @step Given the discovery script is active in the page's main world
      const postMessageSpy = vi.spyOn(window, 'postMessage');

      // Set up a mock WebMCP class on window BEFORE discovery runs
      class MockWebMCP {
        private tools: Array<{
          name: string;
          description: string;
          schema: Record<string, unknown>;
          fn: () => string;
        }> = [];
        registerTool(
          name: string,
          description: string,
          schema: Record<string, unknown>,
          fn: () => string
        ): void {
          this.tools.push({ name, description, schema, fn });
        }
        getTools(): Array<{
          name: string;
          description: string;
          inputSchema: Record<string, unknown>;
        }> {
          return this.tools.map(t => ({
            name: t.name,
            description: t.description,
            inputSchema: t.schema,
          }));
        }
      }
      (window as unknown as Record<string, unknown>).WebMCP = MockWebMCP;

      // Run the discovery script
      webmcpDiscoveryFunction();

      // @step And the page creates a WebMCP instance and calls registerTool with name "calculator"
      const WebMCPClass = (window as unknown as Record<string, unknown>)
        .WebMCP as typeof MockWebMCP;
      const instance = new WebMCPClass();
      const executeFn = vi.fn(() => '42');
      instance.registerTool(
        'calculator',
        'A calculator tool',
        { type: 'object' },
        executeFn
      );

      // @step When the tool registration is intercepted by the WebMCP prototype wrapper
      // (happens automatically via the monkey-patched prototype)

      // @step Then a FSPEC_WEBMCP_TOOL_REGISTERED message is posted with tool name "calculator"
      const registrationMessages = postMessageSpy.mock.calls.filter(
        call =>
          (call[0] as Record<string, unknown>)?.type ===
          'FSPEC_WEBMCP_TOOL_REGISTERED'
      );
      expect(registrationMessages.length).toBeGreaterThanOrEqual(1);
      const msg = registrationMessages[0][0] as Record<string, unknown>;
      const tool = msg.tool as Record<string, unknown>;
      expect(tool.name).toBe('calculator');
      expect(tool.description).toBe('A calculator tool');

      // @step And the execute callback is stored for later invocation
      // Verified by invocation test below

      postMessageSpy.mockRestore();
    });
  });

  describe('Scenario: Discover tools registered via native navigator.modelContext', () => {
    it('should intercept navigator.modelContext.registerTool and post a message', () => {
      // @step Given the discovery script is active in the page's main world
      const postMessageSpy = vi.spyOn(window, 'postMessage');

      // @step And navigator.modelContext exists as a native browser API
      const mockModelContext: Record<string, unknown> = {
        registerTool: vi.fn(),
        unregisterTool: vi.fn(),
      };
      Object.defineProperty(navigator, 'modelContext', {
        value: mockModelContext,
        writable: true,
        configurable: true,
      });

      // Run the discovery script
      webmcpDiscoveryFunction();

      // @step When the page calls navigator.modelContext.registerTool with name "weather"
      const mc = (navigator as unknown as Record<string, unknown>)
        .modelContext as Record<string, unknown>;
      const registerTool = mc.registerTool as (
        toolDef: Record<string, unknown>
      ) => unknown;
      const executeFn = vi.fn(() => 'sunny');
      registerTool({
        name: 'weather',
        description: 'Get weather',
        inputSchema: { type: 'object' },
        execute: executeFn,
      });

      // @step Then a FSPEC_WEBMCP_TOOL_REGISTERED message is posted with tool name "weather"
      const registrationMessages = postMessageSpy.mock.calls.filter(
        call =>
          (call[0] as Record<string, unknown>)?.type ===
          'FSPEC_WEBMCP_TOOL_REGISTERED'
      );
      expect(registrationMessages.length).toBeGreaterThanOrEqual(1);
      const msg = registrationMessages[0][0] as Record<string, unknown>;
      const tool = msg.tool as Record<string, unknown>;
      expect(tool.name).toBe('weather');

      // @step And the execute callback is stored for later invocation
      // Tested via invocation scenario

      postMessageSpy.mockRestore();
      // Cleanup
      delete (navigator as unknown as Record<string, unknown>).modelContext;
    });
  });

  describe('Scenario: Discover tools registered during initial page script execution', () => {
    it('should have the monkey-patch in place before page tools register', () => {
      // @step Given the content script runs at document_start
      // The manifest.json specifies run_at: document_start for early execution

      // @step And the MAIN-world discovery script is injected before page scripts execute
      // Simulate: discovery runs first, then page script runs
      const postMessageSpy = vi.spyOn(window, 'postMessage');

      const mockModelContext: Record<string, unknown> = {
        registerTool: vi.fn(),
        unregisterTool: vi.fn(),
      };
      Object.defineProperty(navigator, 'modelContext', {
        value: mockModelContext,
        writable: true,
        configurable: true,
      });

      // Discovery runs first (simulating early injection)
      webmcpDiscoveryFunction();

      // @step When the page registers a tool during its initial script execution
      const mc = (navigator as unknown as Record<string, unknown>)
        .modelContext as Record<string, unknown>;
      const registerTool = mc.registerTool as (
        toolDef: Record<string, unknown>
      ) => unknown;
      registerTool({
        name: 'early-tool',
        description: 'Registered during initial page load',
        execute: () => 'result',
      });

      // @step Then the monkey-patch is already in place to intercept the registration
      const registrationMessages = postMessageSpy.mock.calls.filter(
        call =>
          (call[0] as Record<string, unknown>)?.type ===
          'FSPEC_WEBMCP_TOOL_REGISTERED'
      );
      expect(registrationMessages.length).toBeGreaterThanOrEqual(1);

      // @step And the tool appears in the MCP tools list
      const tool = (registrationMessages[0][0] as Record<string, unknown>)
        .tool as Record<string, unknown>;
      expect(tool.name).toBe('early-tool');

      postMessageSpy.mockRestore();
      delete (navigator as unknown as Record<string, unknown>).modelContext;
    });
  });

  describe('Scenario: Trap late assignment of WebMCP class on window', () => {
    it('should intercept WebMCP class assigned to window after discovery is active', () => {
      // @step Given the discovery script is active in the page's main world
      const postMessageSpy = vi.spyOn(window, 'postMessage');

      // @step And window.WebMCP does not yet exist
      expect(
        (window as unknown as Record<string, unknown>).WebMCP
      ).toBeUndefined();

      // Run the discovery script
      webmcpDiscoveryFunction();

      // @step When the page assigns a WebMCP class to window.WebMCP
      class LateWebMCP {
        private tools: Array<Record<string, unknown>> = [];
        registerTool(
          name: string,
          description: string,
          schema: Record<string, unknown>,
          fn: () => string
        ): void {
          this.tools.push({ name, description, schema, fn });
        }
        getTools(): Array<Record<string, unknown>> {
          return this.tools;
        }
      }
      (window as unknown as Record<string, unknown>).WebMCP = LateWebMCP;

      // @step Then the Object.defineProperty trap intercepts the assignment
      // @step And the new class's prototype.registerTool is wrapped with the interceptor
      const instance = new ((window as unknown as Record<string, unknown>)
        .WebMCP as typeof LateWebMCP)();
      instance.registerTool('late-tool', 'A late tool', {}, () => 'result');

      const registrationMessages = postMessageSpy.mock.calls.filter(
        call =>
          (call[0] as Record<string, unknown>)?.type ===
          'FSPEC_WEBMCP_TOOL_REGISTERED'
      );
      expect(registrationMessages.length).toBeGreaterThanOrEqual(1);
      const msg = registrationMessages[0][0] as Record<string, unknown>;
      const tool = msg.tool as Record<string, unknown>;
      expect(tool.name).toBe('late-tool');

      postMessageSpy.mockRestore();
    });
  });

  describe('Scenario: Discover pre-existing tools via post-load snapshot', () => {
    it('should scan window.webMCP for already-registered tools after a delay', async () => {
      // @step Given the discovery script is active in the page's main world
      const postMessageSpy = vi.spyOn(window, 'postMessage');
      vi.useFakeTimers();

      // @step And a WebMCP instance on window.webMCP already has tools registered
      const executeMock = vi.fn(() => 'snapshot-result');
      const existingInstance = {
        getTools: () => [
          {
            name: 'existing-tool',
            description: 'Pre-existing tool',
            inputSchema: { type: 'object' },
          },
        ],
        executeTool: executeMock,
      };
      (window as unknown as Record<string, unknown>).webMCP = existingInstance;

      // Run the discovery script
      webmcpDiscoveryFunction();

      // @step When the post-load snapshot runs after a short delay
      await vi.advanceTimersByTimeAsync(600);

      // @step Then all tools from the instance's getTools() are discovered
      // @step And FSPEC_WEBMCP_TOOL_REGISTERED messages are posted for each undiscovered tool
      const registrationMessages = postMessageSpy.mock.calls.filter(
        call =>
          (call[0] as Record<string, unknown>)?.type ===
          'FSPEC_WEBMCP_TOOL_REGISTERED'
      );
      expect(registrationMessages.length).toBeGreaterThanOrEqual(1);
      const toolNames = registrationMessages.map(
        call =>
          ((call[0] as Record<string, unknown>).tool as Record<string, unknown>)
            .name
      );
      expect(toolNames).toContain('existing-tool');

      // Verify snapshot-discovered tools can be invoked via the instance's executeTool
      postMessageSpy.mockClear();
      window.dispatchEvent(
        new MessageEvent('message', {
          source: window,
          data: {
            type: 'FSPEC_INVOKE_TOOL',
            correlationId: 'snap-123',
            toolName: 'existing-tool',
            args: { x: 42 },
          },
        })
      );

      expect(executeMock).toHaveBeenCalledWith('existing-tool', { x: 42 });
      const resultMessages = postMessageSpy.mock.calls.filter(
        call =>
          (call[0] as Record<string, unknown>)?.type === 'FSPEC_INVOKE_RESULT'
      );
      expect(resultMessages.length).toBeGreaterThanOrEqual(1);
      const resultMsg = resultMessages[0][0] as Record<string, unknown>;
      expect(resultMsg.correlationId).toBe('snap-123');
      expect(resultMsg.result).toBe('snapshot-result');

      vi.useRealTimers();
      postMessageSpy.mockRestore();
    });
  });

  describe('Scenario: Invoke a tool registered via WebMCP library', () => {
    it('should execute the stored callback when FSPEC_INVOKE_TOOL arrives', () => {
      // @step Given a tool "calculator" was registered via the WebMCP library
      const postMessageSpy = vi.spyOn(window, 'postMessage');

      class MockWebMCP {
        registerTool(
          _name: string,
          _desc: string,
          _schema: Record<string, unknown>,
          _fn: () => string
        ): void {
          // intentionally blank — will be wrapped
        }
      }
      (window as unknown as Record<string, unknown>).WebMCP = MockWebMCP;

      webmcpDiscoveryFunction();

      // @step And the execute callback was captured by the discovery script
      const executeFn = vi.fn(() => '42');
      const WebMCPClass = (window as unknown as Record<string, unknown>)
        .WebMCP as typeof MockWebMCP;
      const instance = new WebMCPClass();
      instance.registerTool('calculator', 'A calc', {}, executeFn);

      // Clear previous messages
      postMessageSpy.mockClear();

      // @step When a FSPEC_INVOKE_TOOL message arrives for tool "calculator" with arguments
      // The listeners were already set up by webmcpDiscoveryFunction, so dispatch event
      window.dispatchEvent(
        new MessageEvent('message', {
          source: window,
          data: {
            type: 'FSPEC_INVOKE_TOOL',
            correlationId: 'test-123',
            toolName: 'calculator',
            args: { x: 1 },
          },
        })
      );

      // @step Then the stored execute callback is called with the provided arguments
      expect(executeFn).toHaveBeenCalledWith({ x: 1 });

      // @step And a FSPEC_INVOKE_RESULT message is posted with the result
      const resultMessages = postMessageSpy.mock.calls.filter(
        call =>
          (call[0] as Record<string, unknown>)?.type === 'FSPEC_INVOKE_RESULT'
      );
      expect(resultMessages.length).toBeGreaterThanOrEqual(1);
      const resultMsg = resultMessages[0][0] as Record<string, unknown>;
      expect(resultMsg.correlationId).toBe('test-123');
      expect(resultMsg.result).toBe('42');

      postMessageSpy.mockRestore();
    });
  });

  describe('Scenario: Prevent double-injection of discovery script', () => {
    it('should not re-initialize when called twice', () => {
      // @step Given the discovery script has already been injected into the page
      webmcpDiscoveryFunction();

      // @step And the __fspec_webmcp_discovery_active flag is set on window
      expect(
        (window as unknown as Record<string, unknown>)
          .__fspec_webmcp_discovery_active
      ).toBe(true);

      const addEventListenerSpy = vi.spyOn(window, 'addEventListener');

      // @step When the discovery script function is called again
      webmcpDiscoveryFunction();

      // @step Then the function returns immediately without re-initializing
      // @step And no duplicate interceptors are installed
      // addEventListener should not be called again for 'message' listener
      const messageListenerCalls = addEventListenerSpy.mock.calls.filter(
        call => call[0] === 'message'
      );
      expect(messageListenerCalls.length).toBe(0);

      addEventListenerSpy.mockRestore();
    });
  });

  describe('Scenario: Use ModelContextTesting API when available', () => {
    it('should use ontoolchange and listTools when ModelContextTesting is available', async () => {
      // @step Given the discovery script is active in the page's main world
      const postMessageSpy = vi.spyOn(window, 'postMessage');

      // @step And navigator.modelContext.testing exists with ontoolchange and listTools
      const mockTesting = {
        ontoolchange: null as (() => void) | null,
        listTools: vi.fn(() => [
          {
            name: 'pre-existing',
            description: 'Already registered',
            inputSchema: { type: 'object' },
          },
        ]),
        executeTool: vi.fn().mockResolvedValue('testing-result'),
      };
      const mockModelContext: Record<string, unknown> = {
        registerTool: vi.fn(),
        unregisterTool: vi.fn(),
        testing: mockTesting,
      };
      Object.defineProperty(navigator, 'modelContext', {
        value: mockModelContext,
        writable: true,
        configurable: true,
      });

      // @step When the ModelContextTesting API is detected
      webmcpDiscoveryFunction();

      // @step Then the ontoolchange event handler is registered for real-time notifications
      expect(mockTesting.ontoolchange).not.toBeNull();

      // @step And listTools() is called to discover tools registered before injection
      expect(mockTesting.listTools).toHaveBeenCalled();

      // Check that the pre-existing tool was discovered
      const registrationMessages = postMessageSpy.mock.calls.filter(
        call =>
          (call[0] as Record<string, unknown>)?.type ===
          'FSPEC_WEBMCP_TOOL_REGISTERED'
      );
      expect(registrationMessages.length).toBeGreaterThanOrEqual(1);

      // Verify that tools discovered via Layer 4 can be invoked via executeTool()
      postMessageSpy.mockClear();
      mockTesting.executeTool.mockClear();

      window.dispatchEvent(
        new MessageEvent('message', {
          source: window,
          data: {
            type: 'FSPEC_INVOKE_TOOL',
            correlationId: 'test-layer4',
            toolName: 'pre-existing',
            args: { input: 'hello' },
          },
        })
      );

      // executeTool should be called since no callback was stored for this tool
      expect(mockTesting.executeTool).toHaveBeenCalledWith(
        'pre-existing',
        JSON.stringify({ input: 'hello' })
      );

      // Wait for the promise to resolve and check the result message
      await vi.waitFor(() => {
        const resultMessages = postMessageSpy.mock.calls.filter(
          call =>
            (call[0] as Record<string, unknown>)?.type === 'FSPEC_INVOKE_RESULT'
        );
        expect(resultMessages.length).toBeGreaterThanOrEqual(1);
        const resultMsg = resultMessages[0][0] as Record<string, unknown>;
        expect(resultMsg.correlationId).toBe('test-layer4');
        expect(resultMsg.result).toBe('testing-result');
      });

      postMessageSpy.mockRestore();
      delete (navigator as unknown as Record<string, unknown>).modelContext;
    });
  });

  describe('Scenario: Injector uses early injection strategy', () => {
    it('should inject the discovery script into MAIN world on tab update', async () => {
      // @step Given the WebMCP injector is initialized with chrome.scripting and chrome.tabs
      const tabUpdatedCallbacks: Array<
        (
          tabId: number,
          changeInfo: { status?: string; url?: string },
          tab: { id?: number; url?: string }
        ) => void
      > = [];

      const mockScripting: ChromeScriptingForInjector = {
        executeScript: vi.fn().mockResolvedValue(undefined),
      };

      const mockTabs: ChromeTabsForInjector = {
        onUpdated: {
          addListener: (
            callback: (
              tabId: number,
              changeInfo: { status?: string; url?: string },
              tab: { id?: number; url?: string }
            ) => void
          ) => {
            tabUpdatedCallbacks.push(callback);
          },
        },
      };

      createWebMCPInjector({
        scripting: mockScripting,
        tabs: mockTabs,
      });

      // @step When a tab triggers the injection
      for (const cb of tabUpdatedCallbacks) {
        cb(1, { status: 'complete' }, { id: 1, url: 'https://example.com' });
      }

      await vi.waitFor(() => {
        expect(mockScripting.executeScript).toHaveBeenCalled();
      });

      // @step Then the discovery script is injected into the MAIN world
      const call = (mockScripting.executeScript as ReturnType<typeof vi.fn>)
        .mock.calls[0][0] as {
        target: { tabId: number };
        world: string;
        func: () => void;
        injectImmediately?: boolean;
      };
      expect(call.target.tabId).toBe(1);
      expect(call.world).toBe('MAIN');

      // @step And the injection uses the earliest available timing
      expect(call.injectImmediately).toBe(true);
    });
  });
});
