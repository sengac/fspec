/**
 * fspec Browser Agent - Message Router
 *
 * Routes messages between:
 * - Native host (via native messaging port)
 * - Content scripts (via chrome.runtime.onMessage)
 * - Popup (via chrome.runtime.onMessage)
 *
 * Implemented by: EXT-004
 */

import { MCP_DEFAULT_PORT } from '../server/mcp-constants';
import { MESSAGE_TYPES } from '../types';
import { buildWebmcpToolName, parseWebmcpToolName } from './webmcp-naming';
import type { NativeConnectionAPI } from './native-connection';
import type { ToolRegistryAPI } from './tool-registry';
import type { BrowserToolsAPI } from './browser-tools';
import type { ToolRegistryEntry } from '../types';
import type { NotificationEnvelope } from './browser-events';

/** Minimal chrome.tabs interface for dependency injection */
export interface ChromeTabsLike {
  sendMessage: (
    tabId: number,
    message: Record<string, unknown>,
    callback: (response: unknown) => void
  ) => void;
}

/** Minimal chrome.runtime interface for the router */
export interface ChromeRuntimeForRouter {
  sendMessage: (message: Record<string, unknown>) => void;
  lastError?: { message?: string } | null;
}

export interface MessageRouterOptions {
  runtime: ChromeRuntimeForRouter;
  tabs: ChromeTabsLike;
  connection: NativeConnectionAPI;
  toolRegistry: ToolRegistryAPI;
  browserTools?: BrowserToolsAPI;
}

export interface MessageRouterAPI {
  handleNativeMessage: (message: Record<string, unknown>) => void;
  handleContentScriptMessage: (
    message: Record<string, unknown>,
    senderTabId: number,
    sendResponse: (response: unknown) => void
  ) => boolean;
  handlePopupMessage: (
    message: Record<string, unknown>,
    sendResponse: (response: unknown) => void
  ) => boolean;
  forwardNotification: (envelope: NotificationEnvelope) => void;
}

export function createMessageRouter(
  options: MessageRouterOptions
): MessageRouterAPI {
  const { tabs, connection, toolRegistry, browserTools } = options;

  function sendToNativeHost(message: Record<string, unknown>): void {
    const port = connection.getPort();
    if (port) {
      port.postMessage(message);
    }
  }

  function notifyToolsChanged(): void {
    sendToNativeHost({
      type: MESSAGE_TYPES.TOOLS_CHANGED,
      tools: toolRegistry.getAll(),
    });
  }

  return {
    handleNativeMessage(message: Record<string, unknown>): void {
      const type = message.type as string | undefined;
      const correlationId = message.correlationId as string | undefined;

      // Handle tool call from native host
      if (type === MESSAGE_TYPES.TOOL_CALL && correlationId) {
        const params = message.params as
          | { name?: string; arguments?: Record<string, unknown> }
          | undefined;
        const toolName = params?.name;

        if (!toolName) {
          sendToNativeHost({
            correlationId,
            error: { code: -1, message: 'Missing tool name' },
          });
          return;
        }

        // Check if it's a WebMCP tool (has a registered entry with tabId)
        const tool = toolRegistry.getByName(toolName);
        if (tool?.source === 'webmcp' && tool.tabId !== undefined) {
          // Route to content script on the correct tab
          // Strip the "webmcp__<origin>__" prefix to get the original tool name
          const parsed = parseWebmcpToolName(toolName);
          const originalName = parsed ? parsed.toolName : toolName;

          tabs.sendMessage(
            tool.tabId,
            {
              type: MESSAGE_TYPES.INVOKE_TOOL,
              correlationId,
              toolName: originalName,
              args: params?.arguments ?? {},
            },
            () => {
              // Response comes back via handleContentScriptMessage
            }
          );
        } else {
          // Native browser tool — dispatch to browser tools handler
          if (browserTools) {
            const handler = browserTools.getHandler(toolName);
            if (handler) {
              handler(params?.arguments ?? {})
                .then(result => {
                  sendToNativeHost({
                    correlationId,
                    result,
                  });
                })
                .catch((err: Error) => {
                  sendToNativeHost({
                    correlationId,
                    error: { code: -1, message: err.message },
                  });
                });
              return;
            }
          }
          // No handler found
          sendToNativeHost({
            correlationId,
            error: {
              code: -32601,
              message: `No handler registered for tool: ${toolName}`,
            },
          });
        }
        return;
      }
    },

    handleContentScriptMessage(
      message: Record<string, unknown>,
      senderTabId: number,
      _sendResponse: (response: unknown) => void
    ): boolean {
      const type = message.type as string | undefined;

      // Handle WebMCP tool registration
      if (type === MESSAGE_TYPES.TOOL_REGISTERED) {
        const toolMeta = message.tool as
          | {
              name: string;
              description: string;
              inputSchema?: Record<string, unknown>;
            }
          | undefined;
        if (toolMeta) {
          const origin = (message.origin as string) ?? '';
          const namespacedName = buildWebmcpToolName(origin, toolMeta.name);
          const entry: ToolRegistryEntry = {
            name: namespacedName,
            description: toolMeta.description,
            inputSchema: toolMeta.inputSchema,
            source: 'webmcp',
            origin: origin || undefined,
            tabId: senderTabId,
          };
          toolRegistry.register(entry);
          notifyToolsChanged();
        }
        return true;
      }

      // Handle WebMCP tool unregistration
      if (type === MESSAGE_TYPES.TOOL_UNREGISTERED) {
        const toolName = message.toolName as string | undefined;
        const origin = (message.origin as string) ?? '';
        if (toolName) {
          const namespacedName = buildWebmcpToolName(origin, toolName);
          toolRegistry.unregister(namespacedName);
          notifyToolsChanged();
        }
        return true;
      }

      // Handle tool invocation result from content script
      if (type === MESSAGE_TYPES.INVOKE_RESULT) {
        const correlationId = message.correlationId as string | undefined;
        if (correlationId) {
          // Check if the main-world script reported an error
          if (message.error !== undefined) {
            sendToNativeHost({
              correlationId,
              error: {
                code: -1,
                message:
                  typeof message.error === 'string'
                    ? message.error
                    : JSON.stringify(message.error),
              },
            });
          } else {
            sendToNativeHost({
              correlationId,
              result: {
                content: [
                  {
                    type: 'text',
                    text:
                      typeof message.result === 'string'
                        ? message.result
                        : JSON.stringify(message.result),
                  },
                ],
              },
            });
          }
        }
        return true;
      }

      return false;
    },

    handlePopupMessage(
      message: Record<string, unknown>,
      sendResponse: (response: unknown) => void
    ): boolean {
      const type = message.type as string | undefined;

      if (type === MESSAGE_TYPES.GET_STATUS) {
        const allTools = toolRegistry.getAll();
        sendResponse({
          connected: true,
          nativeConnected: connection.isConnected(),
          toolCount: allTools.length,
          port: MCP_DEFAULT_PORT,
          clientCount: 0,
          tools: allTools.map(t => ({
            name: t.name,
            source: t.source,
            origin: t.origin,
            tabId: t.tabId,
          })),
        });
        return true;
      }

      return false;
    },

    forwardNotification(envelope: NotificationEnvelope): void {
      sendToNativeHost({
        type: MESSAGE_TYPES.NOTIFICATION,
        ...envelope,
      });
    },
  };
}
