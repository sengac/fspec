/**
 * fspec Browser Agent - Native Browser Control Tools
 *
 * Implements the 12 native browser control tool handlers:
 * - browser_navigate, browser_screenshot, browser_list_tabs,
 *   browser_execute_script, browser_switch_tab, browser_close_tab,
 *   browser_get_page_content, browser_click_element, browser_fill_form,
 *   browser_go_back, browser_go_forward, browser_create_tab
 *
 * Each handler is an async function that accepts tool arguments and returns
 * an MCP-formatted result (content array with text or image items).
 *
 * Implemented by: EXT-005, EXT-011
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

export interface BrowserToolsDeps {
  tabs: ChromeTabsForTools;
  scripting: ChromeScriptingForTools;
  windows: ChromeWindowsForTools;
  userScripts?: ChromeUserScriptsForTools;
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

type McpContent = McpTextContent | McpImageContent;

/** MCP tool result */
interface McpToolResult {
  content: McpContent[];
  isError?: boolean;
}

/** Tool handler function type */
type ToolHandler = (args: Record<string, unknown>) => Promise<McpToolResult>;

export interface BrowserToolsAPI {
  getHandler: (toolName: string) => ToolHandler | undefined;
  getToolNames: () => string[];
}

function textResult(data: unknown): McpToolResult {
  return {
    content: [
      {
        type: 'text',
        text: typeof data === 'string' ? data : JSON.stringify(data),
      },
    ],
  };
}

function errorResult(message: string): McpToolResult {
  return {
    isError: true,
    content: [{ type: 'text', text: message }],
  };
}

/**
 * Sentinel value returned by the try-catch wrapper when user code throws.
 *
 * Chrome's userScripts.execute() silently returns {result: null} for script
 * throws (see PausableScriptExecutor in Blink — GetSuccessValueOrEmpty()
 * returns empty V8 value on throw, HandleResults converts to nullopt,
 * OnScriptExecuted in user_scripts_api.cc sees empty error + null value →
 * resolves with {result: null}). To surface errors, we wrap code in
 * try-catch and return this sentinel.
 */
interface ScriptErrorSentinel {
  __fspec_error: true;
  message: string;
}

function isScriptErrorSentinel(value: unknown): value is ScriptErrorSentinel {
  return (
    typeof value === 'object' &&
    value !== null &&
    '__fspec_error' in value &&
    (value as ScriptErrorSentinel).__fspec_error === true
  );
}

/**
 * Wraps user code in a try-catch IIFE so that runtime JS errors are captured
 * as a sentinel object rather than silently swallowed by Chrome.
 *
 * Uses eval() inside the USER_SCRIPT world which has unsafe-eval enabled
 * via configureWorld(). The IIFE returns the eval result on success or
 * the error sentinel on failure.
 */
function wrapCodeWithErrorHandling(code: string): string {
  const escaped = JSON.stringify(code);
  return `(function(){try{return eval(${escaped})}catch(e){return{__fspec_error:true,message:e instanceof Error?e.message:String(e)}}})()`;
}

export function createBrowserTools(deps: BrowserToolsDeps): BrowserToolsAPI {
  const { tabs, scripting, windows, userScripts } = deps;

  // Configure USER_SCRIPT world with permissive CSP on startup
  if (userScripts) {
    void userScripts.configureWorld({
      csp: "script-src 'self' 'unsafe-eval' 'unsafe-inline'",
    });
  }

  async function resolveTabId(tabId?: number): Promise<number> {
    if (tabId !== undefined) {
      return tabId;
    }
    const activeTabs = await tabs.query({ active: true, currentWindow: true });
    if (activeTabs.length === 0 || activeTabs[0].id === undefined) {
      throw new Error('No active tab found');
    }
    return activeTabs[0].id;
  }

  const handlers = new Map<string, ToolHandler>();

  /** Wait for a tab to finish loading after navigation */
  function waitForTabLoad(
    targetTabId: number,
    timeoutMs = 30000
  ): Promise<chrome.tabs.Tab> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        tabs.onUpdated.removeListener(listener);
        // On timeout, return whatever state the tab is in rather than failing
        tabs
          .get(targetTabId)
          .then(resolve)
          .catch(() => reject(new Error('Navigation timed out')));
      }, timeoutMs);

      const listener = (
        tabId: number,
        changeInfo: { status?: string },
        tab: chrome.tabs.Tab
      ): void => {
        if (tabId === targetTabId && changeInfo.status === 'complete') {
          clearTimeout(timer);
          tabs.onUpdated.removeListener(listener);
          resolve(tab);
        }
      };

      tabs.onUpdated.addListener(listener);
    });
  }

  // browser_navigate
  handlers.set('browser_navigate', async args => {
    const url = args.url as string;
    if (!url) {
      return errorResult('Missing required parameter: url');
    }
    const tabId = await resolveTabId(args.tabId as number | undefined);
    await tabs.update(tabId, { url });
    const completedTab = await waitForTabLoad(tabId);
    return textResult({
      url: completedTab.url ?? url,
      title: completedTab.title ?? '',
    });
  });

  // browser_screenshot
  handlers.set('browser_screenshot', async args => {
    const tabId = await resolveTabId(args.tabId as number | undefined);
    const tab = await tabs.get(tabId);
    const windowId = tab.windowId ?? 0;
    const dataUrl = await tabs.captureVisibleTab(windowId, { format: 'png' });
    // dataUrl is "data:image/png;base64,..." — extract the base64 portion
    const base64Data = dataUrl.replace(/^data:image\/png;base64,/, '');
    return {
      content: [
        { type: 'image' as const, data: base64Data, mimeType: 'image/png' },
      ],
    };
  });

  // browser_list_tabs
  handlers.set('browser_list_tabs', async () => {
    const allTabs = await tabs.query({});
    const tabList = allTabs.map(t => ({
      id: t.id,
      url: t.url,
      title: t.title,
      active: t.active,
    }));
    return textResult(tabList);
  });

  // browser_execute_script
  //
  // Chrome's userScripts.execute() silently returns {result: null} when the
  // injected code throws (see PausableScriptExecutor::HandleResults in Blink:
  // GetSuccessValueOrEmpty() returns empty on throw, HandleResults skips
  // conversion, callback fires with nullopt → base::Value() NONE → JS null).
  //
  // The only way to surface runtime JS errors is to wrap the user's code
  // in a try-catch. Since we configured the USER_SCRIPT world with
  // unsafe-eval via configureWorld(), eval() works here.
  //
  // Infrastructure errors (invalid tab, permissions) DO reject the promise,
  // so the outer try/catch handles those.
  handlers.set('browser_execute_script', async args => {
    const code = args.code as string;
    if (!code) {
      return errorResult('Missing required parameter: code');
    }
    if (!userScripts) {
      return errorResult(
        'browser_execute_script requires the chrome.userScripts API. ' +
          'Please enable "Allow User Scripts" in the extension settings ' +
          '(chrome://extensions → fspec Browser Agent → Details → Allow User Scripts).'
      );
    }
    const tabId = await resolveTabId(args.tabId as number | undefined);
    const wrappedCode = wrapCodeWithErrorHandling(code);
    try {
      const results = await userScripts.execute({
        target: { tabId },
        world: 'USER_SCRIPT',
        js: [{ code: wrappedCode }],
      });
      const firstResult = results[0];
      // Check InjectionResult.error (per user_scripts.idl — currently never
      // populated by Chrome, but defensive for future Chromium changes)
      if (firstResult?.error) {
        return errorResult(`Script execution failed: ${firstResult.error}`);
      }
      const value = firstResult?.result;
      // Check for our error sentinel from the try-catch wrapper
      if (isScriptErrorSentinel(value)) {
        return errorResult(
          `Script execution failed: ${(value as ScriptErrorSentinel).message}`
        );
      }
      return textResult(
        typeof value === 'string' ? value : JSON.stringify(value)
      );
    } catch (error: unknown) {
      // Infrastructure errors (tab not found, permissions) reject the promise
      const message = error instanceof Error ? error.message : String(error);
      return errorResult(`Script execution failed: ${message}`);
    }
  });

  // browser_switch_tab
  handlers.set('browser_switch_tab', async args => {
    const tabId = args.tabId as number;
    if (tabId === undefined) {
      return errorResult('Missing required parameter: tabId');
    }
    const tab = await tabs.get(tabId);
    await tabs.update(tabId, { active: true });
    if (tab.windowId !== undefined) {
      await windows.update(tab.windowId, { focused: true });
    }
    return textResult({
      switched: true,
      tabId,
      url: tab.url,
      title: tab.title,
    });
  });

  // browser_close_tab
  handlers.set('browser_close_tab', async args => {
    const tabId = args.tabId as number;
    if (tabId === undefined) {
      return errorResult('Missing required parameter: tabId');
    }
    const tab = await tabs.get(tabId);
    await tabs.remove(tabId);
    return textResult({
      closed: true,
      tabId,
      url: tab.url,
      title: tab.title,
    });
  });

  // browser_get_page_content
  handlers.set('browser_get_page_content', async args => {
    const format = (args.format as string) ?? 'text';
    const tabId = await resolveTabId(args.tabId as number | undefined);
    const results = await scripting.executeScript({
      target: { tabId },
      args: [format],
      func: (fmt: string) => {
        return {
          title: document.title,
          url: document.URL,
          content:
            fmt === 'html'
              ? document.documentElement.outerHTML
              : document.body.innerText,
        };
      },
    });
    const value = results[0]?.result;
    return textResult(value);
  });

  // browser_click_element
  handlers.set('browser_click_element', async args => {
    const selector = args.selector as string;
    if (!selector) {
      return errorResult('Missing required parameter: selector');
    }
    const tabId = await resolveTabId(args.tabId as number | undefined);
    const results = await scripting.executeScript({
      target: { tabId },
      args: [selector],
      func: (sel: string) => {
        const el = document.querySelector(sel);
        if (!el) {
          return { error: 'Element not found: ' + sel };
        }
        (el as HTMLElement).click();
        return { clicked: true, selector: sel };
      },
    });
    const value = results[0]?.result as Record<string, unknown> | undefined;
    if (value?.error) {
      return errorResult(value.error as string);
    }
    return textResult(value);
  });

  // browser_fill_form
  handlers.set('browser_fill_form', async args => {
    const selector = args.selector as string;
    const value = args.value as string;
    if (!selector) {
      return errorResult('Missing required parameter: selector');
    }
    if (value === undefined) {
      return errorResult('Missing required parameter: value');
    }
    const tabId = await resolveTabId(args.tabId as number | undefined);
    const results = await scripting.executeScript({
      target: { tabId },
      args: [selector, value],
      func: (sel: string, val: string) => {
        const el = document.querySelector(sel) as HTMLInputElement | null;
        if (!el) {
          return { error: 'Element not found: ' + sel };
        }
        el.value = val;
        el.dispatchEvent(new Event('input', { bubbles: true }));
        el.dispatchEvent(new Event('change', { bubbles: true }));
        return { filled: true, selector: sel, value: val };
      },
    });
    const result = results[0]?.result as Record<string, unknown> | undefined;
    if (result?.error) {
      return errorResult(result.error as string);
    }
    return textResult(result);
  });

  // browser_go_back
  handlers.set('browser_go_back', async args => {
    const tabId = await resolveTabId(args.tabId as number | undefined);
    await tabs.goBack(tabId);
    return textResult({ navigated: true, direction: 'back' });
  });

  // browser_go_forward
  handlers.set('browser_go_forward', async args => {
    const tabId = await resolveTabId(args.tabId as number | undefined);
    await tabs.goForward(tabId);
    return textResult({ navigated: true, direction: 'forward' });
  });

  // browser_create_tab
  handlers.set('browser_create_tab', async args => {
    const createProperties: Record<string, unknown> = {};
    if (args.url !== undefined) {
      createProperties.url = args.url as string;
    }
    if (args.active !== undefined) {
      createProperties.active = args.active as boolean;
    }
    if (args.windowId !== undefined) {
      createProperties.windowId = args.windowId as number;
    }
    if (args.pinned !== undefined) {
      createProperties.pinned = args.pinned as boolean;
    }

    const tab = await tabs.create(createProperties);

    // Wait for load if a URL was provided and the tab has an ID
    if (args.url && tab.id !== undefined) {
      const loadedTab = await waitForTabLoad(tab.id);
      return textResult({
        tabId: loadedTab.id,
        url: loadedTab.url ?? args.url,
        title: loadedTab.title ?? '',
        active: loadedTab.active,
        windowId: loadedTab.windowId,
      });
    }

    return textResult({
      tabId: tab.id,
      url: tab.url ?? '',
      title: tab.title ?? '',
      active: tab.active,
      windowId: tab.windowId,
    });
  });

  return {
    getHandler(toolName: string): ToolHandler | undefined {
      return handlers.get(toolName);
    },

    getToolNames(): string[] {
      return Array.from(handlers.keys());
    },
  };
}
