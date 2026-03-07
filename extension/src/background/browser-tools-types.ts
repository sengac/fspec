/**
 * fspec Browser Agent - Browser Tools Type Definitions
 *
 * Shared interfaces for Chrome API dependency injection,
 * MCP result formatting, and tool handler signatures.
 *
 * Extracted from browser-tools.ts for file-size compliance.
 */

/** Minimal chrome.tabs interface for dependency injection */
export interface ChromeTabsForTools {
  query: (queryInfo: Record<string, unknown>) => Promise<chrome.tabs.Tab[]>;
  update: (
    ...args: [number, Record<string, unknown>]
  ) => Promise<chrome.tabs.Tab | undefined>;
  remove: (tabId: number) => Promise<void>;
  captureVisibleTab: (
    windowId: number,
    options: Record<string, unknown>
  ) => Promise<string>;
  goBack: (tabId: number) => Promise<void>;
  goForward: (tabId: number) => Promise<void>;
  get: (tabId: number) => Promise<chrome.tabs.Tab>;
  create: (createProperties: {
    url?: string;
    active?: boolean;
    index?: number;
    windowId?: number;
    openerTabId?: number;
    pinned?: boolean;
  }) => Promise<chrome.tabs.Tab>;
  onUpdated: {
    addListener: (
      callback: (
        tabId: number,
        changeInfo: { status?: string },
        tab: chrome.tabs.Tab
      ) => void
    ) => void;
    removeListener: (
      callback: (
        tabId: number,
        changeInfo: { status?: string },
        tab: chrome.tabs.Tab
      ) => void
    ) => void;
  };
}

/** Minimal chrome.scripting interface for dependency injection */
export interface ChromeScriptingForTools {
  executeScript: <Args extends unknown[], Result>(
    injection: chrome.scripting.ScriptInjection<Args, Result>
  ) => Promise<chrome.scripting.InjectionResult<Awaited<Result>>[]>;
}

/** Minimal chrome.windows interface for dependency injection */
export interface ChromeWindowsForTools {
  update: (
    windowId: number,
    updateInfo: Record<string, unknown>
  ) => Promise<unknown>;
}

/** Minimal chrome.userScripts interface for dependency injection.
 *
 * Models the Chrome 135+ userScripts.execute() API.
 * See: user_scripts.idl InjectionResult — `error` and `result` are
 * mutually exclusive. In current Chromium (OnScriptExecuted in
 * user_scripts_api.cc), `error` is never populated for script runtime
 * errors — the promise resolves with {result: null} instead.
 * Infrastructure errors (tab not found, permissions) reject the promise.
 */
export interface ChromeUserScriptsForTools {
  configureWorld: (config: { csp: string }) => Promise<void>;
  execute: (injection: {
    target: { tabId: number };
    world: string;
    js: Array<{ code: string }>;
  }) => Promise<Array<{ result?: unknown; error?: string }>>;
}

/** Frame info returned by chrome.webNavigation.getAllFrames */
export interface FrameInfo {
  frameId: number;
  parentFrameId: number;
  url: string;
  documentId: string;
  documentLifecycle: string;
  frameType: string;
  errorOccurred: boolean;
}

/** Minimal chrome.webNavigation interface for dependency injection */
export interface ChromeWebNavigationForTools {
  getAllFrames: (details: { tabId: number }) => Promise<FrameInfo[] | null>;
}

export interface BrowserToolsDeps {
  tabs: ChromeTabsForTools;
  scripting: ChromeScriptingForTools;
  windows: ChromeWindowsForTools;
  userScripts?: ChromeUserScriptsForTools;
  webNavigation?: ChromeWebNavigationForTools;
}

/** MCP content item */
interface McpTextContent {
  type: 'text';
  text: string;
}

interface McpImageContent {
  type: 'image';
  data: string;
  mimeType: string;
}

export type McpContent = McpTextContent | McpImageContent;

/** MCP tool result */
export interface McpToolResult {
  content: McpContent[];
  isError?: boolean;
}

/** Tool handler function type */
export type ToolHandler = (
  args: Record<string, unknown>
) => Promise<McpToolResult>;

export interface BrowserToolsAPI {
  getHandler: (toolName: string) => ToolHandler | undefined;
  getToolNames: () => string[];
}

export function textResult(data: unknown): McpToolResult {
  return {
    content: [
      {
        type: 'text',
        text: typeof data === 'string' ? data : JSON.stringify(data),
      },
    ],
  };
}

export function errorResult(message: string): McpToolResult {
  return {
    isError: true,
    content: [{ type: 'text', text: message }],
  };
}
