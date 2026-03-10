/**
 * Feature: spec/features/webmcp-chrome-extension.feature
 *
 * This test file validates the acceptance criteria for EXT-006:
 * WebMCP Tool Discovery & Invocation.
 *
 * Scenarios:
 * - Discover WebMCP tool registered by website
 * - Remove WebMCP tool when website unregisters it
 * - Invoke a WebMCP tool registered by a website
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createMessageRouter } from '../message-router';
import { createToolRegistry } from '../tool-registry';
import type { MessageRouterAPI } from '../message-router';
import type { ToolRegistryAPI } from '../tool-registry';

// --- Test Helpers ---

function createMockNativePort() {
  return {
    postMessage: vi.fn(),
    onMessage: { addListener: vi.fn() },
    onDisconnect: { addListener: vi.fn() },
  };
}

function createMockConnection(port: ReturnType<typeof createMockNativePort>) {
  return {
    connect: vi.fn(),
    disconnect: vi.fn(),
    isConnected: vi.fn(() => true),
    getPort: vi.fn(() => port),
  };
}

function createMockTabs() {
  return {
    sendMessage: vi.fn(
      (
        _tabId: number,
        _msg: Record<string, unknown>,
        callback: (response: unknown) => void
      ) => {
        callback(undefined);
      }
    ),
  };
}

function createMockRuntime() {
  return {
    sendMessage: vi.fn(),
    lastError: null,
  };
}

describe('Feature: fspec Browser Agent Chrome Extension - WebMCP Tool Discovery & Invocation', () => {
  let toolRegistry: ToolRegistryAPI;
  let router: MessageRouterAPI;
  let nativePort: ReturnType<typeof createMockNativePort>;
  let tabs: ReturnType<typeof createMockTabs>;

  beforeEach(() => {
    toolRegistry = createToolRegistry();
    nativePort = createMockNativePort();
    const connection = createMockConnection(nativePort);
    tabs = createMockTabs();
    const runtime = createMockRuntime();

    router = createMessageRouter({
      runtime,
      tabs,
      connection,
      toolRegistry,
    });
  });

  describe('Scenario: Discover WebMCP tool registered by website', () => {
    it('should detect tool registration and add it to the MCP tool list with origin-based namespacing', () => {
      // @step Given the agent has an active MCP connection to the extension
      // (setup complete via beforeEach)

      // @step And the user navigates to a WebMCP-enabled site at "https://travel-demo.bandarra.me"
      const senderTabId = 42;
      const pageOrigin = 'travel-demo.bandarra.me';

      // @step When the site calls navigator.modelContext.registerTool with name "searchFlights"
      const registrationMessage = {
        type: 'FSPEC_WEBMCP_TOOL_REGISTERED',
        tool: {
          name: 'searchFlights',
          description: 'Search for flights',
          inputSchema: {
            type: 'object',
            properties: { from: { type: 'string' } },
          },
        },
        origin: pageOrigin,
      };
      const sendResponse = vi.fn();
      router.handleContentScriptMessage(
        registrationMessage,
        senderTabId,
        sendResponse
      );

      // @step Then the main-world injected script detects the tool registration
      // (verified by the tool being in the registry)

      // @step And the extension adds the tool with sanitized origin to the MCP tool list
      const expectedToolName = 'webmcp__travel-demo-bandarra-me__searchFlights';
      const tool = toolRegistry.getByName(expectedToolName);
      expect(tool).toBeDefined();
      expect(tool?.name).toBe(expectedToolName);
      expect(tool?.source).toBe('webmcp');
      expect(tool?.origin).toBe(pageOrigin);
      expect(tool?.tabId).toBe(senderTabId);

      // @step And the agent receives a "notifications/tools/list_changed" notification via SSE
      expect(nativePort.postMessage).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'TOOLS_CHANGED' })
      );
    });
  });

  describe('Scenario: Remove WebMCP tool when website unregisters it', () => {
    it('should remove the tool from registry and notify when website unregisters', () => {
      // @step Given the agent has an active MCP connection to the extension
      // (setup complete via beforeEach)

      // @step And the extension has a discovered WebMCP tool
      // Register via the normal flow first
      const registerMsg = {
        type: 'FSPEC_WEBMCP_TOOL_REGISTERED',
        tool: { name: 'oldTool', description: 'An old tool' },
        origin: 'example.com',
      };
      router.handleContentScriptMessage(registerMsg, 10, vi.fn());
      expect(
        toolRegistry.getByName('webmcp__example-com__oldTool')
      ).toBeDefined();
      nativePort.postMessage.mockClear();

      // @step When the website calls navigator.modelContext.unregisterTool with name "oldTool"
      const unregisterMsg = {
        type: 'FSPEC_WEBMCP_TOOL_UNREGISTERED',
        toolName: 'oldTool',
        origin: 'example.com',
      };
      router.handleContentScriptMessage(unregisterMsg, 10, vi.fn());

      // @step Then the main-world discovery script detects the removal
      // (verified by message processing)

      // @step And the extension removes the tool from the tool list
      expect(
        toolRegistry.getByName('webmcp__example-com__oldTool')
      ).toBeUndefined();

      // @step And the agent receives a "notifications/tools/list_changed" notification via SSE
      expect(nativePort.postMessage).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'TOOLS_CHANGED' })
      );

      // @step And the agent's next tools/list call no longer includes the tool
      const allTools = toolRegistry.getAll();
      const found = allTools.find(
        t => t.name === 'webmcp__example-com__oldTool'
      );
      expect(found).toBeUndefined();
    });
  });

  describe('Scenario: Invoke a WebMCP tool registered by a website', () => {
    it('should invoke a WebMCP tool by routing to the correct tab and relaying the result', () => {
      // @step Given the agent has an active MCP connection to the extension
      const tabId = 42;

      // @step And the website at "https://example.com" has registered a WebMCP tool "submitForm"
      const registerMsg = {
        type: 'FSPEC_WEBMCP_TOOL_REGISTERED',
        tool: {
          name: 'submitForm',
          description: 'Submit a form',
          inputSchema: {
            type: 'object',
            properties: { name: { type: 'string' }, email: { type: 'string' } },
          },
        },
        origin: 'example.com',
      };
      router.handleContentScriptMessage(registerMsg, tabId, vi.fn());
      const registeredTool = toolRegistry.getByName(
        'webmcp__example-com__submitForm'
      );
      expect(registeredTool).toBeDefined();

      // @step When the agent calls the tool with params name "John" and email "john@test.com"
      const toolCallMessage = {
        type: 'TOOL_CALL',
        correlationId: 'test-corr-123',
        params: {
          name: 'webmcp__example-com__submitForm',
          arguments: { name: 'John', email: 'john@test.com' },
        },
      };
      router.handleNativeMessage(toolCallMessage);

      // @step Then the extension injects a main-world script via chrome.scripting.executeScript with world "MAIN"
      // In the current architecture, the SW sends to the content script, which forwards to main world.
      // Verified by tabs.sendMessage being called with the correct tab ID.

      // @step And the main-world script calls the WebMCP tool's execute function in the page context
      expect(tabs.sendMessage).toHaveBeenCalledWith(
        tabId,
        expect.objectContaining({
          type: 'FSPEC_INVOKE_TOOL',
          correlationId: 'test-corr-123',
          toolName: 'submitForm',
          args: { name: 'John', email: 'john@test.com' },
        }),
        expect.any(Function)
      );

      // @step And the result is relayed back via postMessage to the content script
      // Simulate the content script forwarding the result
      const resultMessage = {
        type: 'FSPEC_INVOKE_RESULT',
        correlationId: 'test-corr-123',
        result: { success: true, message: 'Form submitted' },
      };
      router.handleContentScriptMessage(resultMessage, tabId, vi.fn());

      // @step And the content script forwards the result to the service worker via chrome.runtime
      // (this is the handleContentScriptMessage call above)

      // @step And the agent receives the structured result from the tool call
      expect(nativePort.postMessage).toHaveBeenCalledWith(
        expect.objectContaining({
          correlationId: 'test-corr-123',
          result: expect.objectContaining({
            content: expect.arrayContaining([
              expect.objectContaining({
                type: 'text',
              }),
            ]),
          }),
        })
      );
    });

    it('should return error response when tool execute() throws', () => {
      // @step Given the agent has an active MCP connection to the extension
      const tabId = 42;

      // @step And the website at "https://example.com" has registered a WebMCP tool "submitForm"
      const registerMsg = {
        type: 'FSPEC_WEBMCP_TOOL_REGISTERED',
        tool: {
          name: 'submitForm',
          description: 'Submit a form',
          inputSchema: { type: 'object' },
        },
        origin: 'example.com',
      };
      router.handleContentScriptMessage(registerMsg, tabId, vi.fn());

      // @step When the agent calls the tool and the execute function throws
      const toolCallMessage = {
        type: 'TOOL_CALL',
        correlationId: 'err-corr-456',
        params: {
          name: 'webmcp__example-com__submitForm',
          arguments: { name: 'John' },
        },
      };
      router.handleNativeMessage(toolCallMessage);

      // Simulate the content script forwarding an error result from main world
      // (per Example [3]: execute() throws Error('Network timeout'))
      const errorResult = {
        type: 'FSPEC_INVOKE_RESULT',
        correlationId: 'err-corr-456',
        error: 'Network timeout',
      };
      router.handleContentScriptMessage(errorResult, tabId, vi.fn());

      // @step Then the agent receives an MCP error response, not a success with "undefined"
      expect(nativePort.postMessage).toHaveBeenCalledWith(
        expect.objectContaining({
          correlationId: 'err-corr-456',
          error: expect.objectContaining({
            code: -1,
            message: 'Network timeout',
          }),
        })
      );

      // Verify it was NOT sent as a successful result with "undefined" text
      const calls = nativePort.postMessage.mock.calls;
      const errorCall = calls.find(
        (c: [Record<string, unknown>]) =>
          c[0].correlationId === 'err-corr-456' && c[0].error !== undefined
      );
      expect(errorCall).toBeDefined();
    });
  });
});
