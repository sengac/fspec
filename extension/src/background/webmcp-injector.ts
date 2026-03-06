/**
 * fspec WebMCP Extension - WebMCP Script Injector
 *
 * Handles injection of the main-world discovery script into web pages
 * using chrome.scripting.executeScript with world: 'MAIN'.
 *
 * Injection is triggered by chrome.tabs.onUpdated when a page load
 * completes (status: 'complete').
 *
 * Implemented by: EXT-006
 */

import { webmcpDiscoveryFunction } from '../content/webmcp-discovery';

/** Minimal chrome.scripting interface for dependency injection */
export interface ChromeScriptingForInjector {
  executeScript: (details: {
    target: { tabId: number };
    world: string;
    func: () => void;
    injectImmediately?: boolean;
  }) => Promise<unknown>;
}

/** Minimal chrome.tabs interface for dependency injection */
export interface ChromeTabsForInjector {
  onUpdated: {
    addListener: (
      callback: (
        tabId: number,
        changeInfo: { status?: string; url?: string },
        tab: { id?: number; url?: string }
      ) => void
    ) => void;
  };
}

export interface WebMCPInjectorOptions {
  scripting: ChromeScriptingForInjector;
  tabs: ChromeTabsForInjector;
}

export interface WebMCPInjectorAPI {
  /**
   * Inject the discovery script into a specific tab.
   * Returns true if injection was attempted.
   */
  injectIntoTab: (tabId: number) => Promise<boolean>;
}

/**
 * Create the WebMCP injector that listens for tab load events
 * and injects the discovery script into pages.
 */
export function createWebMCPInjector(
  options: WebMCPInjectorOptions
): WebMCPInjectorAPI {
  const { scripting, tabs } = options;

  /** Set of tab IDs that already have the discovery script injected */
  const injectedTabs = new Set<number>();

  async function injectIntoTab(tabId: number): Promise<boolean> {
    // Avoid double-injection for the same tab
    if (injectedTabs.has(tabId)) {
      return false;
    }

    try {
      await scripting.executeScript({
        target: { tabId },
        world: 'MAIN',
        func: webmcpDiscoveryFunction,
        injectImmediately: true,
      });
      injectedTabs.add(tabId);
      return true;
    } catch {
      // Injection can fail for chrome:// URLs, extension pages, etc.
      return false;
    }
  }

  // Listen for tab load completions and inject automatically
  tabs.onUpdated.addListener(
    (
      tabId: number,
      changeInfo: { status?: string },
      _tab: { id?: number; url?: string }
    ) => {
      if (changeInfo.status === 'complete') {
        // Clear the injected state for this tab (page navigated, script lost)
        injectedTabs.delete(tabId);
        // Re-inject
        void injectIntoTab(tabId);
      }
    }
  );

  return {
    injectIntoTab,
  };
}
