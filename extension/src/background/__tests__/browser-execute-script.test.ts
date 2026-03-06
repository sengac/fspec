/**
 * Feature: spec/features/browser-script-execution.feature
 *
 * This test file validates the acceptance criteria for EXT-010:
 * browser_execute_script returns null because eval() is blocked by
 * MV3 CSP in the extension's isolated world. Fix replaces eval() with
 * chrome.userScripts API.
 *
 * Tests the browser_execute_script handler in browser-tools.ts and
 * the userScripts world configuration in service worker startup.
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

/** Helper: create a mock userScripts API that succeeds */
function createMockUserScripts(
  executeResult?: unknown
): ChromeUserScriptsForTools {
  return {
    configureWorld: vi.fn().mockResolvedValue(undefined),
    execute: vi.fn().mockResolvedValue([{ result: executeResult ?? null }]),
  };
}

/** Helper: create full deps with optional userScripts */
function createDeps(options?: {
  userScripts?: ChromeUserScriptsForTools | undefined;
  tabs?: Partial<ChromeTabsForTools>;
}): BrowserToolsDeps {
  return {
    tabs: createMockTabs(options?.tabs),
    scripting: createMockScripting(),
    windows: createMockWindows(),
    userScripts: options?.userScripts,
  };
}

describe('Feature: browser_execute_script CSP fix', () => {
  describe('Scenario: Execute script that returns a string value', () => {
    it('should return the actual page title instead of null', async () => {
      // @step Given the USER_SCRIPT world is configured with permissive CSP
      const userScripts = createMockUserScripts('Example Page');
      const deps = createDeps({ userScripts });
      const tools = createBrowserTools(deps);

      // @step And a tab is open with a web page
      const handler = tools.getHandler('browser_execute_script');
      expect(handler).toBeDefined();

      // @step When I call browser_execute_script with code "document.title"
      const result = await handler!({ code: 'document.title' });

      // Verify the code was wrapped in try-catch IIFE (not sent raw)
      const executeCall = (userScripts.execute as ReturnType<typeof vi.fn>).mock
        .calls[0][0] as { js: Array<{ code: string }> };
      expect(executeCall.js[0].code).toContain('try{return eval(');
      expect(executeCall.js[0].code).toContain('__fspec_error');
      expect(executeCall.js[0].code).not.toBe('document.title');

      // @step Then I should receive a text result containing the page title
      expect(result.content[0].type).toBe('text');
      expect((result.content[0] as { type: 'text'; text: string }).text).toBe(
        'Example Page'
      );

      // @step And the result should not be "null"
      expect(
        (result.content[0] as { type: 'text'; text: string }).text
      ).not.toBe('null');
    });
  });

  describe('Scenario: Execute script that returns an expression result', () => {
    it('should return the evaluated expression result', async () => {
      // @step Given the USER_SCRIPT world is configured with permissive CSP
      const userScripts = createMockUserScripts(2);
      const deps = createDeps({ userScripts });
      const tools = createBrowserTools(deps);

      // @step And a tab is open with a web page
      const handler = tools.getHandler('browser_execute_script');
      expect(handler).toBeDefined();

      // @step When I call browser_execute_script with code "1 + 1"
      const result = await handler!({ code: '1 + 1' });

      // @step Then I should receive a text result containing "2"
      expect((result.content[0] as { type: 'text'; text: string }).text).toBe(
        '2'
      );
    });
  });

  describe('Scenario: Execute script that throws an error', () => {
    it('should return an MCP error result with the error message', async () => {
      // @step Given the USER_SCRIPT world is configured with permissive CSP
      //
      // When user code throws, Chrome resolves (not rejects) with {result: null}
      // because PausableScriptExecutor::HandleResults in Blink gets an empty
      // V8 value from GetSuccessValueOrEmpty(), skips conversion, and the
      // callback fires with nullopt → base::Value() NONE → JS null.
      //
      // Our wrapCodeWithErrorHandling() wraps the code in a try-catch IIFE,
      // so Chrome actually returns the error sentinel object instead of null.
      const userScripts = createMockUserScripts({
        __fspec_error: true,
        message: 'test error',
      });
      const deps = createDeps({ userScripts });
      const tools = createBrowserTools(deps);

      // @step And a tab is open with a web page
      const handler = tools.getHandler('browser_execute_script');
      expect(handler).toBeDefined();

      // @step When I call browser_execute_script with code that throws an error
      const result = await handler!({
        code: 'throw new Error("test error")',
      });

      // @step Then I should receive an MCP error result
      expect(result.isError).toBe(true);

      // @step And the error message should contain the thrown error details
      expect(
        (result.content[0] as { type: 'text'; text: string }).text
      ).toContain('test error');
    });

    it('should also handle infrastructure-level promise rejections', async () => {
      // Infrastructure errors (tab not found, permissions denied) DO reject
      // the promise — verified via UserScriptsExecuteFunction::OnScriptExecuted
      // in user_scripts_api.cc: single-frame error calls Respond(Error(...))
      const userScripts = createMockUserScripts();
      userScripts.execute = vi
        .fn()
        .mockRejectedValue(new Error('No tab with id: 999'));
      const deps = createDeps({ userScripts });
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_execute_script');

      const result = await handler!({ code: 'document.title' });
      expect(result.isError).toBe(true);
      expect(
        (result.content[0] as { type: 'text'; text: string }).text
      ).toContain('No tab with id: 999');
    });

    it('should handle InjectionResult.error field if Chrome populates it', async () => {
      // Defensive: user_scripts.idl defines InjectionResult.error as mutually
      // exclusive with result. Chrome doesn't populate it yet (the comment in
      // OnScriptExecuted says "In the future, we can bubble up these error
      // messages"), but we should handle it when they do.
      const userScripts = createMockUserScripts();
      userScripts.execute = vi
        .fn()
        .mockResolvedValue([
          { error: 'Script threw: ReferenceError: x is not defined' },
        ]);
      const deps = createDeps({ userScripts });
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_execute_script');

      const result = await handler!({ code: 'x' });
      expect(result.isError).toBe(true);
      expect(
        (result.content[0] as { type: 'text'; text: string }).text
      ).toContain('ReferenceError');
    });
  });

  describe('Scenario: Execute script when userScripts API is unavailable', () => {
    it('should return an error explaining how to enable user scripts', async () => {
      // @step Given the userScripts API is not available
      const deps = createDeps({ userScripts: undefined });
      const tools = createBrowserTools(deps);

      // @step And a tab is open with a web page
      const handler = tools.getHandler('browser_execute_script');
      expect(handler).toBeDefined();

      // @step When I call browser_execute_script with any code
      const result = await handler!({ code: 'document.title' });

      // @step Then I should receive an MCP error result
      expect(result.isError).toBe(true);

      // @step And the error message should explain how to enable user scripts
      const errorText = (result.content[0] as { type: 'text'; text: string })
        .text;
      expect(errorText).toContain('userScripts');
    });
  });

  describe('Scenario: Configure USER_SCRIPT world on service worker startup', () => {
    it('should call configureWorld with a CSP that allows unsafe-eval and unsafe-inline', async () => {
      // @step Given the extension service worker is starting
      const userScripts = createMockUserScripts();

      // @step When the userScripts API is available
      const deps = createDeps({ userScripts });
      createBrowserTools(deps);

      // @step Then configureWorld should be called with a CSP that allows unsafe-eval
      expect(userScripts.configureWorld).toHaveBeenCalled();
      const callArgs = (userScripts.configureWorld as ReturnType<typeof vi.fn>)
        .mock.calls[0][0] as { csp: string };
      expect(callArgs.csp).toContain('unsafe-eval');

      // @step And configureWorld should be called with a CSP that allows unsafe-inline
      expect(callArgs.csp).toContain('unsafe-inline');
    });
  });

  describe('Scenario: Manifest includes userScripts permission', () => {
    it('should include the userScripts permission in manifest.json', async () => {
      // @step Given the extension manifest.json file
      const { readFile } = await import('fs/promises');
      const { join } = await import('path');
      const manifestPath = join(
        import.meta.dirname ?? '.',
        '..',
        '..',
        '..',
        'manifest.json'
      );
      const manifestContent = await readFile(manifestPath, 'utf-8');
      const manifest = JSON.parse(manifestContent) as {
        permissions: string[];
      };

      // @step Then it should include the "userScripts" permission
      expect(manifest.permissions).toContain('userScripts');
    });
  });
});
