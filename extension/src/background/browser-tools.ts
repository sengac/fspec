/**
 * fspec WebMCP Extension - Native Browser Control Tools
 *
 * Implements the 11 native browser control tool handlers:
 * - browser_navigate, browser_screenshot, browser_list_tabs,
 *   browser_execute_script, browser_switch_tab, browser_close_tab,
 *   browser_get_page_content, browser_click_element, browser_fill_form,
 *   browser_go_back, browser_go_forward
 *
 * Each handler is an async function that accepts tool arguments and returns
 * an MCP-formatted result (content array with text or image items).
 *
 * Implemented by: EXT-005
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

export interface BrowserToolsDeps {
  tabs: ChromeTabsForTools;
  scripting: ChromeScriptingForTools;
  windows: ChromeWindowsForTools;
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

export function createBrowserTools(deps: BrowserToolsDeps): BrowserToolsAPI {
  const { tabs, scripting, windows } = deps;

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
  handlers.set('browser_execute_script', async args => {
    const code = args.code as string;
    if (!code) {
      return errorResult('Missing required parameter: code');
    }
    const tabId = await resolveTabId(args.tabId as number | undefined);
    const results = await scripting.executeScript({
      target: { tabId },
      args: [code],
      func: (codeStr: string) => {
        return eval(codeStr);
      },
    });
    const value = results[0]?.result;
    return textResult(
      typeof value === 'string' ? value : JSON.stringify(value)
    );
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

  return {
    getHandler(toolName: string): ToolHandler | undefined {
      return handlers.get(toolName);
    },

    getToolNames(): string[] {
      return Array.from(handlers.keys());
    },
  };
}
