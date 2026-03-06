/**
 * Feature: spec/features/webmcp-chrome-extension.feature
 *
 * This test file validates the WebMCP script injector (EXT-006).
 * The injector uses chrome.scripting.executeScript with world: 'MAIN'
 * to inject the discovery script into every page on tab load complete.
 *
 * Rules covered:
 *   [2] Discovery script injection MUST use chrome.scripting.executeScript with world: 'MAIN'
 *   [6] Discovery script MUST be injected into every page on document_idle or tab complete
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import type {
  ChromeScriptingForInjector,
  ChromeTabsForInjector,
  WebMCPInjectorAPI,
} from '../../../extension/src/background/webmcp-injector';

// --- Test Helpers ---

type TabUpdateCallback = (
  tabId: number,
  changeInfo: { status?: string; url?: string },
  tab: { id?: number; url?: string }
) => void;

function createMockScripting(): ChromeScriptingForInjector {
  return {
    executeScript: vi.fn(() => Promise.resolve([{ result: undefined }])),
  };
}

function createMockTabs(): ChromeTabsForInjector & {
  triggerUpdate: TabUpdateCallback;
} {
  let callback: TabUpdateCallback | null = null;
  return {
    onUpdated: {
      addListener: vi.fn((cb: TabUpdateCallback) => {
        callback = cb;
      }),
    },
    triggerUpdate(tabId, changeInfo, tab) {
      if (callback) {
        callback(tabId, changeInfo, tab);
      }
    },
  };
}

describe('Feature: fspec Browser Agent Chrome Extension — WebMCP Script Injector', () => {
  let scripting: ReturnType<typeof createMockScripting>;
  let tabs: ReturnType<typeof createMockTabs>;
  let injector: WebMCPInjectorAPI;

  beforeEach(async () => {
    scripting = createMockScripting();
    tabs = createMockTabs();

    const { createWebMCPInjector } = await import(
      /* @vite-ignore */ '../../../extension/src/background/webmcp-injector'
    );
    injector = createWebMCPInjector({ scripting, tabs });
  });

  describe('Rule [2]: Injection uses chrome.scripting.executeScript with world MAIN', () => {
    it('should inject via executeScript with world MAIN and correct tabId', async () => {
      // @step Given a tab with id 42 has loaded
      const tabId = 42;

      // @step When the injector injects into the tab
      const result = await injector.injectIntoTab(tabId);

      // @step Then chrome.scripting.executeScript is called with world MAIN
      expect(result).toBe(true);
      expect(scripting.executeScript).toHaveBeenCalledWith(
        expect.objectContaining({
          target: { tabId: 42 },
          world: 'MAIN',
          func: expect.any(Function),
        })
      );
    });
  });

  describe('Rule [6]: Injection on tab complete for every page', () => {
    it('should listen for chrome.tabs.onUpdated events', () => {
      // @step Given the injector is created
      // (done in beforeEach)

      // @step Then it registers a listener on chrome.tabs.onUpdated
      expect(tabs.onUpdated.addListener).toHaveBeenCalledWith(
        expect.any(Function)
      );
    });

    it('should inject when tab status changes to complete', async () => {
      // @step Given a tab navigates and reaches status complete
      tabs.triggerUpdate(
        55,
        { status: 'complete' },
        { id: 55, url: 'https://example.com' }
      );

      // Wait for the async injection
      await new Promise(r => setTimeout(r, 10));

      // @step Then the discovery script is injected into that tab
      expect(scripting.executeScript).toHaveBeenCalledWith(
        expect.objectContaining({
          target: { tabId: 55 },
          world: 'MAIN',
        })
      );
    });

    it('should NOT inject when tab status is loading', async () => {
      // @step Given a tab status changes to loading (not complete)
      tabs.triggerUpdate(55, { status: 'loading' }, { id: 55 });

      await new Promise(r => setTimeout(r, 10));

      // @step Then the discovery script is NOT injected
      expect(scripting.executeScript).not.toHaveBeenCalled();
    });
  });

  describe('Double-injection prevention', () => {
    it('should not inject the same tab twice without navigation', async () => {
      // @step Given the script was already injected into tab 42
      await injector.injectIntoTab(42);
      expect(scripting.executeScript).toHaveBeenCalledTimes(1);

      // @step When we try to inject again
      const result = await injector.injectIntoTab(42);

      // @step Then the second injection is skipped
      expect(result).toBe(false);
      expect(scripting.executeScript).toHaveBeenCalledTimes(1);
    });

    it('should re-inject after tab navigates (status complete clears injected state)', async () => {
      // @step Given the script was injected into tab 42
      await injector.injectIntoTab(42);
      expect(scripting.executeScript).toHaveBeenCalledTimes(1);

      // @step When the tab navigates (status: complete fires again)
      tabs.triggerUpdate(
        42,
        { status: 'complete' },
        { id: 42, url: 'https://new-page.com' }
      );

      await new Promise(r => setTimeout(r, 10));

      // @step Then the script is re-injected (navigation cleared old state)
      expect(scripting.executeScript).toHaveBeenCalledTimes(2);
    });
  });

  describe('Graceful failure for restricted pages', () => {
    it('should return false when injection fails (e.g., chrome:// URLs)', async () => {
      // @step Given a tab showing a chrome:// URL that rejects script injection
      (
        scripting.executeScript as ReturnType<typeof vi.fn>
      ).mockRejectedValueOnce(new Error('Cannot access a chrome:// URL'));

      // @step When we attempt to inject
      const result = await injector.injectIntoTab(99);

      // @step Then the injector returns false without throwing
      expect(result).toBe(false);
    });

    it('should allow retrying injection after a failure', async () => {
      // @step Given injection failed on the first attempt
      (
        scripting.executeScript as ReturnType<typeof vi.fn>
      ).mockRejectedValueOnce(new Error('Cannot access a chrome:// URL'));
      await injector.injectIntoTab(99);

      // @step When the tab navigates to a normal page and triggers complete
      (
        scripting.executeScript as ReturnType<typeof vi.fn>
      ).mockResolvedValueOnce([{ result: undefined }]);
      tabs.triggerUpdate(
        99,
        { status: 'complete' },
        { id: 99, url: 'https://example.com' }
      );

      await new Promise(r => setTimeout(r, 10));

      // @step Then injection is attempted again
      expect(scripting.executeScript).toHaveBeenCalledTimes(2);
    });
  });
});
