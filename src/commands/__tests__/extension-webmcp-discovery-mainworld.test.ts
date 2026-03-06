/**
 * Feature: spec/features/webmcp-chrome-extension.feature
 *
 * This test file validates the main-world discovery function (EXT-006).
 * The discovery function is injected into pages' MAIN world and:
 *   - Monkey-patches navigator.modelContext to intercept registerTool/unregisterTool
 *   - Stores execute functions for later invocation
 *   - Relays invocation requests, handling sync/async/error results
 *   - Guards against double-injection
 *
 * Rules covered:
 *   [0] Main-world discovery script MUST intercept navigator.modelContext.registerTool()
 *       and unregisterTool() calls by monkey-patching
 *   [7] Tool invocation errors from execute() MUST be caught and returned as MCP error
 *       responses, not silently swallowed
 *
 * Example covered:
 *   [3] Tool execute() throws Error('Network timeout') → error relayed back
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { webmcpDiscoveryFunction } from '../../../extension/src/content/webmcp-discovery';

// --- JSDOM Setup Helpers ---

/** Helper to dispatch a tool invocation message that the listener will accept */
function dispatchInvokeMessage(data: Record<string, unknown>): void {
  // Dispatch directly as a MessageEvent — the listener checks event.source
  const event = new MessageEvent('message', { data, source: window });
  window.dispatchEvent(event);
}
let capturedMessages: Array<{ data: Record<string, unknown>; origin: string }> = [];

/** Store original modelContext so we can clean up */
let originalModelContext: unknown;
let originalGuard: unknown;

/** Track listeners so we can remove them between tests */
let registeredListeners: Array<{ type: string; handler: EventListenerOrEventListenerObject }> = [];
const originalAddEventListener = window.addEventListener.bind(window);

function setupWindowCapture(): void {
  capturedMessages = [];
  registeredListeners = [];

  // Capture postMessages for assertion only — do NOT re-dispatch
  vi.spyOn(window, 'postMessage').mockImplementation((msg: unknown, targetOrigin: string) => {
    capturedMessages.push({ data: msg as Record<string, unknown>, origin: targetOrigin });
  });

  // Track addEventListener calls so we can remove listeners in cleanup
  vi.spyOn(window, 'addEventListener').mockImplementation(
    (type: string, handler: EventListenerOrEventListenerObject) => {
      registeredListeners.push({ type, handler });
      originalAddEventListener(type, handler);
    }
  );
}

function cleanupWindow(): void {
  // Remove all event listeners registered during this test
  for (const { type, handler } of registeredListeners) {
    window.removeEventListener(type, handler);
  }
  registeredListeners = [];

  // Reset the guard
  delete (window as Record<string, unknown>).__fspec_webmcp_discovery_active;
  // Restore navigator.modelContext
  const nav = navigator as Record<string, unknown>;
  if (originalModelContext !== undefined) {
    nav.modelContext = originalModelContext;
  } else {
    delete nav.modelContext;
  }
}

describe('Feature: fspec WebMCP Chrome Extension — Main-World Discovery Function', () => {
  beforeEach(() => {
    const nav = navigator as Record<string, unknown>;
    originalModelContext = nav.modelContext;
    originalGuard = (window as Record<string, unknown>).__fspec_webmcp_discovery_active;
    delete (window as Record<string, unknown>).__fspec_webmcp_discovery_active;
    delete nav.modelContext;
    setupWindowCapture();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    cleanupWindow();
  });

  describe('Rule [0]: Intercepting navigator.modelContext.registerTool', () => {
    it('should intercept registerTool calls and post FSPEC_WEBMCP_TOOL_REGISTERED', () => {
      // @step Given the discovery function is injected into the page
      webmcpDiscoveryFunction();
      const nav = navigator as Record<string, unknown>;

      // Trigger the lazy interceptor by assigning modelContext
      nav.modelContext = {};

      // @step When a site calls navigator.modelContext.registerTool
      const mc = nav.modelContext as Record<string, (toolDef: Record<string, unknown>) => unknown>;
      mc.registerTool({
        name: 'searchFlights',
        description: 'Search for flights',
        inputSchema: { type: 'object' },
        execute: () => ({ flights: [] }),
      });

      // @step Then the discovery script posts FSPEC_WEBMCP_TOOL_REGISTERED
      const regMsg = capturedMessages.find((m) => m.data.type === 'FSPEC_WEBMCP_TOOL_REGISTERED');
      expect(regMsg).toBeDefined();
      expect(regMsg?.data.tool).toEqual(
        expect.objectContaining({
          name: 'searchFlights',
          description: 'Search for flights',
        })
      );
      expect(regMsg?.data.origin).toBe('localhost');
    });

    it('should intercept unregisterTool calls and post FSPEC_WEBMCP_TOOL_UNREGISTERED', () => {
      // @step Given the discovery function is injected and a tool was registered
      webmcpDiscoveryFunction();
      const nav = navigator as Record<string, unknown>;
      nav.modelContext = {};
      const mc = nav.modelContext as Record<string, (arg: unknown) => unknown>;
      mc.registerTool({ name: 'myTool', description: 'test', execute: () => 'ok' });

      capturedMessages = [];

      // @step When the site calls unregisterTool
      mc.unregisterTool('myTool');

      // @step Then the discovery script posts FSPEC_WEBMCP_TOOL_UNREGISTERED
      const unregMsg = capturedMessages.find((m) => m.data.type === 'FSPEC_WEBMCP_TOOL_UNREGISTERED');
      expect(unregMsg).toBeDefined();
      expect(unregMsg?.data.toolName).toBe('myTool');
    });

    it('should intercept modelContext set lazily via defineProperty', () => {
      // @step Given the discovery script runs before navigator.modelContext exists
      webmcpDiscoveryFunction();
      const nav = navigator as Record<string, unknown>;

      // @step When the site sets navigator.modelContext after the fact
      nav.modelContext = {};
      const mc = nav.modelContext as Record<string, (arg: Record<string, unknown>) => unknown>;
      mc.registerTool({ name: 'lazyTool', description: 'lazy', execute: () => 'ok' });

      // @step Then the registration is still captured
      const regMsg = capturedMessages.find(
        (m) => m.data.type === 'FSPEC_WEBMCP_TOOL_REGISTERED'
          && (m.data.tool as Record<string, unknown>)?.name === 'lazyTool'
      );
      expect(regMsg).toBeDefined();
    });

    it('should call original registerTool if it already existed', () => {
      // @step Given navigator.modelContext already exists with a registerTool
      const originalRegister = vi.fn();
      const nav = navigator as Record<string, unknown>;
      nav.modelContext = { registerTool: originalRegister, unregisterTool: vi.fn() };

      // @step When the discovery function runs and a tool is registered
      webmcpDiscoveryFunction();
      const mc = nav.modelContext as Record<string, (arg: Record<string, unknown>) => unknown>;
      mc.registerTool({ name: 'wrappedTool', description: 'test' });

      // @step Then the original registerTool is also called
      expect(originalRegister).toHaveBeenCalledTimes(1);
    });
  });

  describe('Rule [7]: Tool invocation error handling', () => {
    it('should catch sync errors from execute() and post error result', () => {
      // @step Given a tool is registered with an execute that throws
      webmcpDiscoveryFunction();
      const nav = navigator as Record<string, unknown>;
      nav.modelContext = {};
      const mc = nav.modelContext as Record<string, (arg: Record<string, unknown>) => unknown>;
      mc.registerTool({
        name: 'failTool',
        description: 'always fails',
        execute: () => {
          throw new Error('Network timeout');
        },
      });

      capturedMessages = [];

      // @step When an invocation request is dispatched for this tool
      dispatchInvokeMessage({
        type: 'FSPEC_INVOKE_TOOL',
        correlationId: 'err-123',
        toolName: 'failTool',
        args: {},
      });

      // @step Then an FSPEC_INVOKE_RESULT with error is posted
      const errMsg = capturedMessages.find(
        (m) => m.data.type === 'FSPEC_INVOKE_RESULT' && m.data.correlationId === 'err-123'
      );
      expect(errMsg).toBeDefined();
      expect(errMsg?.data.error).toBe('Network timeout');
      expect(errMsg?.data.result).toBeUndefined();
    });

    it('should catch async errors from execute() and post error result', async () => {
      // @step Given a tool with an async execute that rejects
      webmcpDiscoveryFunction();
      const nav = navigator as Record<string, unknown>;
      nav.modelContext = {};
      const mc = nav.modelContext as Record<string, (arg: Record<string, unknown>) => unknown>;
      mc.registerTool({
        name: 'asyncFailTool',
        description: 'async failure',
        execute: () => Promise.reject(new Error('Async timeout')),
      });

      capturedMessages = [];

      // @step When an invocation request is dispatched
      dispatchInvokeMessage({
        type: 'FSPEC_INVOKE_TOOL',
        correlationId: 'async-err-456',
        toolName: 'asyncFailTool',
        args: {},
      });

      // Wait for the promise rejection to be caught
      await new Promise((r) => setTimeout(r, 50));

      // @step Then an FSPEC_INVOKE_RESULT with error is posted
      const errMsg = capturedMessages.find(
        (m) => m.data.type === 'FSPEC_INVOKE_RESULT' && m.data.correlationId === 'async-err-456'
      );
      expect(errMsg).toBeDefined();
      expect(errMsg?.data.error).toBe('Async timeout');
    });

    it('should return tool-not-found error for unknown tool invocations', () => {
      // @step Given the discovery script is running with no tools registered
      webmcpDiscoveryFunction();
      const nav = navigator as Record<string, unknown>;
      nav.modelContext = {};

      capturedMessages = [];

      // @step When an invocation for a non-existent tool arrives
      dispatchInvokeMessage({
        type: 'FSPEC_INVOKE_TOOL',
        correlationId: 'notfound-789',
        toolName: 'nonExistentTool',
        args: {},
      });

      // @step Then an error result is posted
      const errMsg = capturedMessages.find(
        (m) => m.data.type === 'FSPEC_INVOKE_RESULT' && m.data.correlationId === 'notfound-789'
      );
      expect(errMsg).toBeDefined();
      expect(errMsg?.data.error).toContain('not found');
    });
  });

  describe('Successful invocation', () => {
    it('should relay sync execute result back via postMessage', () => {
      // @step Given a tool with a synchronous execute function
      webmcpDiscoveryFunction();
      const nav = navigator as Record<string, unknown>;
      nav.modelContext = {};
      const mc = nav.modelContext as Record<string, (arg: Record<string, unknown>) => unknown>;
      mc.registerTool({
        name: 'syncTool',
        description: 'sync',
        execute: (args: Record<string, unknown>) => ({ echo: args }),
      });

      capturedMessages = [];

      // @step When the tool is invoked
      dispatchInvokeMessage({
        type: 'FSPEC_INVOKE_TOOL',
        correlationId: 'sync-001',
        toolName: 'syncTool',
        args: { hello: 'world' },
      });

      // @step Then the result is relayed back
      const resultMsg = capturedMessages.find(
        (m) => m.data.type === 'FSPEC_INVOKE_RESULT' && m.data.correlationId === 'sync-001'
      );
      expect(resultMsg).toBeDefined();
      expect(resultMsg?.data.result).toEqual({ echo: { hello: 'world' } });
      expect(resultMsg?.data.error).toBeUndefined();
    });

    it('should relay async execute result back via postMessage', async () => {
      // @step Given a tool with an async execute function
      webmcpDiscoveryFunction();
      const nav = navigator as Record<string, unknown>;
      nav.modelContext = {};
      const mc = nav.modelContext as Record<string, (arg: Record<string, unknown>) => unknown>;
      mc.registerTool({
        name: 'asyncTool',
        description: 'async',
        execute: () => Promise.resolve({ flights: [{ id: 1 }] }),
      });

      capturedMessages = [];

      // @step When the tool is invoked
      dispatchInvokeMessage({
        type: 'FSPEC_INVOKE_TOOL',
        correlationId: 'async-001',
        toolName: 'asyncTool',
        args: {},
      });

      await new Promise((r) => setTimeout(r, 50));

      // @step Then the resolved result is relayed back
      const resultMsg = capturedMessages.find(
        (m) => m.data.type === 'FSPEC_INVOKE_RESULT' && m.data.correlationId === 'async-001'
      );
      expect(resultMsg).toBeDefined();
      expect(resultMsg?.data.result).toEqual({ flights: [{ id: 1 }] });
    });
  });

  describe('Double-injection guard', () => {
    it('should not run twice on the same page', () => {
      // @step Given the discovery function has already been called
      webmcpDiscoveryFunction();
      const firstCallMessages = capturedMessages.length;

      // Store reference to first modelContext
      const nav = navigator as Record<string, unknown>;
      const firstMc = nav.modelContext;

      // @step When it is called again
      webmcpDiscoveryFunction();

      // @step Then it exits early without re-patching
      // The modelContext should still be the same object (not re-wrapped)
      expect(nav.modelContext).toBe(firstMc);
    });
  });
});
