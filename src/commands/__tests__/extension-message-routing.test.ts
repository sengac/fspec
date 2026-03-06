/**
 * Feature: spec/features/webmcp-chrome-extension.feature
 *
 * This test file validates the acceptance criteria for EXT-004:
 * Service Worker & Content Script Message Routing.
 * Scenarios map directly to Gherkin scenarios tagged @EXT-004.
 *
 * Chrome APIs are mocked since these run in Vitest, not a real browser.
 * Logic is extracted into testable modules with dependency injection.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { NativeConnectionAPI } from '../../../extension/src/background/native-connection';
import type { ToolRegistryAPI } from '../../../extension/src/background/tool-registry';
import type { MessageRouterAPI } from '../../../extension/src/background/message-router';
import type {
  ToolRegistryEntry,
  StatusResponse,
} from '../../../extension/src/types';

/**
 * Mock Chrome types for testing.
 * We mock the chrome.runtime and chrome.tabs APIs.
 */
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
    onMessage: {
      addListener: vi.fn(),
      removeListener: vi.fn(),
    },
    onDisconnect: {
      addListener: vi.fn(),
      removeListener: vi.fn(),
    },
    disconnect: vi.fn(),
  };
}

interface MockChromeRuntime {
  connectNative: ReturnType<typeof vi.fn>;
  onMessage: {
    addListener: ReturnType<typeof vi.fn>;
    removeListener: ReturnType<typeof vi.fn>;
  };
  sendMessage: ReturnType<typeof vi.fn>;
  lastError: { message: string } | null;
}

interface MockChromeTabs {
  sendMessage: ReturnType<typeof vi.fn>;
}

function createMockChromeRuntime(): MockChromeRuntime {
  return {
    connectNative: vi.fn(),
    onMessage: {
      addListener: vi.fn(),
      removeListener: vi.fn(),
    },
    sendMessage: vi.fn(),
    lastError: null,
  };
}

function createMockChromeTabs(): MockChromeTabs {
  return {
    sendMessage: vi.fn(),
  };
}

/**
 * Import implementation modules dynamically in tests since they
 * live in the extension/ subtree with its own tsconfig.
 */

describe('Feature: fspec Browser Agent Chrome Extension — EXT-004 Message Routing', () => {
  let mockRuntime: MockChromeRuntime;
  let mockTabs: MockChromeTabs;
  let mockPort: MockPort;

  beforeEach(() => {
    mockRuntime = createMockChromeRuntime();
    mockTabs = createMockChromeTabs();
    mockPort = createMockPort('com.fspec.browser.agent');
    mockRuntime.connectNative.mockReturnValue(mockPort);
    vi.clearAllMocks();
  });

  describe('Scenario: Service worker connects to native messaging host on startup', () => {
    it('should call chrome.runtime.connectNative and establish a native messaging port', async () => {
      // @step Given the fspec WebMCP Chrome extension is installed
      // Extension is installed — we have mock chrome APIs available

      // @step When the service worker activates
      // Dynamically import the module — this will fail until implemented
      const { createNativeConnection } = await import(
        /* @vite-ignore */ '../../../extension/src/background/native-connection'
      );
      const connection: NativeConnectionAPI = createNativeConnection({
        runtime: mockRuntime,
      });
      connection.connect();

      // @step Then the service worker calls chrome.runtime.connectNative with host name "com.fspec.browser.agent"
      expect(mockRuntime.connectNative).toHaveBeenCalledWith(
        'com.fspec.browser.agent'
      );

      // @step And a native messaging port is established for bidirectional communication
      expect(connection.isConnected()).toBe(true);
      expect(connection.getPort()).toBe(mockPort);

      // @step And the service worker logs the connection status
      // Connection listener was set up
      expect(mockPort.onMessage.addListener).toHaveBeenCalled();
      expect(mockPort.onDisconnect.addListener).toHaveBeenCalled();
    });
  });

  describe('Scenario: Service worker relays tool calls between native host and content scripts', () => {
    it('should route native host tool calls to handlers and return results', async () => {
      // @step Given the fspec WebMCP Chrome extension is installed and running
      const { createNativeConnection } = await import(
        /* @vite-ignore */ '../../../extension/src/background/native-connection'
      );
      const { createMessageRouter } = await import(
        /* @vite-ignore */ '../../../extension/src/background/message-router'
      );
      const { createToolRegistry } = await import(
        /* @vite-ignore */ '../../../extension/src/background/tool-registry'
      );

      const connection: NativeConnectionAPI = createNativeConnection({
        runtime: mockRuntime,
      });
      const toolRegistry: ToolRegistryAPI = createToolRegistry();
      const router: MessageRouterAPI = createMessageRouter({
        runtime: mockRuntime,
        tabs: mockTabs,
        connection,
        toolRegistry,
      });

      // @step And the service worker has an active native messaging connection to the host
      connection.connect();
      expect(connection.isConnected()).toBe(true);

      // @step When the native host sends a tool call message with a correlation ID
      const toolCallMessage = {
        type: 'TOOL_CALL',
        correlationId: 'abc-123',
        method: 'tools/call',
        params: {
          name: 'browser_navigate',
          arguments: { url: 'https://example.com' },
        },
      };

      // Simulate the tool call being handled
      // Native browser tools have no handler in EXT-004 (registered by EXT-005),
      // so the router responds with a -32601 error. The key assertion is that the
      // correlation ID is preserved and the error is structured correctly.
      router.handleNativeMessage(toolCallMessage);

      // @step Then the service worker routes the call to the appropriate handler
      // @step And the service worker sends the result back to the native host via the native messaging port with the matching correlation ID
      expect(mockPort.postMessage).toHaveBeenCalledWith(
        expect.objectContaining({
          correlationId: 'abc-123',
          error: expect.objectContaining({
            code: -32601,
            message: expect.stringContaining('browser_navigate'),
          }),
        })
      );
    });
  });

  describe('Scenario: Content script relays WebMCP tool registration from main world to service worker', () => {
    it('should receive tool registration from content script and update registry', async () => {
      // @step Given the content script is running on a web page in tab 42
      const { createNativeConnection } = await import(
        /* @vite-ignore */ '../../../extension/src/background/native-connection'
      );
      const { createMessageRouter } = await import(
        /* @vite-ignore */ '../../../extension/src/background/message-router'
      );
      const { createToolRegistry } = await import(
        /* @vite-ignore */ '../../../extension/src/background/tool-registry'
      );

      const connection: NativeConnectionAPI = createNativeConnection({
        runtime: mockRuntime,
      });
      const toolRegistry: ToolRegistryAPI = createToolRegistry();
      const router: MessageRouterAPI = createMessageRouter({
        runtime: mockRuntime,
        tabs: mockTabs,
        connection,
        toolRegistry,
      });
      connection.connect();

      // @step When the main-world script posts a message with type "FSPEC_WEBMCP_TOOL_REGISTERED" and tool metadata
      const toolRegMessage = {
        type: 'FSPEC_WEBMCP_TOOL_REGISTERED',
        tool: {
          name: 'searchFlights',
          description: 'Search for flights',
          inputSchema: {
            type: 'object',
            properties: { from: { type: 'string' } },
          },
        },
        origin: 'travel-demo.bandarra.me',
      };

      // @step Then the content script forwards the message to the service worker via chrome.runtime.sendMessage
      // Content script forwarding is tested separately — here we test the SW receiving it
      const sendResponse = vi.fn();

      // @step And the service worker receives the message with the sender tab ID 42
      const result = router.handleContentScriptMessage(
        toolRegMessage,
        42,
        sendResponse
      );
      expect(result).toBe(true);

      // @step And the service worker updates its internal tool registry
      const registeredTool = toolRegistry.getByName(
        'webmcp__travel-demo-bandarra-me__searchFlights'
      );
      expect(registeredTool).toBeDefined();
      expect(registeredTool?.source).toBe('webmcp');
      expect(registeredTool?.tabId).toBe(42);

      // @step And the service worker forwards a TOOLS_CHANGED message to the native host
      expect(mockPort.postMessage).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'TOOLS_CHANGED',
          tools: expect.arrayContaining([
            expect.objectContaining({
              name: 'webmcp__travel-demo-bandarra-me__searchFlights',
            }),
          ]),
        })
      );
    });
  });

  describe('Scenario: Service worker routes tool invocation to correct tab via content script', () => {
    it('should route WebMCP tool call to correct tab and relay result back', async () => {
      // @step Given the service worker has WebMCP tools registered from tab 42
      const { createNativeConnection } = await import(
        /* @vite-ignore */ '../../../extension/src/background/native-connection'
      );
      const { createMessageRouter } = await import(
        /* @vite-ignore */ '../../../extension/src/background/message-router'
      );
      const { createToolRegistry } = await import(
        /* @vite-ignore */ '../../../extension/src/background/tool-registry'
      );

      const connection: NativeConnectionAPI = createNativeConnection({
        runtime: mockRuntime,
      });
      const toolRegistry: ToolRegistryAPI = createToolRegistry();
      const router: MessageRouterAPI = createMessageRouter({
        runtime: mockRuntime,
        tabs: mockTabs,
        connection,
        toolRegistry,
      });
      connection.connect();

      // Register a WebMCP tool from tab 42
      toolRegistry.register({
        name: 'webmcp__travel-demo.bandarra.me__searchFlights',
        description: 'Search for flights',
        source: 'webmcp',
        origin: 'travel-demo.bandarra.me',
        tabId: 42,
      });

      // @step When the native host sends a tool call for a WebMCP tool on tab 42
      const toolCallMessage = {
        type: 'TOOL_CALL',
        correlationId: 'xyz-789',
        method: 'tools/call',
        params: {
          name: 'webmcp__travel-demo.bandarra.me__searchFlights',
          arguments: { from: 'London', to: 'Paris' },
        },
      };
      router.handleNativeMessage(toolCallMessage);

      // @step Then the service worker sends the invocation request to tab 42 via chrome.tabs.sendMessage
      expect(mockTabs.sendMessage).toHaveBeenCalledWith(
        42,
        expect.objectContaining({
          type: 'FSPEC_INVOKE_TOOL',
          correlationId: 'xyz-789',
          toolName: 'searchFlights',
        }),
        expect.any(Function) // callback
      );

      // @step And the content script relays the request to the main world via window.postMessage
      // (Content script relay is tested separately in content script tests)

      // @step And the main world executes the tool and posts the result back
      // (Main world execution is out of scope — we test the return path)

      // @step And the content script relays the result to the service worker via chrome.runtime.sendMessage
      // Simulate the result coming back from content script
      const resultMessage = {
        type: 'FSPEC_INVOKE_RESULT',
        correlationId: 'xyz-789',
        result: { flights: [{ airline: 'BA', price: 150 }] },
      };
      const sendResponse = vi.fn();
      router.handleContentScriptMessage(resultMessage, 42, sendResponse);

      // @step And the service worker returns the result to the native host
      expect(mockPort.postMessage).toHaveBeenCalledWith(
        expect.objectContaining({
          correlationId: 'xyz-789',
          result: expect.objectContaining({
            content: expect.arrayContaining([
              expect.objectContaining({ type: 'text' }),
            ]),
          }),
        })
      );
    });
  });

  describe('Scenario: Service worker handles native messaging port disconnect and reconnects', () => {
    it('should detect disconnection and attempt reconnection', async () => {
      // @step Given the service worker has an active native messaging connection
      const { createNativeConnection } = await import(
        /* @vite-ignore */ '../../../extension/src/background/native-connection'
      );

      const connection: NativeConnectionAPI = createNativeConnection({
        runtime: mockRuntime,
        reconnectDelay: 50, // Short delay for testing
      });
      connection.connect();
      expect(connection.isConnected()).toBe(true);

      // @step When the native messaging port disconnects
      // Get the onDisconnect handler and call it
      const disconnectHandler = mockPort.onDisconnect.addListener.mock
        .calls[0][0] as () => void;
      disconnectHandler();

      // @step Then the service worker detects the disconnection via port.onDisconnect
      expect(connection.isConnected()).toBe(false);

      // @step And the service worker waits before attempting reconnection
      // Reset the mock to track reconnection
      mockRuntime.connectNative.mockClear();
      const newPort = createMockPort('com.fspec.browser.agent');
      mockRuntime.connectNative.mockReturnValue(newPort);

      // Wait for reconnection delay (50ms * 2^0 = 50ms, allow margin)
      await new Promise(resolve => setTimeout(resolve, 150));

      // @step And the service worker establishes a new native messaging connection
      expect(mockRuntime.connectNative).toHaveBeenCalledWith(
        'com.fspec.browser.agent'
      );
      expect(connection.isConnected()).toBe(true);

      // Clean up
      connection.disconnect();
    });

    it('should retry with exponential backoff on repeated failures', async () => {
      const { createNativeConnection } = await import(
        /* @vite-ignore */ '../../../extension/src/background/native-connection'
      );

      // Initial connect succeeds
      const connection: NativeConnectionAPI = createNativeConnection({
        runtime: mockRuntime,
        reconnectDelay: 10, // Very short for testing
        maxReconnectAttempts: 5,
      });
      connection.connect();
      expect(connection.isConnected()).toBe(true);

      // After disconnect, first two reconnect attempts fail, third succeeds
      let reconnectCallCount = 0;
      mockRuntime.connectNative.mockImplementation(() => {
        reconnectCallCount++;
        if (reconnectCallCount <= 2) {
          throw new Error('Host not found');
        }
        return createMockPort('com.fspec.browser.agent');
      });

      // Simulate disconnect
      const handler = mockPort.onDisconnect.addListener.mock
        .calls[0][0] as () => void;
      handler();
      expect(connection.isConnected()).toBe(false);

      // Wait for retries (10ms, 20ms, 40ms — should succeed on 3rd)
      await new Promise(resolve => setTimeout(resolve, 200));

      expect(connection.isConnected()).toBe(true);
      expect(reconnectCallCount).toBe(3);

      connection.disconnect();
    });

    it('should not double-connect when already connected', async () => {
      const { createNativeConnection } = await import(
        /* @vite-ignore */ '../../../extension/src/background/native-connection'
      );

      const connection: NativeConnectionAPI = createNativeConnection({
        runtime: mockRuntime,
      });
      connection.connect();
      connection.connect(); // second call should be a no-op

      expect(mockRuntime.connectNative).toHaveBeenCalledTimes(1);

      connection.disconnect();
    });
  });

  describe('Scenario: Service worker responds to status queries from popup', () => {
    it('should return connection status, tool count, and native messaging state', async () => {
      // @step Given the service worker is running with an active native messaging connection
      const { createNativeConnection } = await import(
        /* @vite-ignore */ '../../../extension/src/background/native-connection'
      );
      const { createMessageRouter } = await import(
        /* @vite-ignore */ '../../../extension/src/background/message-router'
      );
      const { createToolRegistry } = await import(
        /* @vite-ignore */ '../../../extension/src/background/tool-registry'
      );

      const connection: NativeConnectionAPI = createNativeConnection({
        runtime: mockRuntime,
      });
      const toolRegistry: ToolRegistryAPI = createToolRegistry();
      const router: MessageRouterAPI = createMessageRouter({
        runtime: mockRuntime,
        tabs: mockTabs,
        connection,
        toolRegistry,
      });
      connection.connect();

      // @step And the tool registry contains 5 tools
      for (let i = 0; i < 5; i++) {
        toolRegistry.register({
          name: `tool_${i}`,
          description: `Tool ${i}`,
          source: i < 3 ? 'native' : 'webmcp',
          tabId: i >= 3 ? 42 : undefined,
        });
      }
      expect(toolRegistry.size()).toBe(5);

      // @step When the popup sends a message with type "FSPEC_GET_STATUS"
      const statusMessage = { type: 'FSPEC_GET_STATUS' };
      const sendResponse = vi.fn();
      const handled = router.handlePopupMessage(statusMessage, sendResponse);

      // @step Then the service worker responds with connection status, tool count, and native messaging state
      expect(handled).toBe(true);
      expect(sendResponse).toHaveBeenCalledWith(
        expect.objectContaining({
          connected: true,
          nativeConnected: true,
          toolCount: 5,
          port: 19876,
        } satisfies StatusResponse)
      );
    });
  });
});
