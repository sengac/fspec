/**
 * fspec Browser Agent - Native Browser Control Tools
 *
 * Implements the native browser control tool handlers:
 * - browser_navigate, browser_screenshot, browser_list_tabs,
 *   browser_execute_script, browser_switch_tab, browser_close_tab,
 *   browser_get_page_content, browser_click_element, browser_fill_form,
 *   browser_go_back, browser_go_forward, browser_create_tab,
 *   browser_scan_page, browser_diff_page
 *
 * Each handler is an async function that accepts tool arguments and returns
 * an MCP-formatted result (content array with text or image items).
 *
 * Implemented by: EXT-005, EXT-011, LOCATE-004, LOCATE-006
 */

import { formatAccessibilityTree } from './dom-scanner';
import { setTabScanState, getTabScanState, resolveRef } from './ref-state';
import { myersDiff, formatDiffOutput } from './myers-diff';
import type { RefEntry } from './ref-state';
import { scanPageDOM } from './scan-page-dom';
import { textResult, errorResult } from './browser-tools-types';
import {
  DEFAULT_MAX_FRAMES,
  isScannableFrame,
  prioritizeFrames,
  injectFrameMarkers,
  scanFrames,
  mergeFrameResults,
} from './iframe-scanner';
import type { FrameScanResult } from './iframe-scanner';
import type {
  BrowserToolsDeps,
  McpToolResult,
  ToolHandler,
  BrowserToolsAPI,
  FrameInfo,
} from './browser-tools-types';

export type {
  ChromeTabsForTools,
  ChromeScriptingForTools,
  ChromeWindowsForTools,
  ChromeUserScriptsForTools,
  ChromeWebNavigationForTools,
  FrameInfo,
  BrowserToolsDeps,
  BrowserToolsAPI,
} from './browser-tools-types';

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
  const { tabs, scripting, windows, userScripts, webNavigation } = deps;

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

  /** Result from executing a page scan, assigning refs, and storing state */
  interface ScanAndStoreResult {
    refs: Map<string, RefEntry>;
    treeText: string;
    metadata: FrameScanResult['metadata'];
  }

  /**
   * Execute a page scan, assign refs, format tree, and store state.
   * Shared by browser_scan_page and browser_diff_page (Rule [1]: reuse).
   *
   * When webNavigation is available, performs multi-frame scanning:
   * 1. getAllFrames to discover all frames
   * 2. Two-pass injection for frame-to-DOM correlation
   * 3. Per-frame scanPageDOM via executeScript
   * 4. Merge results into unified tree with frame-prefixed refs
   *
   * @returns Scan result with refs, tree text, and metadata, or null if scan returned no results.
   */
  async function executeScanAndStore(
    tabId: number,
    interactive: boolean,
    scopeSelector?: string,
    maxFrames?: number
  ): Promise<ScanAndStoreResult | null> {
    // Phase 1: Discover frames
    let allFrames: FrameInfo[] | null = null;
    if (webNavigation) {
      try {
        allFrames = await webNavigation.getAllFrames({ tabId });
      } catch {
        // Fall back to single-frame scan
        allFrames = null;
      }
    }

    // Single-frame path (no webNavigation or no frames)
    if (!allFrames || allFrames.length <= 1) {
      return executeSingleFrameScan(tabId, interactive, scopeSelector);
    }

    // Multi-frame path
    const mainFrame = allFrames.find(
      f => f.frameId === 0 || f.frameType === 'outermost_frame'
    );
    if (!mainFrame) {
      return executeSingleFrameScan(tabId, interactive, scopeSelector);
    }

    const subframes = allFrames.filter(
      f => f.frameId !== 0 && f.frameType !== 'outermost_frame'
    );
    const scannableSubframes = subframes.filter(
      f => f.documentLifecycle === 'active' && isScannableFrame(f.url)
    );

    const limit = maxFrames ?? DEFAULT_MAX_FRAMES;
    const { scanned: framesToScan, skipped: skippedFrames } = prioritizeFrames(
      scannableSubframes,
      mainFrame.url,
      limit
    );

    // Non-scannable frames (chrome://, chrome-extension://)
    const nonScannableFrames = subframes.filter(
      f => !isScannableFrame(f.url) || f.documentLifecycle !== 'active'
    );

    // Phase 2: Two-pass injection — inject frameId markers into each subframe
    await injectFrameMarkers(scripting, tabId, framesToScan);

    // Phase 3: Scan main frame
    const mainResults = await scripting.executeScript({
      target: { tabId },
      args: [interactive, scopeSelector ?? null] as [boolean, string | null],
      func: scanPageDOM,
    });
    const mainScanResult = mainResults[0]?.result as FrameScanResult | null;
    if (!mainScanResult) {
      return null;
    }

    // Phase 4: Scan each iframe
    const frameScanResults = await scanFrames(
      scripting,
      tabId,
      framesToScan,
      scanPageDOM,
      interactive
    );

    // Phase 5: Merge results, assign refs, and build tree
    const { mergedElements, refs } = mergeFrameResults(
      mainScanResult.elements,
      framesToScan,
      frameScanResults,
      skippedFrames,
      nonScannableFrames,
      allFrames
    );

    const treeText = formatAccessibilityTree(mergedElements);
    setTabScanState(tabId, { refs, treeText, timestamp: Date.now() });

    return { refs, treeText, metadata: mainScanResult.metadata };
  }

  /** Single-frame scan — legacy path for pages without iframes or without webNavigation */
  async function executeSingleFrameScan(
    tabId: number,
    interactive: boolean,
    scopeSelector?: string
  ): Promise<ScanAndStoreResult | null> {
    const results = await scripting.executeScript({
      target: { tabId },
      args: [interactive, scopeSelector ?? null] as [boolean, string | null],
      func: scanPageDOM,
    });

    const scanResult = results[0]?.result as FrameScanResult | null;
    if (!scanResult) {
      return null;
    }

    // Assign refs to interactive elements (service worker side)
    let refCounter = 1;
    const refs = new Map<string, RefEntry>();
    for (const element of scanResult.elements) {
      if (element.interactive) {
        const refKey = `e${refCounter++}`;
        refs.set(refKey, {
          selector: element.selector,
          role: element.role,
          name: element.name,
          frameId: 0,
        });
        element.ref = refKey;
      }
    }

    const treeText = formatAccessibilityTree(scanResult.elements);
    setTabScanState(tabId, { refs, treeText, timestamp: Date.now() });

    return { refs, treeText, metadata: scanResult.metadata };
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

  /**
   * Resolve a @ref selector to its CSS selector and frame context.
   * Raw CSS selectors (not starting with '@') pass through unchanged.
   * Returns { selector, frameId } on success, or an error result if
   * the ref is not found.
   */
  function resolveRefSelector(
    selector: string,
    tabId: number
  ): { selector: string; frameId: number } | McpToolResult {
    if (!selector.startsWith('@')) {
      return { selector, frameId: 0 };
    }
    const refKey = selector.slice(1);
    const entry = resolveRef(tabId, refKey);
    if (!entry) {
      return errorResult(
        `Ref ${selector} not found. Run browser_scan_page first to scan the page.`
      );
    }
    return { selector: entry.selector, frameId: entry.frameId };
  }

  /** Type guard: check if resolve result is an error */
  function isResolveError(
    result: { selector: string; frameId: number } | McpToolResult
  ): result is McpToolResult {
    return 'content' in result;
  }

  // browser_click_element
  handlers.set('browser_click_element', async args => {
    let selector = args.selector as string;
    if (!selector) {
      return errorResult('Missing required parameter: selector');
    }
    const tabId = await resolveTabId(args.tabId as number | undefined);

    // Ref resolution: '@e3' or '@f5e4' → look up CSS selector + frameId from scan state
    const resolved = resolveRefSelector(selector, tabId);
    if (isResolveError(resolved)) {
      return resolved;
    }
    selector = resolved.selector;
    const frameId = resolved.frameId;

    const target: chrome.scripting.InjectionTarget =
      frameId > 0 ? { tabId, frameIds: [frameId] } : { tabId };

    const results = await scripting.executeScript({
      target,
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
    let selector = args.selector as string;
    const value = args.value as string;
    if (!selector) {
      return errorResult('Missing required parameter: selector');
    }
    if (value === undefined) {
      return errorResult('Missing required parameter: value');
    }
    const tabId = await resolveTabId(args.tabId as number | undefined);

    // Ref resolution: '@e3' or '@f5e1' → look up CSS selector + frameId from scan state
    const resolved = resolveRefSelector(selector, tabId);
    if (isResolveError(resolved)) {
      return resolved;
    }
    selector = resolved.selector;
    const frameId = resolved.frameId;

    const target: chrome.scripting.InjectionTarget =
      frameId > 0 ? { tabId, frameIds: [frameId] } : { tabId };

    const results = await scripting.executeScript({
      target,
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

  // browser_scan_page
  handlers.set('browser_scan_page', async args => {
    const interactive = args.interactive !== false; // default true
    const scopeSelector = args.selector as string | undefined;
    const maxFrames = args.maxFrames as number | undefined;
    let tabId: number;
    try {
      tabId = await resolveTabId(args.tabId as number | undefined);
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      return errorResult(`Tab not found: ${message}`);
    }

    try {
      const scan = await executeScanAndStore(
        tabId,
        interactive,
        scopeSelector,
        maxFrames
      );
      if (!scan) {
        return errorResult(
          'Scan returned no results — page may be restricted (chrome://, edge://)'
        );
      }

      const meta = scan.metadata;
      const header = `Page: ${meta.url} — "${meta.title}"\nViewport: ${meta.viewportWidth}x${meta.viewportHeight} | Elements: ${meta.totalElements} | Interactive: ${scan.refs.size}`;
      return textResult(
        `${header}\n\n${scan.treeText}\n\n${scan.refs.size} interactive elements`
      );
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      return errorResult(`Scan failed: ${message}`);
    }
  });

  // browser_diff_page
  handlers.set('browser_diff_page', async args => {
    let tabId: number;
    try {
      tabId = await resolveTabId(args.tabId as number | undefined);
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      return errorResult(`Tab not found: ${message}`);
    }

    try {
      // 1. Get previous scan state
      const previousState = getTabScanState(tabId);
      const previousTreeText = previousState?.treeText ?? '';

      // 2. Run fresh scan (reuses executeScanAndStore — Rule [1])
      const scan = await executeScanAndStore(tabId, true);
      if (!scan) {
        return errorResult(
          'Scan returned no results — page may be restricted (chrome://, edge://)'
        );
      }

      // 3. Compute diff
      const oldLines = previousTreeText
        ? previousTreeText.split('\n').filter(l => l.length > 0)
        : [];
      const newLines = scan.treeText
        ? scan.treeText.split('\n').filter(l => l.length > 0)
        : [];
      const diff = myersDiff(oldLines, newLines);

      // 4. Format output
      let output: string;
      if (!previousState) {
        // First scan — show all as additions with note
        const allAdditions = newLines.map(l => `+ ${l}`).join('\n');
        const summary = `${newLines.length} addition${newLines.length !== 1 ? 's' : ''}, 0 removals, 0 unchanged`;
        output = `No previous scan to compare against. Showing current state.\n\n${allAdditions}\n\nChanges: ${summary}`;
      } else {
        output = formatDiffOutput(diff);
      }

      return textResult(output);
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      return errorResult(`Diff failed: ${message}`);
    }
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
