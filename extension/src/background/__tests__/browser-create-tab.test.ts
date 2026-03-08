/**
 * Feature: spec/features/browser-tab-creation.feature
 *
 * This test file validates the acceptance criteria for EXT-011:
 * Add browser_create_tab tool to fspec Browser Agent Chrome Extension.
 *
 * Tests the browser_create_tab handler in browser-tools.ts,
 * the ChromeTabsForTools interface, and the MCP server NATIVE_TOOLS array.
 */

import { describe, it, expect, vi } from 'vitest';
import { createBrowserTools } from '../browser-tools';
import type {
  BrowserToolsDeps,
  ChromeTabsForTools,
  ChromeScriptingForTools,
  ChromeWindowsForTools,
  ChromeUserScriptsForTools,
} from '../browser-tools';

/** Helper: create a minimal mock tabs object with create support */
function createMockTabs(
  overrides?: Partial<ChromeTabsForTools>
): ChromeTabsForTools {
  return {
    query: vi.fn().mockResolvedValue([{ id: 1, url: 'https://example.com' }]),
    update: vi.fn().mockResolvedValue({ id: 1 }),
    remove: vi.fn().mockResolvedValue(undefined),
    captureVisibleTab: vi.fn().mockResolvedValue('data:image/png;base64,abc'),
    goBack: vi.fn().mockResolvedValue(undefined),
    goForward: vi.fn().mockResolvedValue(undefined),
    get: vi.fn().mockResolvedValue({
      id: 1,
      windowId: 1,
      url: 'https://example.com',
      title: 'Example',
    }),
    onUpdated: {
      addListener: vi.fn(),
      removeListener: vi.fn(),
    },
    create: vi.fn().mockResolvedValue({
      id: 42,
      url: '',
      title: '',
      active: true,
      windowId: 1,
      index: 0,
    }),
    ...overrides,
  };
}

/** Helper: create a minimal mock scripting object */
function createMockScripting(): ChromeScriptingForTools {
  return {
    executeScript: vi.fn().mockResolvedValue([{ result: null }]),
  };
}

/** Helper: create a minimal mock windows object */
function createMockWindows(): ChromeWindowsForTools {
  return {
    update: vi.fn().mockResolvedValue(undefined),
  };
}

/** Helper: create a mock userScripts API */
function createMockUserScripts(): ChromeUserScriptsForTools {
  return {
    configureWorld: vi.fn().mockResolvedValue(undefined),
    execute: vi.fn().mockResolvedValue([{ result: null }]),
  };
}

/** Helper: create full deps */
function createDeps(options?: {
  tabs?: Partial<ChromeTabsForTools>;
}): BrowserToolsDeps {
  return {
    tabs: createMockTabs(options?.tabs),
    scripting: createMockScripting(),
    windows: createMockWindows(),
    userScripts: createMockUserScripts(),
  };
}

describe('Feature: Add browser_create_tab tool', () => {
  describe('Scenario: Create a new tab with a URL', () => {
    it('should create a tab with the given URL and wait for load', async () => {
      // @step Given the browser tools are initialized with a mock tabs API
      const mockCreate = vi.fn().mockResolvedValue({
        id: 42,
        url: '',
        title: '',
        active: true,
        windowId: 1,
        index: 0,
      });
      const deps = createDeps({
        tabs: {
          create: mockCreate,
          get: vi.fn().mockResolvedValue({
            id: 42,
            windowId: 1,
            url: 'https://example.com',
            title: 'Example',
            active: true,
          }),
          onUpdated: {
            addListener: vi.fn(cb => {
              // Simulate tab load completing immediately
              cb(42, { status: 'complete' }, {
                id: 42,
                windowId: 1,
                url: 'https://example.com',
                title: 'Example',
                active: true,
              } as chrome.tabs.Tab);
            }),
            removeListener: vi.fn(),
          },
        },
      });
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_create_tab');
      expect(handler).toBeDefined();

      // @step When I call browser_create_tab with url "https://example.com"
      const result = await handler!({ url: 'https://example.com' });

      // @step Then the handler should call tabs.create with url "https://example.com"
      expect(mockCreate).toHaveBeenCalledWith(
        expect.objectContaining({ url: 'https://example.com' })
      );

      // @step And the handler should wait for the tab to finish loading
      expect(deps.tabs.onUpdated.addListener).toHaveBeenCalled();

      // @step And the result should contain tabId, url, title, active, and windowId
      const content = result.content[0] as { type: 'text'; text: string };
      expect(content.type).toBe('text');
      const parsed = JSON.parse(content.text) as Record<string, unknown>;
      expect(parsed).toHaveProperty('tabId');
      expect(parsed).toHaveProperty('url');
      expect(parsed).toHaveProperty('title');
      expect(parsed).toHaveProperty('active');
      expect(parsed).toHaveProperty('windowId');
    });
  });

  describe('Scenario: Create a new tab without a URL', () => {
    it('should create a blank tab without waiting for load', async () => {
      // @step Given the browser tools are initialized with a mock tabs API
      const mockCreate = vi.fn().mockResolvedValue({
        id: 43,
        url: '',
        title: '',
        active: true,
        windowId: 1,
        index: 0,
      });
      const deps = createDeps({ tabs: { create: mockCreate } });
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_create_tab');
      expect(handler).toBeDefined();

      // @step When I call browser_create_tab with no arguments
      const result = await handler!({});

      // @step Then the handler should call tabs.create with an empty properties object
      expect(mockCreate).toHaveBeenCalledWith({});

      // @step And the result should return immediately without waiting for load
      // onUpdated.addListener should NOT have been called — no URL means no waitForTabLoad
      expect(deps.tabs.onUpdated.addListener).not.toHaveBeenCalled();

      // @step And the result should contain tabId, url, title, active, and windowId
      const content = result.content[0] as { type: 'text'; text: string };
      const parsed = JSON.parse(content.text) as Record<string, unknown>;
      expect(parsed).toHaveProperty('tabId', 43);
      expect(parsed).toHaveProperty('url', '');
      expect(parsed).toHaveProperty('title', '');
      expect(parsed).toHaveProperty('active', true);
      expect(parsed).toHaveProperty('windowId', 1);
    });
  });

  describe('Scenario: Create a background tab', () => {
    it('should create a tab with active set to false', async () => {
      // @step Given the browser tools are initialized with a mock tabs API
      const mockCreate = vi.fn().mockResolvedValue({
        id: 44,
        url: '',
        title: '',
        active: false,
        windowId: 1,
        index: 1,
      });
      const deps = createDeps({
        tabs: {
          create: mockCreate,
          get: vi.fn().mockResolvedValue({
            id: 44,
            windowId: 1,
            url: 'https://example.com',
            title: 'Example',
            active: false,
          }),
          onUpdated: {
            addListener: vi.fn(cb => {
              cb(44, { status: 'complete' }, {
                id: 44,
                windowId: 1,
                url: 'https://example.com',
                title: 'Example',
                active: false,
              } as chrome.tabs.Tab);
            }),
            removeListener: vi.fn(),
          },
        },
      });
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_create_tab');
      expect(handler).toBeDefined();

      // @step When I call browser_create_tab with url "https://example.com" and active false
      const result = await handler!({
        url: 'https://example.com',
        active: false,
      });

      // @step Then the handler should call tabs.create with active set to false
      expect(mockCreate).toHaveBeenCalledWith(
        expect.objectContaining({ active: false })
      );

      // @step And the result should show active as false
      const content = result.content[0] as { type: 'text'; text: string };
      const parsed = JSON.parse(content.text) as Record<string, unknown>;
      expect(parsed).toHaveProperty('active', false);
    });
  });

  describe('Scenario: Create a pinned tab', () => {
    it('should create a tab with pinned set to true', async () => {
      // @step Given the browser tools are initialized with a mock tabs API
      const mockCreate = vi.fn().mockResolvedValue({
        id: 45,
        url: '',
        title: '',
        active: true,
        windowId: 1,
        index: 0,
        pinned: true,
      });
      const deps = createDeps({
        tabs: {
          create: mockCreate,
          get: vi.fn().mockResolvedValue({
            id: 45,
            windowId: 1,
            url: 'https://example.com',
            title: 'Example',
            active: true,
          }),
          onUpdated: {
            addListener: vi.fn(cb => {
              cb(45, { status: 'complete' }, {
                id: 45,
                windowId: 1,
                url: 'https://example.com',
                title: 'Example',
                active: true,
              } as chrome.tabs.Tab);
            }),
            removeListener: vi.fn(),
          },
        },
      });
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_create_tab');
      expect(handler).toBeDefined();

      // @step When I call browser_create_tab with url "https://example.com" and pinned true
      await handler!({
        url: 'https://example.com',
        pinned: true,
      });

      // @step Then the handler should call tabs.create with pinned set to true
      expect(mockCreate).toHaveBeenCalledWith(
        expect.objectContaining({ pinned: true })
      );
    });
  });

  describe('Scenario: Tool is registered in the handler map', () => {
    it('should include browser_create_tab in the tool names', () => {
      // @step Given the browser tools are initialized with a mock tabs API
      const deps = createDeps();
      const tools = createBrowserTools(deps);

      // @step Then the tool names should include "browser_create_tab"
      expect(tools.getToolNames()).toContain('browser_create_tab');
    });
  });

  describe('Scenario: ChromeTabsForTools interface includes create method', () => {
    it('should have a create method on the interface', async () => {
      // @step Given the browser tools source code
      const { readFile } = await import('fs/promises');
      const { join } = await import('path');
      const sourcePath = join(
        import.meta.dirname ?? '.',
        '..',
        'browser-tools-types.ts'
      );
      const source = await readFile(sourcePath, 'utf-8');

      // @step Then the ChromeTabsForTools interface should declare a create method
      expect(source).toContain('export interface ChromeTabsForTools');
      expect(source).toMatch(/create\s*:/);
    });
  });

  describe('Scenario: MCP server NATIVE_TOOLS includes browser_create_tab', () => {
    it('should have browser_create_tab in the NATIVE_TOOLS array', async () => {
      // @step Given the MCP server source code
      const { readFile } = await import('fs/promises');
      const { join } = await import('path');
      const mcpPath = join(
        import.meta.dirname ?? '.',
        '..',
        '..',
        '..',
        'host',
        'lib',
        'mcp-server.mjs'
      );
      const source = await readFile(mcpPath, 'utf-8');

      // @step Then the NATIVE_TOOLS array should contain a tool named "browser_create_tab"
      expect(source).toContain("'browser_create_tab'");

      // @step And the tool schema should have optional properties url, active, windowId, and pinned
      // Extract a generous section around the browser_create_tab entry
      const startIdx = source.indexOf("'browser_create_tab'");
      const createTabSection = source.substring(startIdx, startIdx + 500);
      expect(createTabSection).toContain('url');
      expect(createTabSection).toContain('active');
      expect(createTabSection).toContain('windowId');
      expect(createTabSection).toContain('pinned');
    });
  });

  describe('Scenario: Skill documentation includes browser_create_tab', () => {
    it('should document browser_create_tab in extension-skill.md', async () => {
      // @step Given the extension-skill.md documentation file
      const { readFile } = await import('fs/promises');
      const { join } = await import('path');
      const skillPath = join(
        import.meta.dirname ?? '.',
        '..',
        '..',
        '..',
        'extension-skill.md'
      );
      const source = await readFile(skillPath, 'utf-8');

      // @step Then the documentation should reference browser_create_tab with its parameters and return value
      expect(source).toContain('browser_create_tab');
      expect(source).toContain('url');
      expect(source).toContain('active');
      expect(source).toContain('pinned');
      expect(source).toContain('windowId');
      expect(source).toContain('tabId');
    });
  });
});
