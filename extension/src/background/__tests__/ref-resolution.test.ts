/**
 * Feature: spec/features/ref-resolution-click-fill.feature
 *
 * This test file validates the acceptance criteria for LOCATE-005:
 * Ref Resolution in Click and Fill Tools.
 *
 * Tests that browser_click_element and browser_fill_form resolve @ref
 * selectors from the scan state, while passing raw CSS selectors through
 * unchanged.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createBrowserTools } from '../browser-tools';
import type {
  BrowserToolsDeps,
  ChromeTabsForTools,
  ChromeScriptingForTools,
  ChromeWindowsForTools,
  ChromeUserScriptsForTools,
} from '../browser-tools';
import { setTabScanState, _resetForTesting } from '../ref-state';

/** Helper: create a minimal mock tabs object */
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

/** Helper: create a mock scripting object that returns a result */
function createMockScripting(result?: unknown): ChromeScriptingForTools {
  return {
    executeScript: vi
      .fn()
      .mockResolvedValue([
        { result: result ?? { clicked: true, selector: '#submit-btn' } },
      ]),
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
  scriptResult?: unknown;
}): BrowserToolsDeps {
  return {
    tabs: createMockTabs(options?.tabs),
    scripting: createMockScripting(options?.scriptResult),
    windows: createMockWindows(),
    userScripts: createMockUserScripts(),
  };
}

/** Helper: set up a scan state with refs for tab 1 */
function setupScanState(tabId: number, refs: Record<string, string>): void {
  const refMap = new Map<
    string,
    { selector: string; role: string; name: string; frameId: number }
  >();
  for (const [key, selector] of Object.entries(refs)) {
    refMap.set(key, {
      selector,
      role: 'button',
      name: `Element ${key}`,
      frameId: 0,
    });
  }
  setTabScanState(tabId, {
    refs: refMap,
    treeText: 'mock tree text',
    timestamp: Date.now(),
  });
}

describe('Feature: Ref Resolution in Click and Fill Tools', () => {
  beforeEach(() => {
    _resetForTesting();
  });

  describe('Scenario: Click element using ref after page scan', () => {
    it('should resolve @e1 to the stored CSS selector and click it', async () => {
      // @step Given a page has been scanned and ref "e1" maps to CSS selector "#submit-btn"
      const deps = createDeps({
        scriptResult: { clicked: true, selector: '#submit-btn' },
      });
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_click_element');
      expect(handler).toBeDefined();
      setupScanState(1, { e1: '#submit-btn' });

      // @step When I call browser_click_element with selector "@e1"
      const result = await handler!({ selector: '@e1' });

      // @step Then the handler should resolve "@e1" to "#submit-btn"
      const executeScript = deps.scripting.executeScript as ReturnType<
        typeof vi.fn
      >;
      expect(executeScript).toHaveBeenCalled();
      const callArgs = executeScript.mock.calls[0][0] as {
        args: [string];
      };
      expect(callArgs.args[0]).toBe('#submit-btn');

      // @step And the element "#submit-btn" should be clicked
      expect(result.isError).toBeUndefined();
      const content = result.content[0] as { type: 'text'; text: string };
      expect(content.type).toBe('text');
    });
  });

  describe('Scenario: Fill form field using ref after page scan', () => {
    it('should resolve @e3 to the stored CSS selector and fill the value', async () => {
      // @step Given a page has been scanned and ref "e3" maps to CSS selector "input[name=email]"
      const deps = createDeps({
        scriptResult: {
          filled: true,
          selector: 'input[name=email]',
          value: 'user@example.com',
        },
      });
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_fill_form');
      expect(handler).toBeDefined();
      setupScanState(1, { e3: 'input[name=email]' });

      // @step When I call browser_fill_form with selector "@e3" and value "user@example.com"
      const result = await handler!({
        selector: '@e3',
        value: 'user@example.com',
      });

      // @step Then the handler should resolve "@e3" to "input[name=email]"
      const executeScript = deps.scripting.executeScript as ReturnType<
        typeof vi.fn
      >;
      expect(executeScript).toHaveBeenCalled();
      const callArgs = executeScript.mock.calls[0][0] as {
        args: [string, string];
      };
      expect(callArgs.args[0]).toBe('input[name=email]');

      // @step And the field "input[name=email]" should be filled with "user@example.com"
      expect(result.isError).toBeUndefined();
      expect(callArgs.args[1]).toBe('user@example.com');
    });
  });

  describe('Scenario: Click with nonexistent ref returns error', () => {
    it('should return error when ref does not exist in scan state', async () => {
      // @step Given a page has been scanned with refs "e1" through "e5"
      const deps = createDeps();
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_click_element');
      expect(handler).toBeDefined();
      setupScanState(1, {
        e1: '#btn1',
        e2: '#btn2',
        e3: '#btn3',
        e4: '#btn4',
        e5: '#btn5',
      });

      // @step When I call browser_click_element with selector "@e99"
      const result = await handler!({ selector: '@e99' });

      // @step Then the handler should return an error
      expect(result.isError).toBe(true);

      // @step And the error message should contain "Ref @e99 not found"
      const content = result.content[0] as { type: 'text'; text: string };
      expect(content.text).toContain('Ref @e99 not found');

      // @step And the error message should suggest running browser_scan_page
      expect(content.text).toContain('browser_scan_page');
    });
  });

  describe('Scenario: Click with ref on tab with no prior scan returns error', () => {
    it('should return error when no scan state exists for the tab', async () => {
      // @step Given no page scan has been performed on the active tab
      const deps = createDeps();
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_click_element');
      expect(handler).toBeDefined();
      // No setupScanState call — no scan exists

      // @step When I call browser_click_element with selector "@e1"
      const result = await handler!({ selector: '@e1' });

      // @step Then the handler should return an error
      expect(result.isError).toBe(true);

      // @step And the error message should contain "Ref @e1 not found"
      const content = result.content[0] as { type: 'text'; text: string };
      expect(content.text).toContain('Ref @e1 not found');
    });
  });

  describe('Scenario: Click with raw CSS selector passes through unchanged', () => {
    it('should pass raw CSS selector directly to executeScript', async () => {
      // @step Given a page has been scanned with some refs
      const deps = createDeps({
        scriptResult: { clicked: true, selector: '#submit' },
      });
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_click_element');
      expect(handler).toBeDefined();
      setupScanState(1, { e1: '#other-btn' });

      // @step When I call browser_click_element with selector "#submit"
      const result = await handler!({ selector: '#submit' });

      // @step Then the handler should use "#submit" as the CSS selector directly
      const executeScript = deps.scripting.executeScript as ReturnType<
        typeof vi.fn
      >;
      const callArgs = executeScript.mock.calls[0][0] as {
        args: [string];
      };
      expect(callArgs.args[0]).toBe('#submit');

      // @step And the element "#submit" should be clicked
      expect(result.isError).toBeUndefined();
    });
  });

  describe('Scenario: Fill form with raw CSS selector passes through unchanged', () => {
    it('should pass raw CSS selector directly to executeScript for fill', async () => {
      // @step Given a page has been scanned with some refs
      const deps = createDeps({
        scriptResult: {
          filled: true,
          selector: 'input[name=email]',
          value: 'test@test.com',
        },
      });
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_fill_form');
      expect(handler).toBeDefined();
      setupScanState(1, { e1: '#other-field' });

      // @step When I call browser_fill_form with selector "input[name=email]" and value "test@test.com"
      await handler!({
        selector: 'input[name=email]',
        value: 'test@test.com',
      });

      // @step Then the handler should use "input[name=email]" as the CSS selector directly
      const executeScript = deps.scripting.executeScript as ReturnType<
        typeof vi.fn
      >;
      const callArgs = executeScript.mock.calls[0][0] as {
        args: [string, string];
      };
      expect(callArgs.args[0]).toBe('input[name=email]');

      // @step And the field should be filled with "test@test.com"
      expect(callArgs.args[1]).toBe('test@test.com');
    });
  });

  describe('Scenario: Selector with @ in the middle is not treated as a ref', () => {
    it('should treat selector with @ in middle as raw CSS selector', async () => {
      // @step Given a page has been scanned with some refs
      const deps = createDeps({
        scriptResult: { clicked: true, selector: 'div@e1' },
      });
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_click_element');
      expect(handler).toBeDefined();
      setupScanState(1, { e1: '#btn1' });

      // @step When I call browser_click_element with selector "div@e1"
      await handler!({ selector: 'div@e1' });

      // @step Then the handler should use "div@e1" as the CSS selector directly
      const executeScript = deps.scripting.executeScript as ReturnType<
        typeof vi.fn
      >;
      const callArgs = executeScript.mock.calls[0][0] as {
        args: [string];
      };
      expect(callArgs.args[0]).toBe('div@e1');

      // @step And the selector should NOT be resolved through the ref map
      // Verify executeScript was called with the raw selector, not '#btn1'
      expect(callArgs.args[0]).not.toBe('#btn1');
    });
  });
});
