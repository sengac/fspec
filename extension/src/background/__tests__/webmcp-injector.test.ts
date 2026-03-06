/**
 * Feature: spec/features/webmcp-dynamic-tool-discovery.feature
 *
 * This test file validates the injection timing scenarios for EXT-009.
 * Tests the WebMCP injector's early injection strategy.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  createWebMCPInjector,
  type ChromeScriptingForInjector,
  type ChromeTabsForInjector,
} from '../webmcp-injector';

describe('Feature: WebMCP Dynamic Tool Discovery - Injector', () => {
  let mockScripting: ChromeScriptingForInjector;
  let mockTabs: ChromeTabsForInjector;
  let tabUpdatedCallbacks: Array<
    (
      tabId: number,
      changeInfo: { status?: string; url?: string },
      tab: { id?: number; url?: string }
    ) => void
  >;

  beforeEach(() => {
    tabUpdatedCallbacks = [];

    mockScripting = {
      executeScript: vi.fn().mockResolvedValue(undefined),
    };

    mockTabs = {
      onUpdated: {
        addListener: (
          callback: (
            tabId: number,
            changeInfo: { status?: string; url?: string },
            tab: { id?: number; url?: string }
          ) => void
        ) => {
          tabUpdatedCallbacks.push(callback);
        },
      },
    };
  });

  describe('Scenario: Injector uses early injection strategy', () => {
    it('should inject the discovery script into MAIN world on tab update', async () => {
      // @step Given the WebMCP injector is initialized with chrome.scripting and chrome.tabs
      const injector = createWebMCPInjector({
        scripting: mockScripting,
        tabs: mockTabs,
      });

      // @step When a tab triggers the injection
      // Simulate tab completing load
      for (const cb of tabUpdatedCallbacks) {
        cb(1, { status: 'complete' }, { id: 1, url: 'https://example.com' });
      }

      // Wait for async injection
      await vi.waitFor(() => {
        expect(mockScripting.executeScript).toHaveBeenCalled();
      });

      // @step Then the discovery script is injected into the MAIN world
      const call = (mockScripting.executeScript as ReturnType<typeof vi.fn>)
        .mock.calls[0][0] as {
        target: { tabId: number };
        world: string;
        func: () => void;
        injectImmediately?: boolean;
      };
      expect(call.target.tabId).toBe(1);
      expect(call.world).toBe('MAIN');

      // @step And the injection uses the earliest available timing
      expect(call.injectImmediately).toBe(true);
    });
  });

  describe('Scenario: Injector re-injects on navigation', () => {
    it('should clear injection state and re-inject when tab navigates', async () => {
      // Given the injector has already injected into tab 1
      const injector = createWebMCPInjector({
        scripting: mockScripting,
        tabs: mockTabs,
      });

      // First injection
      for (const cb of tabUpdatedCallbacks) {
        cb(1, { status: 'complete' }, { id: 1, url: 'https://example.com' });
      }
      await vi.waitFor(() => {
        expect(mockScripting.executeScript).toHaveBeenCalledTimes(1);
      });

      // When the tab navigates to a new page (complete fires again)
      for (const cb of tabUpdatedCallbacks) {
        cb(
          1,
          { status: 'complete' },
          {
            id: 1,
            url: 'https://other.com',
          }
        );
      }

      await vi.waitFor(() => {
        expect(mockScripting.executeScript).toHaveBeenCalledTimes(2);
      });

      // Then injection happens again for the new page
      expect(mockScripting.executeScript).toHaveBeenCalledTimes(2);
    });
  });
});
