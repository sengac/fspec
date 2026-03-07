/**
 * Feature: spec/features/iframe-aware-dom-scanning.feature
 *
 * This test file validates all acceptance criteria for LOCATE-009:
 * Iframe-Aware DOM Scanning.
 *
 * Tests that browser_scan_page, browser_click_element, browser_fill_form,
 * and browser_diff_page discover and interact with elements inside iframes.
 * Uses mock Chrome APIs (getAllFrames, executeScript with frameIds) to
 * simulate multi-frame scanning without a real browser.
 */

import { readFileSync } from 'fs';
import { join } from 'path';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { createBrowserTools } from '../browser-tools';
import type {
  BrowserToolsDeps,
  ChromeTabsForTools,
  ChromeScriptingForTools,
  ChromeWindowsForTools,
  ChromeUserScriptsForTools,
} from '../browser-tools';
import { _resetForTesting, getTabScanState } from '../ref-state';
import { mergeFrameResults } from '../iframe-scanner';

const EXTENSION_ROOT = join(__dirname, '..', '..', '..');

/* ── Types for iframe-aware mocking ──────────────────────────── */

/** Simulated frame info from chrome.webNavigation.getAllFrames */
interface MockFrameInfo {
  frameId: number;
  parentFrameId: number;
  url: string;
  documentId: string;
  documentLifecycle: string;
  frameType: string;
  errorOccurred: boolean;
}

/** Simulated per-frame scan result */
interface MockFrameScanResult {
  elements: Array<{
    tagName: string;
    role: string;
    name: string;
    selector: string;
    interactive: boolean;
    depth: number;
    attributes: Record<string, string>;
  }>;
  metadata: {
    url: string;
    title: string;
    viewportWidth: number;
    viewportHeight: number;
    totalElements: number;
  };
}

/* ── Mock factories ──────────────────────────────────────────── */

function createMockTabs(ov?: Partial<ChromeTabsForTools>): ChromeTabsForTools {
  return {
    query: vi
      .fn()
      .mockResolvedValue([
        { id: 1, url: 'https://example.com', title: 'Test' },
      ]),
    update: vi.fn().mockResolvedValue({ id: 1 }),
    remove: vi.fn().mockResolvedValue(undefined),
    captureVisibleTab: vi.fn().mockResolvedValue('data:image/png;base64,abc'),
    goBack: vi.fn().mockResolvedValue(undefined),
    goForward: vi.fn().mockResolvedValue(undefined),
    get: vi.fn().mockResolvedValue({
      id: 1,
      windowId: 1,
      url: 'https://example.com',
      title: 'Test',
    }),
    onUpdated: { addListener: vi.fn(), removeListener: vi.fn() },
    create: vi.fn().mockResolvedValue({ id: 2 }),
    ...ov,
  };
}

function createMockWindows(): ChromeWindowsForTools {
  return { update: vi.fn().mockResolvedValue(undefined) };
}

function createMockUserScripts(): ChromeUserScriptsForTools {
  return {
    configureWorld: vi.fn().mockResolvedValue(undefined),
    execute: vi.fn().mockResolvedValue([{ result: null }]),
  };
}

function createMockScripting(
  executeScript: ReturnType<typeof vi.fn>
): ChromeScriptingForTools {
  return { executeScript } as ChromeScriptingForTools;
}

/* ── Scan result builders ────────────────────────────────────── */

/** Build a main frame scan result with heading + email input */
function buildMainFrameResult(): MockFrameScanResult {
  return {
    elements: [
      {
        tagName: 'H1',
        role: 'heading',
        name: 'My Page',
        selector: '',
        interactive: false,
        depth: 0,
        attributes: { level: '1' },
      },
      {
        tagName: 'INPUT',
        role: 'textbox',
        name: 'Email',
        selector: '#email',
        interactive: true,
        depth: 0,
        attributes: { type: 'email' },
      },
    ],
    metadata: {
      url: 'https://example.com',
      title: 'Test',
      viewportWidth: 1280,
      viewportHeight: 720,
      totalElements: 10,
    },
  };
}

/** Build an iframe scan result simulating Stripe payment fields */
function buildStripeFrameResult(): MockFrameScanResult {
  return {
    elements: [
      {
        tagName: 'INPUT',
        role: 'textbox',
        name: 'Card Number',
        selector: '#card-number',
        interactive: true,
        depth: 0,
        attributes: { type: 'text' },
      },
      {
        tagName: 'INPUT',
        role: 'textbox',
        name: 'Expiry',
        selector: '#expiry',
        interactive: true,
        depth: 0,
        attributes: { type: 'text' },
      },
      {
        tagName: 'INPUT',
        role: 'textbox',
        name: 'CVC',
        selector: '#cvc',
        interactive: true,
        depth: 0,
        attributes: { type: 'text' },
      },
      {
        tagName: 'BUTTON',
        role: 'button',
        name: 'Pay',
        selector: '#pay-btn',
        interactive: true,
        depth: 0,
        attributes: {},
      },
    ],
    metadata: {
      url: 'https://js.stripe.com/v3/',
      title: '',
      viewportWidth: 400,
      viewportHeight: 200,
      totalElements: 4,
    },
  };
}

/** Build a simple page result with no iframes */
function buildSimplePageResult(): MockFrameScanResult {
  return {
    elements: [
      {
        tagName: 'H1',
        role: 'heading',
        name: 'Simple Page',
        selector: '',
        interactive: false,
        depth: 0,
        attributes: { level: '1' },
      },
      {
        tagName: 'INPUT',
        role: 'textbox',
        name: 'Search',
        selector: '#search',
        interactive: true,
        depth: 0,
        attributes: { type: 'text' },
      },
      {
        tagName: 'BUTTON',
        role: 'button',
        name: 'Go',
        selector: '#go-btn',
        interactive: true,
        depth: 0,
        attributes: {},
      },
    ],
    metadata: {
      url: 'https://example.com',
      title: 'Simple',
      viewportWidth: 1280,
      viewportHeight: 720,
      totalElements: 5,
    },
  };
}

describe('Feature: Iframe-Aware DOM Scanning', () => {
  beforeEach(() => {
    _resetForTesting();
  });

  describe('Scenario: Scan a page with a cross-origin payment iframe and receive nested tree with frame-prefixed refs', () => {
    it('should return main frame elements with simple refs and iframe children with frame-prefixed refs', async () => {
      // @step Given a page with a heading, an email input, and a cross-origin Stripe payment iframe containing card number, expiry, CVC, and pay button fields
      const frames: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
        {
          frameId: 5,
          parentFrameId: 0,
          url: 'https://js.stripe.com/v3/',
          documentId: 'doc-stripe',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
      ];
      const mainResult = buildMainFrameResult();
      const stripeResult = buildStripeFrameResult();

      const executeScript = vi
        .fn()
        .mockImplementation((injection: Record<string, unknown>) => {
          const target = injection.target as {
            tabId: number;
            frameIds?: number[];
          };
          if (target.frameIds && target.frameIds[0] === 5) {
            return Promise.resolve([
              { result: stripeResult, frameId: 5, documentId: 'doc-stripe' },
            ]);
          }
          return Promise.resolve([
            { result: mainResult, frameId: 0, documentId: 'doc-main' },
          ]);
        });

      const webNavigation = { getAllFrames: vi.fn().mockResolvedValue(frames) };
      const deps: BrowserToolsDeps = {
        tabs: createMockTabs(),
        scripting: createMockScripting(executeScript),
        windows: createMockWindows(),
        userScripts: createMockUserScripts(),
        webNavigation,
      };
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_scan_page');
      expect(handler).toBeDefined();

      // @step When I call browser_scan_page with default parameters
      const result = await handler!({});

      // @step Then the result should contain main frame elements with simple refs e1 and e2
      const text = (result.content[0] as { text: string }).text;
      expect(text).toContain('[ref=e1]');

      // @step And the result should contain an iframe element showing the iframe's src URL
      expect(text).toContain('iframe');
      expect(text).toContain('stripe.com');

      // @step And the iframe's children should appear nested under the iframe element with refs f5e1, f5e2, f5e3, f5e4
      expect(text).toContain('[ref=f5e1]');
      expect(text).toContain('[ref=f5e2]');
      expect(text).toContain('[ref=f5e3]');
      expect(text).toContain('[ref=f5e4]');

      // @step And the tree should show the iframe content indented one level deeper than the iframe element itself
      const lines = text.split('\n');
      const iframeLine = lines.find(
        (l: string) => l.includes('iframe') && l.includes('stripe')
      );
      const f5e1Line = lines.find((l: string) => l.includes('[ref=f5e1]'));
      if (iframeLine && f5e1Line) {
        const iframeIndent = iframeLine.match(/^(\s*)/)?.[1]?.length ?? 0;
        const childIndent = f5e1Line.match(/^(\s*)/)?.[1]?.length ?? 0;
        expect(childIndent).toBeGreaterThan(iframeIndent);
      }
    });
  });

  // --- More scenarios below ---

  describe('Scenario: Page with no iframes returns backward-compatible simple refs', () => {
    it('should return only simple refs with no frame-prefixed refs', async () => {
      // @step Given a page with a heading, a text input, and a button but no iframes
      const frames: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
      ];
      const simpleResult = buildSimplePageResult();

      const executeScript = vi
        .fn()
        .mockResolvedValue([
          { result: simpleResult, frameId: 0, documentId: 'doc-main' },
        ]);
      const webNavigation = { getAllFrames: vi.fn().mockResolvedValue(frames) };
      const deps: BrowserToolsDeps = {
        tabs: createMockTabs(),
        scripting: createMockScripting(executeScript),
        windows: createMockWindows(),
        userScripts: createMockUserScripts(),
        webNavigation,
      };
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_scan_page');

      // @step When I call browser_scan_page
      const result = await handler!({});
      const text = (result.content[0] as { text: string }).text;

      // @step Then the result should contain only main frame elements with simple refs e1, e2, e3
      expect(text).toContain('[ref=e1]');
      expect(text).toContain('[ref=e2]');

      // @step And no frame-prefixed refs should appear in the output
      expect(text).not.toMatch(/\[ref=f\d+e\d+\]/);

      // @step And the behavior should be identical to pre-iframe-support scanning
      expect(result.isError).toBeUndefined();
    });
  });

  // --- More scenarios below 2 ---

  describe('Scenario: Click an element inside an iframe using frame-prefixed ref', () => {
    it('should parse f5e4 into frameId 5 and elementRef e4, then click in the correct frame', async () => {
      // @step Given a previous scan stored refs including f5e4 with frameId 5 and CSS selector for a pay button
      const frames: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
        {
          frameId: 5,
          parentFrameId: 0,
          url: 'https://js.stripe.com/v3/',
          documentId: 'doc-stripe',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
      ];

      const executeScript = vi
        .fn()
        .mockImplementation((injection: Record<string, unknown>) => {
          const target = injection.target as {
            tabId: number;
            frameIds?: number[];
          };
          const args = injection.args as unknown[] | undefined;
          if (target.frameIds && target.frameIds[0] === 5) {
            // Marker injection: args is [number]
            if (args && typeof args[0] === 'number') {
              return Promise.resolve([{ result: undefined }]);
            }
            // Click call: args is [string] (CSS selector)
            if (args && typeof args[0] === 'string') {
              return Promise.resolve([
                { result: { clicked: true, selector: args[0] } },
              ]);
            }
            // Scan call: args is [boolean, string?]
            return Promise.resolve([
              { result: buildStripeFrameResult(), frameId: 5 },
            ]);
          }
          // Main frame scan
          return Promise.resolve([
            { result: buildMainFrameResult(), frameId: 0 },
          ]);
        });
      const webNavigation = { getAllFrames: vi.fn().mockResolvedValue(frames) };
      const deps: BrowserToolsDeps = {
        tabs: createMockTabs(),
        scripting: createMockScripting(executeScript),
        windows: createMockWindows(),
        userScripts: createMockUserScripts(),
        webNavigation,
      };
      const tools = createBrowserTools(deps);

      // First do a scan to populate ref state
      const scanHandler = tools.getHandler('browser_scan_page');
      await scanHandler!({});

      // @step When I call browser_click_element with selector "@f5e4"
      const clickHandler = tools.getHandler('browser_click_element');
      expect(clickHandler).toBeDefined();
      const result = await clickHandler!({ selector: '@f5e4' });

      // @step Then resolveRefSelector should parse "f5e4" into frameId 5 and elementRef "e4"
      // @step And executeScript should target frameIds [5] to click the element within the iframe
      const calls = executeScript.mock.calls;
      const clickCall = calls.find((c: unknown[]) => {
        const inj = c[0] as Record<string, unknown>;
        const tgt = inj.target as { frameIds?: number[] };
        const callArgs = inj.args as string[] | undefined;
        return (
          tgt.frameIds?.includes(5) && callArgs && callArgs[0] === '#pay-btn'
        );
      });
      expect(clickCall).toBeDefined();

      // @step And the result should confirm the click succeeded
      expect(result.isError).toBeUndefined();
      const text = (result.content[0] as { text: string }).text;
      const parsed = JSON.parse(text) as { clicked: boolean; selector: string };
      expect(parsed.clicked).toBe(true);
      expect(parsed.selector).toBe('#pay-btn');
    });
  });

  // --- More scenarios below 3 ---

  describe('Scenario: Fill a form field inside an iframe using frame-prefixed ref', () => {
    it('should resolve @f5e1 to the correct iframe and fill the input', async () => {
      // @step Given a previous scan stored refs including f5e1 with frameId 5 and CSS selector for a card number input
      const frames: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
        {
          frameId: 5,
          parentFrameId: 0,
          url: 'https://js.stripe.com/v3/',
          documentId: 'doc-stripe',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
      ];

      const executeScript = vi
        .fn()
        .mockImplementation((injection: Record<string, unknown>) => {
          const target = injection.target as {
            tabId: number;
            frameIds?: number[];
          };
          const args = injection.args as unknown[] | undefined;
          if (target.frameIds && target.frameIds[0] === 5) {
            // Marker injection: args is [number]
            if (args && typeof args[0] === 'number') {
              return Promise.resolve([{ result: undefined }]);
            }
            // Fill call: args is [string, string] (selector, value)
            if (args && typeof args[0] === 'string') {
              return Promise.resolve([
                { result: { filled: true, selector: args[0], value: args[1] } },
              ]);
            }
            // Scan call: args is [boolean, string?]
            return Promise.resolve([
              { result: buildStripeFrameResult(), frameId: 5 },
            ]);
          }
          return Promise.resolve([
            { result: buildMainFrameResult(), frameId: 0 },
          ]);
        });
      const webNavigation = { getAllFrames: vi.fn().mockResolvedValue(frames) };
      const deps: BrowserToolsDeps = {
        tabs: createMockTabs(),
        scripting: createMockScripting(executeScript),
        windows: createMockWindows(),
        userScripts: createMockUserScripts(),
        webNavigation,
      };
      const tools = createBrowserTools(deps);

      // First scan to populate ref state
      await tools.getHandler('browser_scan_page')!({});

      // @step When I call browser_fill_form with selector "@f5e1" and value "4242424242424242"
      const fillHandler = tools.getHandler('browser_fill_form');
      expect(fillHandler).toBeDefined();
      const result = await fillHandler!({
        selector: '@f5e1',
        value: '4242424242424242',
      });

      // @step Then executeScript should target frameIds [5] to fill the input within the iframe
      const calls = executeScript.mock.calls;
      const fillCall = calls.find((c: unknown[]) => {
        const inj = c[0] as Record<string, unknown>;
        const tgt = inj.target as { frameIds?: number[] };
        const callArgs = inj.args as string[] | undefined;
        return (
          tgt.frameIds?.includes(5) &&
          callArgs &&
          callArgs[1] === '4242424242424242'
        );
      });
      expect(fillCall).toBeDefined();

      // @step And the result should confirm the value was set
      expect(result.isError).toBeUndefined();
      const text = (result.content[0] as { text: string }).text;
      const parsed = JSON.parse(text) as {
        filled: boolean;
        selector: string;
        value: string;
      };
      expect(parsed.filled).toBe(true);
      expect(parsed.value).toBe('4242424242424242');
    });
  });

  // --- More scenarios below 4 ---

  describe('Scenario: Ad-heavy page with excess iframes respects maxFrames limit', () => {
    it('should scan at most maxFrames iframes and mark the rest as skipped', async () => {
      // @step Given a page with 25 iframes including both same-origin content iframes and small cross-origin ad iframes
      const frames: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
      ];
      for (let i = 1; i <= 25; i++) {
        frames.push({
          frameId: i,
          parentFrameId: 0,
          url:
            i <= 5
              ? `https://example.com/widget${i}`
              : `https://ads.example.com/ad${i}`,
          documentId: `doc-frame-${i}`,
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        });
      }

      const adFrameResult: MockFrameScanResult = {
        elements: [
          {
            tagName: 'A',
            role: 'link',
            name: 'Ad',
            selector: '#ad-link',
            interactive: true,
            depth: 0,
            attributes: {},
          },
        ],
        metadata: {
          url: 'https://ads.example.com',
          title: '',
          viewportWidth: 300,
          viewportHeight: 250,
          totalElements: 1,
        },
      };

      const executeScript = vi
        .fn()
        .mockImplementation((injection: Record<string, unknown>) => {
          const target = injection.target as {
            tabId: number;
            frameIds?: number[];
          };
          if (target.frameIds && target.frameIds[0] > 0) {
            return Promise.resolve([
              { result: adFrameResult, frameId: target.frameIds[0] },
            ]);
          }
          return Promise.resolve([
            { result: buildMainFrameResult(), frameId: 0 },
          ]);
        });
      const webNavigation = { getAllFrames: vi.fn().mockResolvedValue(frames) };
      const deps: BrowserToolsDeps = {
        tabs: createMockTabs(),
        scripting: createMockScripting(executeScript),
        windows: createMockWindows(),
        userScripts: createMockUserScripts(),
        webNavigation,
      };
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_scan_page');

      // @step When I call browser_scan_page with maxFrames set to 10
      const result = await handler!({ maxFrames: 10 });
      const text = (result.content[0] as { text: string }).text;

      // @step Then at most 10 iframes should be scanned for content
      // Count frame-prefixed refs — each scanned frame contributes at least one
      const scannedFrameIds = new Set<string>();
      const refMatches = text.matchAll(/\[ref=f(\d+)e\d+\]/g);
      for (const m of refMatches) {
        scannedFrameIds.add(m[1]);
      }
      expect(scannedFrameIds.size).toBeLessThanOrEqual(10);

      // @step And the remaining iframes should appear in the tree as "iframe [skipped]"
      expect(text).toContain('[skipped]');

      // @step And same-origin and larger iframes should be prioritized over small cross-origin ones
      // Same-origin frames (1-5) should be among the scanned ones
      for (let i = 1; i <= 5; i++) {
        expect(scannedFrameIds.has(String(i))).toBe(true);
      }
    });
  });

  // --- More scenarios below 5 ---

  describe('Scenario: Nested iframes are scanned at all depths with correct refs', () => {
    it('should show nested indentation and use the direct frame ID for refs', async () => {
      // @step Given a page with an iframe containing another iframe inside it
      // @step And getAllFrames returns frames at all nesting levels with parentFrameId chain
      const frames: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
        {
          frameId: 3,
          parentFrameId: 0,
          url: 'https://outer.example.com',
          documentId: 'doc-outer',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
        {
          frameId: 12,
          parentFrameId: 3,
          url: 'https://inner.example.com',
          documentId: 'doc-inner',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
      ];

      const outerResult: MockFrameScanResult = {
        elements: [
          {
            tagName: 'BUTTON',
            role: 'button',
            name: 'Outer Button',
            selector: '#outer-btn',
            interactive: true,
            depth: 0,
            attributes: {},
          },
        ],
        metadata: {
          url: 'https://outer.example.com',
          title: '',
          viewportWidth: 800,
          viewportHeight: 600,
          totalElements: 1,
        },
      };
      const innerResult: MockFrameScanResult = {
        elements: [
          {
            tagName: 'INPUT',
            role: 'textbox',
            name: 'Inner Input',
            selector: '#inner-input',
            interactive: true,
            depth: 0,
            attributes: {},
          },
        ],
        metadata: {
          url: 'https://inner.example.com',
          title: '',
          viewportWidth: 400,
          viewportHeight: 300,
          totalElements: 1,
        },
      };

      const executeScript = vi
        .fn()
        .mockImplementation((injection: Record<string, unknown>) => {
          const target = injection.target as {
            tabId: number;
            frameIds?: number[];
          };
          if (target.frameIds?.[0] === 12) {
            return Promise.resolve([{ result: innerResult, frameId: 12 }]);
          }
          if (target.frameIds?.[0] === 3) {
            return Promise.resolve([{ result: outerResult, frameId: 3 }]);
          }
          return Promise.resolve([
            { result: buildMainFrameResult(), frameId: 0 },
          ]);
        });
      const webNavigation = { getAllFrames: vi.fn().mockResolvedValue(frames) };
      const deps: BrowserToolsDeps = {
        tabs: createMockTabs(),
        scripting: createMockScripting(executeScript),
        windows: createMockWindows(),
        userScripts: createMockUserScripts(),
        webNavigation,
      };
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_scan_page');

      // @step When I call browser_scan_page
      const result = await handler!({});
      const text = (result.content[0] as { text: string }).text;

      // @step Then the tree should show nested indentation matching the iframe nesting depth
      expect(text).toContain('Outer Button');
      expect(text).toContain('Inner Input');

      // Outer iframe (child of main) should be at depth 0
      const lines = text.split('\n');
      const outerIframeLine = lines.find(
        (l: string) => l.includes('iframe') && l.includes('outer.example.com')
      );
      const innerIframeLine = lines.find(
        (l: string) => l.includes('iframe') && l.includes('inner.example.com')
      );
      const outerBtnLine = lines.find((l: string) => l.includes('[ref=f3e1]'));
      const innerInputLine = lines.find((l: string) =>
        l.includes('[ref=f12e1]')
      );
      expect(outerIframeLine).toBeDefined();
      expect(innerIframeLine).toBeDefined();
      expect(outerBtnLine).toBeDefined();
      expect(innerInputLine).toBeDefined();

      const indent = (line: string): number =>
        line.match(/^(\s*)/)?.[1]?.length ?? 0;
      // Outer iframe at depth 0, its children at depth 1
      expect(indent(outerIframeLine!)).toBe(0);
      expect(indent(outerBtnLine!)).toBe(2);
      // Inner iframe at depth 2 (inside outer iframe content), its children at depth 3
      expect(indent(innerIframeLine!)).toBe(4);
      expect(indent(innerInputLine!)).toBe(6);

      // @step And refs should use the direct frame's ID not the parent chain
      // @step And a deeply nested element should use ref format f12e1 for frame 12 element 1
      expect(text).toContain('[ref=f12e1]');
      expect(text).toContain('[ref=f3e1]');
      // No chained ref like f3f12e1
      expect(text).not.toMatch(/f\d+f\d+e\d+/);
    });
  });

  // --- More scenarios below 6 ---

  describe('Scenario: Sandboxed iframe without allow-scripts is still scanned via ISOLATED world', () => {
    it('should scan sandboxed iframe content because executeScript runs in ISOLATED world', async () => {
      // @step Given a page with a sandboxed iframe having sandbox attribute "allow-same-origin" but not "allow-scripts"
      const frames: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
        {
          frameId: 7,
          parentFrameId: 0,
          url: 'https://sandboxed.example.com',
          documentId: 'doc-sandbox',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
      ];

      const sandboxedResult: MockFrameScanResult = {
        elements: [
          {
            tagName: 'INPUT',
            role: 'textbox',
            name: 'Sandboxed Input',
            selector: '#sandbox-input',
            interactive: true,
            depth: 0,
            attributes: {},
          },
          {
            tagName: 'BUTTON',
            role: 'button',
            name: 'Submit',
            selector: '#sandbox-btn',
            interactive: true,
            depth: 0,
            attributes: {},
          },
        ],
        metadata: {
          url: 'https://sandboxed.example.com',
          title: '',
          viewportWidth: 600,
          viewportHeight: 400,
          totalElements: 2,
        },
      };

      const executeScript = vi
        .fn()
        .mockImplementation((injection: Record<string, unknown>) => {
          const target = injection.target as {
            tabId: number;
            frameIds?: number[];
          };
          if (target.frameIds?.[0] === 7) {
            return Promise.resolve([{ result: sandboxedResult, frameId: 7 }]);
          }
          return Promise.resolve([
            { result: buildMainFrameResult(), frameId: 0 },
          ]);
        });
      const webNavigation = { getAllFrames: vi.fn().mockResolvedValue(frames) };
      const deps: BrowserToolsDeps = {
        tabs: createMockTabs(),
        scripting: createMockScripting(executeScript),
        windows: createMockWindows(),
        userScripts: createMockUserScripts(),
        webNavigation,
      };
      const tools = createBrowserTools(deps);

      // @step When I call browser_scan_page
      const result = await tools.getHandler('browser_scan_page')!({});
      const text = (result.content[0] as { text: string }).text;

      // @step Then the iframe should be scanned successfully because executeScript runs in ISOLATED world
      expect(result.isError).toBeUndefined();

      // @step And the iframe's interactive elements should appear in the tree with frame-prefixed refs
      expect(text).toContain('[ref=f7e1]');
      expect(text).toContain('[ref=f7e2]');
      expect(text).toContain('Sandboxed Input');

      // @step And the sandbox attribute should not prevent scanning
      expect(text).toContain('Submit');
    });
  });

  // --- More scenarios below 7 ---

  describe('Scenario: Non-scannable chrome URLs are skipped gracefully', () => {
    it('should skip chrome-extension:// and chrome:// iframes without error', async () => {
      // @step Given a page with iframes pointing to chrome-extension:// and chrome:// URLs
      const frames: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
        {
          frameId: 2,
          parentFrameId: 0,
          url: 'chrome-extension://abcdef/popup.html',
          documentId: 'doc-ext',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
        {
          frameId: 3,
          parentFrameId: 0,
          url: 'chrome://settings/',
          documentId: 'doc-chrome',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
        {
          frameId: 4,
          parentFrameId: 0,
          url: 'https://content.example.com',
          documentId: 'doc-content',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
      ];

      const contentResult: MockFrameScanResult = {
        elements: [
          {
            tagName: 'BUTTON',
            role: 'button',
            name: 'Content Button',
            selector: '#content-btn',
            interactive: true,
            depth: 0,
            attributes: {},
          },
        ],
        metadata: {
          url: 'https://content.example.com',
          title: '',
          viewportWidth: 800,
          viewportHeight: 600,
          totalElements: 1,
        },
      };

      const executeScript = vi
        .fn()
        .mockImplementation((injection: Record<string, unknown>) => {
          const target = injection.target as {
            tabId: number;
            frameIds?: number[];
          };
          if (target.frameIds?.[0] === 4) {
            return Promise.resolve([{ result: contentResult, frameId: 4 }]);
          }
          if (target.frameIds?.[0] === 2 || target.frameIds?.[0] === 3) {
            return Promise.reject(new Error('Cannot access chrome:// URL'));
          }
          return Promise.resolve([
            { result: buildMainFrameResult(), frameId: 0 },
          ]);
        });
      const webNavigation = { getAllFrames: vi.fn().mockResolvedValue(frames) };
      const deps: BrowserToolsDeps = {
        tabs: createMockTabs(),
        scripting: createMockScripting(executeScript),
        windows: createMockWindows(),
        userScripts: createMockUserScripts(),
        webNavigation,
      };
      const tools = createBrowserTools(deps);

      // @step When I call browser_scan_page
      const result = await tools.getHandler('browser_scan_page')!({});
      const text = (result.content[0] as { text: string }).text;

      // @step Then the chrome-extension and chrome URL iframes should be skipped without error
      expect(result.isError).toBeUndefined();

      // @step And the iframe elements should still appear in the tree without nested children
      // The iframe elements themselves should be visible but with no scanned children
      expect(text).not.toContain('[ref=f2e');
      expect(text).not.toContain('[ref=f3e');

      // @step And all other scannable frames should be scanned normally
      expect(text).toContain('[ref=f4e1]');
      expect(text).toContain('Content Button');
    });
  });

  // --- More scenarios below 8 ---

  describe('Scenario: about:blank iframes with content are scanned', () => {
    it('should scan about:blank iframes that have JS-populated content', async () => {
      // @step Given a page with an about:blank iframe that has been populated with content via JavaScript
      const frames: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
        {
          frameId: 8,
          parentFrameId: 0,
          url: 'about:blank',
          documentId: 'doc-blank',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
      ];

      const blankFrameResult: MockFrameScanResult = {
        elements: [
          {
            tagName: 'INPUT',
            role: 'textbox',
            name: 'Dynamic Field',
            selector: '#dynamic-field',
            interactive: true,
            depth: 0,
            attributes: {},
          },
        ],
        metadata: {
          url: 'about:blank',
          title: '',
          viewportWidth: 500,
          viewportHeight: 300,
          totalElements: 1,
        },
      };

      const executeScript = vi
        .fn()
        .mockImplementation((injection: Record<string, unknown>) => {
          const target = injection.target as {
            tabId: number;
            frameIds?: number[];
          };
          if (target.frameIds?.[0] === 8) {
            return Promise.resolve([{ result: blankFrameResult, frameId: 8 }]);
          }
          return Promise.resolve([
            { result: buildMainFrameResult(), frameId: 0 },
          ]);
        });
      const webNavigation = { getAllFrames: vi.fn().mockResolvedValue(frames) };
      const deps: BrowserToolsDeps = {
        tabs: createMockTabs(),
        scripting: createMockScripting(executeScript),
        windows: createMockWindows(),
        userScripts: createMockUserScripts(),
        webNavigation,
      };
      const tools = createBrowserTools(deps);

      // @step When I call browser_scan_page
      const result = await tools.getHandler('browser_scan_page')!({});
      const text = (result.content[0] as { text: string }).text;

      // @step Then the about:blank iframe should be scanned because it may have same-origin JS-populated content
      expect(result.isError).toBeUndefined();

      // @step And its interactive elements should appear nested under the iframe element
      expect(text).toContain('[ref=f8e1]');
      expect(text).toContain('Dynamic Field');
    });
  });

  describe('Scenario: about:srcdoc iframes are always scanned', () => {
    it('should scan srcdoc iframes and include their elements with frame-prefixed refs', async () => {
      // @step Given a page with an iframe using srcdoc attribute containing inline HTML with form fields
      const frames: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
        {
          frameId: 9,
          parentFrameId: 0,
          url: 'about:srcdoc',
          documentId: 'doc-srcdoc',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
      ];

      const srcdocResult: MockFrameScanResult = {
        elements: [
          {
            tagName: 'INPUT',
            role: 'textbox',
            name: 'Inline Name',
            selector: '#inline-name',
            interactive: true,
            depth: 0,
            attributes: {},
          },
          {
            tagName: 'BUTTON',
            role: 'button',
            name: 'Inline Submit',
            selector: '#inline-submit',
            interactive: true,
            depth: 0,
            attributes: {},
          },
        ],
        metadata: {
          url: 'about:srcdoc',
          title: '',
          viewportWidth: 600,
          viewportHeight: 400,
          totalElements: 2,
        },
      };

      const executeScript = vi
        .fn()
        .mockImplementation((injection: Record<string, unknown>) => {
          const target = injection.target as {
            tabId: number;
            frameIds?: number[];
          };
          if (target.frameIds?.[0] === 9) {
            return Promise.resolve([{ result: srcdocResult, frameId: 9 }]);
          }
          return Promise.resolve([
            { result: buildMainFrameResult(), frameId: 0 },
          ]);
        });
      const webNavigation = { getAllFrames: vi.fn().mockResolvedValue(frames) };
      const deps: BrowserToolsDeps = {
        tabs: createMockTabs(),
        scripting: createMockScripting(executeScript),
        windows: createMockWindows(),
        userScripts: createMockUserScripts(),
        webNavigation,
      };
      const tools = createBrowserTools(deps);

      // @step When I call browser_scan_page
      const result = await tools.getHandler('browser_scan_page')!({});
      const text = (result.content[0] as { text: string }).text;

      // @step Then the srcdoc iframe should be scanned and its elements should appear in the tree
      expect(text).toContain('Inline Name');
      expect(text).toContain('Inline Submit');

      // @step And the elements should have frame-prefixed refs
      expect(text).toContain('[ref=f9e1]');
      expect(text).toContain('[ref=f9e2]');
    });
  });

  // --- More scenarios below 9 ---

  describe('Scenario: Frame-to-DOM correlation maps frameIds to iframe elements via two-pass injection', () => {
    it('should inject markers in first pass and correlate in second pass', async () => {
      // @step Given a page with multiple iframes including both same-origin and cross-origin iframes
      const frames: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
        {
          frameId: 4,
          parentFrameId: 0,
          url: 'https://example.com/widget',
          documentId: 'doc-same',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
        {
          frameId: 6,
          parentFrameId: 0,
          url: 'https://third-party.com/embed',
          documentId: 'doc-cross',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
      ];

      const sameOriginResult: MockFrameScanResult = {
        elements: [
          {
            tagName: 'BUTTON',
            role: 'button',
            name: 'Same Origin',
            selector: '#same-btn',
            interactive: true,
            depth: 0,
            attributes: {},
          },
        ],
        metadata: {
          url: 'https://example.com/widget',
          title: '',
          viewportWidth: 400,
          viewportHeight: 300,
          totalElements: 1,
        },
      };
      const crossOriginResult: MockFrameScanResult = {
        elements: [
          {
            tagName: 'BUTTON',
            role: 'button',
            name: 'Cross Origin',
            selector: '#cross-btn',
            interactive: true,
            depth: 0,
            attributes: {},
          },
        ],
        metadata: {
          url: 'https://third-party.com/embed',
          title: '',
          viewportWidth: 400,
          viewportHeight: 300,
          totalElements: 1,
        },
      };

      const markerCalls: number[] = [];
      const executeScript = vi
        .fn()
        .mockImplementation((injection: Record<string, unknown>) => {
          const target = injection.target as {
            tabId: number;
            frameIds?: number[];
          };
          const args = injection.args as unknown[] | undefined;

          // Track marker injection calls (first pass — func sets __fspec_frameId)
          if (target.frameIds && args && typeof args[0] === 'number') {
            markerCalls.push(args[0] as number);
            return Promise.resolve([{ result: undefined }]);
          }

          if (target.frameIds?.[0] === 4) {
            return Promise.resolve([{ result: sameOriginResult, frameId: 4 }]);
          }
          if (target.frameIds?.[0] === 6) {
            return Promise.resolve([{ result: crossOriginResult, frameId: 6 }]);
          }
          return Promise.resolve([
            { result: buildMainFrameResult(), frameId: 0 },
          ]);
        });
      const webNavigation = { getAllFrames: vi.fn().mockResolvedValue(frames) };
      const deps: BrowserToolsDeps = {
        tabs: createMockTabs(),
        scripting: createMockScripting(executeScript),
        windows: createMockWindows(),
        userScripts: createMockUserScripts(),
        webNavigation,
      };
      const tools = createBrowserTools(deps);

      // @step When the scan runs the two-pass injection
      const result = await tools.getHandler('browser_scan_page')!({});
      const text = (result.content[0] as { text: string }).text;

      // @step Then first pass should inject a __fspec_frameId marker into each frame
      // Marker injection calls should have been made for each subframe
      expect(markerCalls).toContain(4);
      expect(markerCalls).toContain(6);

      // @step And second pass should correlate same-origin iframes by reading contentWindow.__fspec_frameId
      // @step And cross-origin iframes should fall back to matching iframe.src against getAllFrames URL data
      // Both frame contents should appear in the tree correctly mapped
      expect(text).toContain('Same Origin');
      expect(text).toContain('Cross Origin');
      expect(text).toContain('[ref=f4e1]');
      expect(text).toContain('[ref=f6e1]');
    });
  });

  // --- More scenarios below 10 ---

  describe('Scenario: RefEntry includes frameId field for frame-aware click and fill', () => {
    it('should store frameId 0 for main frame refs and frameId N for iframe refs', async () => {
      // @step Given a completed scan of a page with main frame and iframe elements
      const frames: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
        {
          frameId: 5,
          parentFrameId: 0,
          url: 'https://js.stripe.com/v3/',
          documentId: 'doc-stripe',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
      ];

      const executeScript = vi
        .fn()
        .mockImplementation((injection: Record<string, unknown>) => {
          const target = injection.target as {
            tabId: number;
            frameIds?: number[];
          };
          if (target.frameIds?.[0] === 5) {
            return Promise.resolve([
              { result: buildStripeFrameResult(), frameId: 5 },
            ]);
          }
          return Promise.resolve([
            { result: buildMainFrameResult(), frameId: 0 },
          ]);
        });
      const webNavigation = { getAllFrames: vi.fn().mockResolvedValue(frames) };
      const deps: BrowserToolsDeps = {
        tabs: createMockTabs(),
        scripting: createMockScripting(executeScript),
        windows: createMockWindows(),
        userScripts: createMockUserScripts(),
        webNavigation,
      };
      const tools = createBrowserTools(deps);
      await tools.getHandler('browser_scan_page')!({});

      // @step When I inspect the stored RefEntry for a main frame element ref "e1"
      // @step Then the RefEntry should have frameId 0
      const state = getTabScanState(1);
      expect(state).toBeDefined();
      const e1 = state!.refs.get('e1');
      expect(e1).toBeDefined();
      expect(e1!.frameId).toBe(0);

      // @step When I inspect the stored RefEntry for an iframe element ref "f5e3"
      // @step Then the RefEntry should have frameId 5
      const f5e3 = state!.refs.get('f5e3');
      expect(f5e3).toBeDefined();
      expect(f5e3!.frameId).toBe(5);
    });
  });

  // --- More scenarios below 11 ---

  describe('Scenario: browser_diff_page produces diffs on merged multi-frame tree', () => {
    it('should diff the merged multi-frame tree including iframe content changes', async () => {
      // @step Given a previous scan of a page with an iframe containing a card number field
      const frames: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
        {
          frameId: 5,
          parentFrameId: 0,
          url: 'https://js.stripe.com/v3/',
          documentId: 'doc-stripe',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
      ];

      const successResult: MockFrameScanResult = {
        elements: [
          {
            tagName: 'DIV',
            role: 'status',
            name: 'Payment successful!',
            selector: '',
            interactive: false,
            depth: 0,
            attributes: {},
          },
        ],
        metadata: {
          url: 'https://js.stripe.com/v3/',
          title: '',
          viewportWidth: 400,
          viewportHeight: 200,
          totalElements: 1,
        },
      };

      // Track how many actual scan calls (with scanPageDOM func) happen to frame 5
      let frame5ScanCount = 0;
      const executeScript = vi
        .fn()
        .mockImplementation((injection: Record<string, unknown>) => {
          const target = injection.target as {
            tabId: number;
            frameIds?: number[];
          };
          const args = injection.args as unknown[] | undefined;

          if (target.frameIds?.[0] === 5) {
            // Marker injection: args is [number] (the frameId)
            if (args && args.length === 1 && typeof args[0] === 'number') {
              return Promise.resolve([{ result: undefined }]);
            }
            // Actual scan call
            frame5ScanCount++;
            if (frame5ScanCount <= 1) {
              return Promise.resolve([
                { result: buildStripeFrameResult(), frameId: 5 },
              ]);
            }
            // @step And the iframe content has changed to show a success message
            return Promise.resolve([{ result: successResult, frameId: 5 }]);
          }
          return Promise.resolve([
            { result: buildMainFrameResult(), frameId: 0 },
          ]);
        });
      const webNavigation = { getAllFrames: vi.fn().mockResolvedValue(frames) };
      const deps: BrowserToolsDeps = {
        tabs: createMockTabs(),
        scripting: createMockScripting(executeScript),
        windows: createMockWindows(),
        userScripts: createMockUserScripts(),
        webNavigation,
      };
      const tools = createBrowserTools(deps);

      // Initial scan
      await tools.getHandler('browser_scan_page')!({});

      // @step When I call browser_diff_page
      const diffHandler = tools.getHandler('browser_diff_page');
      expect(diffHandler).toBeDefined();
      const result = await diffHandler!({});
      const text = (result.content[0] as { text: string }).text;

      // @step Then the diff should show removals of iframe's old elements and additions of new elements
      expect(text).toContain('Payment successful');
      // Verify actual diff markers — removals of old elements and additions of new
      const lines = text.split('\n');
      const removals = lines.filter((l: string) => l.startsWith('- '));
      const additions = lines.filter((l: string) => l.startsWith('+ '));
      expect(removals.length).toBeGreaterThan(0);
      expect(additions.length).toBeGreaterThan(0);

      // @step And the diff should operate on the merged multi-frame tree
      expect(result.isError).toBeUndefined();
    });
  });

  // --- More scenarios below 12 ---

  describe('Scenario: Dynamically added iframes are discovered on re-scan', () => {
    it('should discover a newly added iframe on re-scan', async () => {
      // @step Given a page that initially has no iframes
      const framesInitial: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
      ];
      const framesAfter: MockFrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
        {
          frameId: 10,
          parentFrameId: 0,
          url: 'https://payments.example.com',
          documentId: 'doc-payment',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
      ];

      const paymentResult: MockFrameScanResult = {
        elements: [
          {
            tagName: 'INPUT',
            role: 'textbox',
            name: 'Amount',
            selector: '#amount',
            interactive: true,
            depth: 0,
            attributes: {},
          },
        ],
        metadata: {
          url: 'https://payments.example.com',
          title: '',
          viewportWidth: 400,
          viewportHeight: 300,
          totalElements: 1,
        },
      };

      let scanCallCount = 0;
      const getAllFrames = vi.fn().mockImplementation(() => {
        scanCallCount++;
        return Promise.resolve(
          scanCallCount <= 1 ? framesInitial : framesAfter
        );
      });

      const executeScript = vi
        .fn()
        .mockImplementation((injection: Record<string, unknown>) => {
          const target = injection.target as {
            tabId: number;
            frameIds?: number[];
          };
          if (target.frameIds?.[0] === 10) {
            return Promise.resolve([{ result: paymentResult, frameId: 10 }]);
          }
          return Promise.resolve([
            { result: buildSimplePageResult(), frameId: 0 },
          ]);
        });
      const webNavigation = { getAllFrames };
      const deps: BrowserToolsDeps = {
        tabs: createMockTabs(),
        scripting: createMockScripting(executeScript),
        windows: createMockWindows(),
        userScripts: createMockUserScripts(),
        webNavigation,
      };
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_scan_page');

      // First scan — no iframes
      const firstResult = await handler!({});
      const firstText = (firstResult.content[0] as { text: string }).text;
      expect(firstText).not.toMatch(/\[ref=f\d+e\d+\]/);

      // @step And a payment modal iframe is dynamically added after initial scan
      // (getAllFrames now returns framesAfter)

      // @step When I call browser_scan_page again
      const secondResult = await handler!({});
      const secondText = (secondResult.content[0] as { text: string }).text;

      // @step Then the newly added iframe should appear in the scan results
      expect(secondText).toContain('Amount');

      // @step And its interactive elements should have frame-prefixed refs
      expect(secondText).toContain('[ref=f10e1]');
    });
  });

  describe('Rule [12]: Iframe content is spliced at correct DOM position', () => {
    it('should place iframe children where the IFRAME element sits, not at the end', () => {
      // Main frame scan includes an IFRAME element between heading and footer
      const mainElements: import('../dom-scanner').RawElement[] = [
        {
          tagName: 'H1',
          role: 'heading',
          name: 'Checkout',
          selector: '',
          interactive: false,
          depth: 0,
          attributes: { level: '1' },
        },
        {
          tagName: 'IFRAME',
          role: 'iframe',
          name: '',
          selector: '',
          interactive: false,
          depth: 0,
          attributes: { src: 'https://js.stripe.com/v3/' },
        },
        {
          tagName: 'FOOTER',
          role: 'contentinfo',
          name: '© 2026',
          selector: '',
          interactive: false,
          depth: 0,
          attributes: {},
        },
      ];

      const frameScanResults = new Map<
        number,
        import('../iframe-scanner').FrameScanResult
      >();
      frameScanResults.set(5, {
        elements: [
          {
            tagName: 'INPUT',
            role: 'textbox',
            name: 'Card Number',
            selector: '#card',
            interactive: true,
            depth: 0,
            attributes: {},
          },
        ],
        metadata: {
          url: 'https://js.stripe.com/v3/',
          title: '',
          viewportWidth: 400,
          viewportHeight: 200,
          totalElements: 1,
        },
      });

      const frames: import('../browser-tools-types').FrameInfo[] = [
        {
          frameId: 0,
          parentFrameId: -1,
          url: 'https://example.com',
          documentId: 'doc-main',
          documentLifecycle: 'active',
          frameType: 'outermost_frame',
          errorOccurred: false,
        },
        {
          frameId: 5,
          parentFrameId: 0,
          url: 'https://js.stripe.com/v3/',
          documentId: 'doc-stripe',
          documentLifecycle: 'active',
          frameType: 'sub_frame',
          errorOccurred: false,
        },
      ];

      const { mergedElements } = mergeFrameResults(
        mainElements,
        [frames[1]],
        frameScanResults,
        [],
        [],
        frames
      );

      // Find positions
      const names = mergedElements.map(e => e.name || e.tagName);
      const headingIdx = names.findIndex(n => n === 'Checkout');
      const cardIdx = names.findIndex(n => n === 'Card Number');
      const footerIdx = names.findIndex(n => n === '© 2026');

      // Iframe content must appear BETWEEN heading and footer
      expect(headingIdx).toBeLessThan(cardIdx);
      expect(cardIdx).toBeLessThan(footerIdx);
    });
  });

  describe('Scenario: manifest.json includes webNavigation permission', () => {
    it('should have webNavigation in the permissions array', () => {
      // @step Given the extension manifest.json
      const manifestContent = readFileSync(
        join(EXTENSION_ROOT, 'manifest.json'),
        'utf-8'
      );
      const manifest = JSON.parse(manifestContent) as { permissions: string[] };

      // @step Then the permissions array should include "webNavigation"
      expect(manifest.permissions).toContain('webNavigation');

      // @step And this permission should enable chrome.webNavigation.getAllFrames for frame discovery
      // (Verified by the permission being present — Chrome APIs are enabled by manifest permissions)
    });
  });
});
