/**
 * fspec Browser Agent - Service Worker
 *
 * Central event hub for the Chrome extension:
 * - Tool registry management
 * - Chrome event listeners (tabs, navigation)
 * - Native messaging client connection
 * - Message routing between content scripts and native host
 *
 * EXT-004: Message routing and tool registry (core implementation)
 * EXT-005: Browser control tool handlers (implemented)
 * EXT-006: WebMCP tool discovery forwarding (implemented)
 * EXT-007: Browser event notification handlers (implemented)
 */

import { createNativeConnection } from './native-connection';
import { createMessageRouter } from './message-router';
import { createToolRegistry } from './tool-registry';
import { createBrowserTools } from './browser-tools';
import { createWebMCPInjector } from './webmcp-injector';
import { createBrowserEventListeners } from './browser-events';
import { MESSAGE_TYPES } from '../types';

// Create tool registry
const toolRegistry = createToolRegistry();

// Create browser tools (native tool handlers)
const browserTools = createBrowserTools({
  tabs: chrome.tabs,
  scripting: chrome.scripting,
  windows: chrome.windows,
  userScripts:
    chrome.userScripts as unknown as import('./browser-tools').ChromeUserScriptsForTools,
  webNavigation: chrome.webNavigation,
});

// Create native connection
const nativeConnection = createNativeConnection({
  runtime: chrome.runtime,
  onMessage: message => {
    messageRouter.handleNativeMessage(message);
  },
  onDisconnect: () => {
    // Log disconnection — reconnect happens automatically
  },
  onReconnect: () => {
    // Log reconnection
  },
});

// Create message router
const messageRouter = createMessageRouter({
  runtime: chrome.runtime,
  tabs: chrome.tabs,
  connection: nativeConnection,
  toolRegistry,
  browserTools,
});

// Connect to native messaging host on service worker startup.
// This fires on first install, extension updates, and SW restarts.
nativeConnection.connect();

// Create WebMCP injector — injects main-world discovery script into pages
// when they finish loading (chrome.tabs.onUpdated status: 'complete').
// EXT-006: WebMCP tool discovery & invocation
createWebMCPInjector({
  scripting:
    chrome.scripting as unknown as import('./webmcp-injector').ChromeScriptingForInjector,
  tabs: chrome.tabs,
});

// Register browser event listeners — captures chrome.tabs events and
// forwards them as MCP notifications to the agent via SSE.
// EXT-007: Bidirectional Browser Event Notifications
createBrowserEventListeners({
  tabs: chrome.tabs as unknown as import('./browser-events').ChromeTabsEvents,
  onNotify: envelope => {
    messageRouter.forwardNotification(envelope);
  },
  toolRegistry,
  onToolsChanged: () => {
    const port = nativeConnection.getPort();
    if (port) {
      port.postMessage({
        type: MESSAGE_TYPES.TOOLS_CHANGED,
        tools: toolRegistry.getAll(),
      });
    }
  },
});

// Listen for messages from content scripts and popup
chrome.runtime.onMessage.addListener(
  (
    message: Record<string, unknown>,
    sender: chrome.runtime.MessageSender,
    sendResponse: (response?: unknown) => void
  ) => {
    const type = message.type as string | undefined;

    // Route content script messages (have sender.tab)
    if (sender.tab?.id !== undefined) {
      return messageRouter.handleContentScriptMessage(
        message,
        sender.tab.id,
        sendResponse
      );
    }

    // Route popup messages
    if (type === MESSAGE_TYPES.GET_STATUS) {
      return messageRouter.handlePopupMessage(message, sendResponse);
    }

    return false;
  }
);

export {};
