/**
 * Feature: spec/features/webmcp-chrome-extension.feature
 *
 * This test file validates the content script relay logic for EXT-004.
 * The content script bridges main-world scripts and the service worker:
 *   window.postMessage ↔ chrome.runtime.sendMessage
 *
 * Covers the content script side of:
 *   - "Content script relays WebMCP tool registration from main world to service worker"
 *   - "Service worker routes tool invocation to correct tab via content script"
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import type {
  ContentRelayAPI,
  WindowLike,
  ContentRuntimeLike,
} from '../../../extension/src/content/relay';

function createMockWindow(): WindowLike & {
  simulateMessage: (data: unknown) => void;
} {
  let messageHandler: ((event: MessageEvent) => void) | null = null;
  const win: WindowLike & { simulateMessage: (data: unknown) => void } = {
    addEventListener: vi.fn(
      (type: string, handler: (event: MessageEvent) => void) => {
        if (type === 'message') {
          messageHandler = handler;
        }
      }
    ),
    postMessage: vi.fn(),
    simulateMessage(data: unknown): void {
      if (messageHandler) {
        // Create a MessageEvent-like object with source pointing to this window
        messageHandler({ source: win, data } as MessageEvent);
      }
    },
  };
  return win;
}

function createMockRuntime(): ContentRuntimeLike & {
  simulateMessage: (message: { type?: string }) => void;
} {
  let runtimeHandler:
    | ((
        message: { type?: string },
        sender: unknown,
        sendResponse: (response?: unknown) => void
      ) => boolean | void)
    | null = null;
  return {
    sendMessage: vi.fn(),
    onMessage: {
      addListener: vi.fn(
        (
          cb: (
            message: { type?: string },
            sender: unknown,
            sendResponse: (response?: unknown) => void
          ) => boolean | void
        ) => {
          runtimeHandler = cb;
        }
      ),
    },
    simulateMessage(message: { type?: string }): void {
      if (runtimeHandler) {
        runtimeHandler(message, {}, vi.fn());
      }
    },
  };
}

describe('Feature: fspec Browser Agent Chrome Extension — Content Script Relay', () => {
  let mockWindow: ReturnType<typeof createMockWindow>;
  let mockRuntime: ReturnType<typeof createMockRuntime>;
  let relay: ContentRelayAPI;

  beforeEach(async () => {
    mockWindow = createMockWindow();
    mockRuntime = createMockRuntime();

    const { createContentRelay } = await import(
      /* @vite-ignore */ '../../../extension/src/content/relay'
    );
    relay = createContentRelay({
      win: mockWindow,
      runtime: mockRuntime,
    });
  });

  describe('Content script relays WebMCP tool registration from main world to service worker', () => {
    it('should forward FSPEC_WEBMCP_TOOL_REGISTERED to service worker', () => {
      // @step Given the content script is running on a web page in tab 42
      // Content script relay is initialised with mock window and runtime

      // @step When the main-world script posts a message with type "FSPEC_WEBMCP_TOOL_REGISTERED" and tool metadata
      const toolMessage = {
        type: 'FSPEC_WEBMCP_TOOL_REGISTERED',
        tool: { name: 'searchFlights', description: 'Search flights' },
      };
      const forwarded = relay.handleWindowMessage({
        source: mockWindow,
        data: toolMessage,
      });

      // @step Then the content script forwards the message to the service worker via chrome.runtime.sendMessage
      expect(forwarded).toBe(true);
      expect(mockRuntime.sendMessage).toHaveBeenCalledWith(toolMessage);

      // @step And the service worker receives the message with the sender tab ID 42
      // (Service worker side tested in extension-message-routing.test.ts — content relay delivers the message)

      // @step And the service worker updates its internal tool registry
      // (Registry update tested in extension-message-routing.test.ts)

      // @step And the service worker forwards a TOOLS_CHANGED message to the native host
      // (Native host forwarding tested in extension-message-routing.test.ts)
    });

    it('should forward FSPEC_WEBMCP_TOOL_UNREGISTERED to service worker', () => {
      const unregMessage = {
        type: 'FSPEC_WEBMCP_TOOL_UNREGISTERED',
        toolName: 'searchFlights',
      };
      const forwarded = relay.handleWindowMessage({
        source: mockWindow,
        data: unregMessage,
      });

      expect(forwarded).toBe(true);
      expect(mockRuntime.sendMessage).toHaveBeenCalledWith(unregMessage);
    });

    it('should ignore messages from other sources', () => {
      const toolMessage = {
        type: 'FSPEC_WEBMCP_TOOL_REGISTERED',
        tool: { name: 'hack', description: 'Injected' },
      };
      const forwarded = relay.handleWindowMessage({
        source: {},
        data: toolMessage,
      });

      expect(forwarded).toBe(false);
      expect(mockRuntime.sendMessage).not.toHaveBeenCalled();
    });

    it('should ignore messages without a type', () => {
      const forwarded = relay.handleWindowMessage({
        source: mockWindow,
        data: { foo: 'bar' },
      });

      expect(forwarded).toBe(false);
      expect(mockRuntime.sendMessage).not.toHaveBeenCalled();
    });

    it('should ignore messages with unrelated types', () => {
      const forwarded = relay.handleWindowMessage({
        source: mockWindow,
        data: { type: 'REACT_DEVTOOLS_GLOBAL_HOOK' },
      });

      expect(forwarded).toBe(false);
      expect(mockRuntime.sendMessage).not.toHaveBeenCalled();
    });
  });

  describe('Content script relays tool invocation results from main world to service worker', () => {
    it('should forward FSPEC_INVOKE_RESULT to service worker', () => {
      // @step And the main world executes the tool and posts the result back
      // (Main world execution is out of scope — we test the relay of the result)

      // @step And the content script relays the result to the service worker via chrome.runtime.sendMessage
      const resultMessage = {
        type: 'FSPEC_INVOKE_RESULT',
        correlationId: 'xyz-789',
        result: { flights: [{ airline: 'BA', price: 150 }] },
      };
      const forwarded = relay.handleWindowMessage({
        source: mockWindow,
        data: resultMessage,
      });

      expect(forwarded).toBe(true);
      expect(mockRuntime.sendMessage).toHaveBeenCalledWith(resultMessage);
    });
  });

  describe('Content script relays tool invocation requests from service worker to main world', () => {
    it('should forward FSPEC_INVOKE_TOOL to main world via window.postMessage', () => {
      // @step Given the service worker has WebMCP tools registered from tab 42
      // (Service worker setup tested in extension-message-routing.test.ts)

      // @step When the native host sends a tool call for a WebMCP tool on tab 42
      // (Native host routing tested in extension-message-routing.test.ts)

      // @step Then the service worker sends the invocation request to tab 42 via chrome.tabs.sendMessage
      // (Service worker routing tested in extension-message-routing.test.ts)

      // @step And the content script relays the request to the main world via window.postMessage
      const invokeMessage = {
        type: 'FSPEC_INVOKE_TOOL',
        correlationId: 'xyz-789',
        toolName: 'searchFlights',
        args: { from: 'London', to: 'Paris' },
      };
      const forwarded = relay.handleRuntimeMessage(invokeMessage);

      expect(forwarded).toBe(true);
      expect(mockWindow.postMessage).toHaveBeenCalledWith(invokeMessage, '*');

      // @step And the main world executes the tool and posts the result back
      // (Main world execution is out of extension test scope)

      // @step And the content script relays the result to the service worker via chrome.runtime.sendMessage
      // (Result relay tested in the INVOKE_RESULT test below and in extension-message-routing.test.ts)

      // @step And the service worker returns the result to the native host
      // (Service worker side tested in extension-message-routing.test.ts)
    });

    it('should not forward non-invoke messages to main world', () => {
      const statusMessage = { type: 'FSPEC_GET_STATUS' };
      const forwarded = relay.handleRuntimeMessage(statusMessage);

      expect(forwarded).toBe(false);
      expect(mockWindow.postMessage).not.toHaveBeenCalled();
    });
  });

  describe('Content script wires up event listeners on creation', () => {
    it('should register a window message listener', () => {
      expect(mockWindow.addEventListener).toHaveBeenCalledWith(
        'message',
        expect.any(Function)
      );
    });

    it('should register a chrome.runtime.onMessage listener', () => {
      expect(mockRuntime.onMessage.addListener).toHaveBeenCalledWith(
        expect.any(Function)
      );
    });

    it('should relay messages received via the wired-up window listener', () => {
      const toolMessage = {
        type: 'FSPEC_WEBMCP_TOOL_REGISTERED',
        tool: { name: 'test', description: 'Test tool' },
      };
      mockWindow.simulateMessage(toolMessage);

      expect(mockRuntime.sendMessage).toHaveBeenCalledWith(toolMessage);
    });

    it('should relay messages received via the wired-up runtime listener', () => {
      const invokeMessage = {
        type: 'FSPEC_INVOKE_TOOL',
        correlationId: 'abc',
        toolName: 'test',
        args: {},
      };
      mockRuntime.simulateMessage(invokeMessage);

      expect(mockWindow.postMessage).toHaveBeenCalledWith(invokeMessage, '*');
    });
  });
});
