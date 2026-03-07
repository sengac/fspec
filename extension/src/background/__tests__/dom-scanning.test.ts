/**
 * Feature: spec/features/dom-scanning.feature
 *
 * This test file validates all acceptance criteria for LOCATE-004:
 * DOM Scanning Core — browser_scan_page Tool.
 *
 * Tests the scanPageDOM injected function (with jsdom), the pure
 * helper functions, and the browser_scan_page handler integration
 * (ref assignment, ref state storage, error handling).
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  isDynamicId,
  getAccessibleName,
  generateSelector,
  isInteractiveElement,
  shouldClaimChildren,
  getRelevantAttributes,
  formatAccessibilityTree,
} from '../dom-scanner';
import type { RawElement } from '../dom-scanner';
import { scanPageDOM } from '../scan-page-dom';
import { createBrowserTools } from '../browser-tools';
import type {
  BrowserToolsDeps,
  ChromeTabsForTools,
  ChromeScriptingForTools,
  ChromeWindowsForTools,
  ChromeUserScriptsForTools,
} from '../browser-tools';
import { getTabScanState, resolveRef, _resetForTesting } from '../ref-state';

/* ── Mock factories ───────────────────────────────────────────── */

function createMockTabs(ov?: Partial<ChromeTabsForTools>): ChromeTabsForTools {
  return {
    query: vi
      .fn()
      .mockResolvedValue([
        { id: 1, url: 'https://example.com/login', title: 'Login' },
      ]),
    update: vi.fn().mockResolvedValue({ id: 1 }),
    remove: vi.fn().mockResolvedValue(undefined),
    captureVisibleTab: vi.fn().mockResolvedValue('data:image/png;base64,abc'),
    goBack: vi.fn().mockResolvedValue(undefined),
    goForward: vi.fn().mockResolvedValue(undefined),
    get: vi.fn().mockResolvedValue({
      id: 1,
      windowId: 1,
      url: 'https://example.com/login',
      title: 'Login',
    }),
    onUpdated: { addListener: vi.fn(), removeListener: vi.fn() },
    create: vi.fn().mockResolvedValue({ id: 2 }),
    ...ov,
  };
}

/**
 * Build a mock scanResult that simulates what scanPageDOM would return.
 * The mock executeScript calls scanPageDOM in the test's jsdom, which
 * IS the real scanning code, just running in jsdom instead of Chrome.
 */
function createDeps(opts?: {
  tabs?: Partial<ChromeTabsForTools>;
  scanResult?: unknown;
  useLiveScan?: boolean;
}): BrowserToolsDeps {
  let executeScript: ChromeScriptingForTools['executeScript'];

  if (opts?.useLiveScan) {
    // Run the REAL scanPageDOM against the test's jsdom document
    executeScript = vi
      .fn()
      .mockImplementation(
        (injection: { args: [boolean, string | undefined] }) => {
          const [interactive, scope] = injection.args;
          const result = scanPageDOM(interactive, scope);
          return Promise.resolve([{ result }]);
        }
      );
  } else if (opts?.scanResult !== undefined) {
    executeScript = vi.fn().mockResolvedValue([{ result: opts.scanResult }]);
  } else {
    // Default: run real scan
    executeScript = vi
      .fn()
      .mockImplementation((injection: { args?: unknown[] }) => {
        if (injection.args) {
          const [interactive, scope] = injection.args as [
            boolean,
            string | undefined,
          ];
          const result = scanPageDOM(interactive, scope);
          return Promise.resolve([{ result }]);
        }
        return Promise.resolve([{ result: null }]);
      });
  }

  return {
    tabs: createMockTabs(opts?.tabs),
    scripting: { executeScript } as unknown as ChromeScriptingForTools,
    windows: {
      update: vi.fn().mockResolvedValue(undefined),
    } as unknown as ChromeWindowsForTools,
    userScripts: {
      configureWorld: vi.fn().mockResolvedValue(undefined),
      execute: vi.fn().mockResolvedValue([{ result: null }]),
    } as unknown as ChromeUserScriptsForTools,
  };
}

/* ── DOM helpers for building test pages ──────────────────────── */

function clearBody(): void {
  document.body.innerHTML = '';
}

function buildLoginPage(): void {
  clearBody();
  const h1 = document.createElement('h1');
  h1.textContent = 'Sign In';
  document.body.appendChild(h1);

  const form = document.createElement('form');
  form.className = 'login';

  const emailInput = document.createElement('input');
  emailInput.type = 'email';
  emailInput.id = 'email';
  emailInput.setAttribute('aria-label', 'Email');
  form.appendChild(emailInput);

  const pwInput = document.createElement('input');
  pwInput.type = 'password';
  pwInput.id = 'password';
  pwInput.setAttribute('aria-label', 'Password');
  form.appendChild(pwInput);

  const submitBtn = document.createElement('button');
  submitBtn.type = 'submit';
  submitBtn.id = 'submit-btn';
  submitBtn.textContent = 'Sign In';
  form.appendChild(submitBtn);

  document.body.appendChild(form);

  const forgotLink = document.createElement('a');
  forgotLink.href = '/forgot';
  forgotLink.textContent = 'Forgot Password';
  document.body.appendChild(forgotLink);
}

/* ── Tests ────────────────────────────────────────────────────── */

describe('Feature: DOM Scanning Core — browser_scan_page Tool', () => {
  beforeEach(() => {
    _resetForTesting();
    clearBody();
  });

  afterEach(() => {
    clearBody();
  });

  describe('Scenario: Scan a login page and receive accessibility tree with refs', () => {
    it('should return tree with heading context and 4 interactive refs', async () => {
      // @step Given a page with a heading "Sign In", an email input, a password input, a submit button, and a "Forgot Password" link
      buildLoginPage();
      const deps = createDeps({ useLiveScan: true });
      const tools = createBrowserTools(deps);
      const handler = tools.getHandler('browser_scan_page');
      expect(handler).toBeDefined();

      // @step When I call browser_scan_page with default parameters
      const result = await handler!({});
      const text =
        result.content[0].type === 'text' ? result.content[0].text : '';

      // @step Then the result should contain a heading "Sign In" for structural context
      expect(text).toContain('heading "Sign In"');

      // @step And the result should contain 4 interactive elements with refs e1 through e4
      expect(text).toContain('[ref=e1]');
      expect(text).toContain('[ref=e2]');
      expect(text).toContain('[ref=e3]');
      expect(text).toContain('[ref=e4]');

      // @step And each interactive element should have a role, name, and ref annotation
      expect(text).toContain('textbox "Email"');
      expect(text).toContain('textbox "Password"');
      expect(text).toContain('button "Sign In"');
      expect(text).toContain('link "Forgot Password"');

      // @step And the metadata should include url, title, viewport dimensions, and interactive element count
      expect(text).toContain('4 interactive elements');
    });
  });

  describe('Scenario: Hidden elements are filtered from scan results', () => {
    it('should exclude hidden elements from scan', () => {
      // @step Given a page with a hidden div containing a button and a visible button
      clearBody();
      const hidden = document.createElement('div');
      hidden.style.display = 'none';
      const hiddenBtn = document.createElement('button');
      hiddenBtn.textContent = 'Hidden';
      hidden.appendChild(hiddenBtn);
      document.body.appendChild(hidden);

      const visibleBtn = document.createElement('button');
      visibleBtn.textContent = 'Visible';
      document.body.appendChild(visibleBtn);

      // @step When I call browser_scan_page
      const result = scanPageDOM(true);

      // @step Then only the visible button should appear in the tree with a ref
      const interactiveEls = result.elements.filter(e => e.interactive);
      expect(interactiveEls.length).toBe(1);
      expect(interactiveEls[0].name).toBe('Visible');

      // @step And the hidden button should not appear in the results
      const hiddenEls = result.elements.filter(e => e.name === 'Hidden');
      expect(hiddenEls.length).toBe(0);
    });
  });

  describe('Scenario: Dynamic IDs are excluded from selector generation', () => {
    it('should detect dynamic vs stable IDs', () => {
      // @step Given a page with a button having id "btn-abc123def" and another with id "submit-btn"
      // @step When I call browser_scan_page
      // @step Then the button with dynamic ID should use a fallback selector instead of the ID
      expect(isDynamicId('btn-abc123def')).toBe(true);
      expect(isDynamicId('a1b2c3d4e5')).toBe(true);
      expect(isDynamicId('r-1a2b3c')).toBe(true);
      expect(isDynamicId('ember1234')).toBe(true);
      expect(isDynamicId('react-abc123')).toBe(true);

      // @step And the button with stable ID "submit-btn" should use selector "#submit-btn"
      expect(isDynamicId('submit-btn')).toBe(false);
      expect(isDynamicId('login-form')).toBe(false);
    });

    it('should use stable IDs in selectors and skip dynamic ones via scanPageDOM', () => {
      clearBody();
      const btn1 = document.createElement('button');
      btn1.id = 'btn-abc123def';
      btn1.textContent = 'Dynamic';
      document.body.appendChild(btn1);

      const btn2 = document.createElement('button');
      btn2.id = 'submit-btn';
      btn2.textContent = 'Stable';
      document.body.appendChild(btn2);

      const result = scanPageDOM(true);
      const dynamic = result.elements.find(e => e.name === 'Dynamic');
      const stable = result.elements.find(e => e.name === 'Stable');

      expect(dynamic).toBeDefined();
      expect(dynamic!.selector).not.toBe('#btn-abc123def');

      expect(stable).toBeDefined();
      expect(stable!.selector).toBe('#submit-btn');
    });
  });

  describe('Scenario: Non-interactive scan mode returns all elements without refs', () => {
    it('should return all visible elements with no interactive flags', () => {
      // @step Given a page with interactive and non-interactive elements
      clearBody();
      const p = document.createElement('p');
      p.textContent = 'Hello world';
      document.body.appendChild(p);
      const btn = document.createElement('button');
      btn.textContent = 'Click me';
      document.body.appendChild(btn);

      // @step When I call browser_scan_page with interactive set to false
      const result = scanPageDOM(false);

      // @step Then the result should include all visible elements including paragraphs and divs
      expect(result.elements.length).toBeGreaterThanOrEqual(2);
      const pEl = result.elements.find(e => e.name === 'Hello world');
      expect(pEl).toBeDefined();

      // @step And no elements should have ref annotations
      const interactiveCount = result.elements.filter(
        e => e.interactive
      ).length;
      expect(interactiveCount).toBe(0);
    });
  });

  describe('Scenario: Scoped scan via CSS selector parameter', () => {
    it('should only scan within the scoped selector', async () => {
      // @step Given a page with a login form and a navigation bar both containing buttons
      clearBody();
      const nav = document.createElement('nav');
      const navBtn = document.createElement('button');
      navBtn.textContent = 'Nav Button';
      nav.appendChild(navBtn);
      document.body.appendChild(nav);

      const form = document.createElement('form');
      form.className = 'login';
      const formBtn = document.createElement('button');
      formBtn.textContent = 'Login';
      form.appendChild(formBtn);
      document.body.appendChild(form);

      // @step When I call browser_scan_page with selector "form.login"
      const deps = createDeps({ useLiveScan: true });
      const handler = createBrowserTools(deps).getHandler('browser_scan_page');
      const result = await handler!({ selector: 'form.login' });
      const text =
        result.content[0].type === 'text' ? result.content[0].text : '';

      // @step Then only elements within the login form should appear in the results
      expect(text).toContain('button "Login"');

      // @step And navigation bar elements should not be included
      expect(text).not.toContain('Nav Button');
    });
  });

  describe('Scenario: Aria-label takes priority in accessible name extraction', () => {
    it('should use aria-label over placeholder', () => {
      // @step Given a page with an input having aria-label "Search products" and placeholder "Type here"
      const el = document.createElement('input');
      el.setAttribute('aria-label', 'Search products');
      el.setAttribute('placeholder', 'Type here');
      document.body.appendChild(el);

      // @step When I call browser_scan_page
      // @step Then the input should appear as textbox "Search products" using aria-label over placeholder
      expect(getAccessibleName(el)).toBe('Search products');

      // Also verify via scanPageDOM
      const result = scanPageDOM(true);
      const inputEl = result.elements.find(e => e.role === 'textbox');
      expect(inputEl).toBeDefined();
      expect(inputEl!.name).toBe('Search products');
    });

    it('should fall back through priority chain and truncate at 80 chars', () => {
      const input = document.createElement('input');
      input.setAttribute('placeholder', 'Enter email');
      expect(getAccessibleName(input)).toBe('Enter email');

      const btn = document.createElement('button');
      btn.textContent = 'A'.repeat(100);
      const name = getAccessibleName(btn);
      expect(name.length).toBeLessThanOrEqual(83);
      expect(name).toContain('...');
    });
  });

  describe('Scenario: Interactive parent claims contained children', () => {
    it('should only include the parent link, not its children', () => {
      // @step Given a page with a link containing a span and an SVG icon
      clearBody();
      const link = document.createElement('a');
      link.href = '/home';
      const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
      link.appendChild(svg);
      const span = document.createElement('span');
      span.textContent = 'Home';
      link.appendChild(span);
      document.body.appendChild(link);

      // @step When I call browser_scan_page
      const result = scanPageDOM(true);

      // @step Then only the parent link should receive a ref
      const linkEl = result.elements.find(e => e.role === 'link');
      expect(linkEl).toBeDefined();
      expect(linkEl!.interactive).toBe(true);
      expect(linkEl!.name).toBe('Home');

      // @step And the contained span and SVG should not appear as separate interactive elements
      expect(shouldClaimChildren(link)).toBe(true);
      expect(shouldClaimChildren(span)).toBe(false);
      // Children should not be in the elements list
      const spanEls = result.elements.filter(e => e.tagName === 'SPAN');
      expect(spanEls.length).toBe(0);
    });
  });

  describe('Scenario: Aria-disabled and aria-hidden elements are excluded', () => {
    it('should exclude aria-disabled and aria-hidden elements', () => {
      // @step Given a page with a button having aria-disabled "true" and a div with aria-hidden "true" containing a link
      clearBody();
      const btn = document.createElement('button');
      btn.setAttribute('aria-disabled', 'true');
      btn.textContent = 'Disabled Button';
      document.body.appendChild(btn);

      const div = document.createElement('div');
      div.setAttribute('aria-hidden', 'true');
      const link = document.createElement('a');
      link.href = '/x';
      link.textContent = 'Hidden Link';
      div.appendChild(link);
      document.body.appendChild(div);

      // @step When I call browser_scan_page
      // @step Then neither the disabled button nor the hidden link should have refs in the result
      expect(isInteractiveElement(btn)).toBe(false);
      expect(isInteractiveElement(link)).toBe(false);

      // Also verify via scanPageDOM
      const result = scanPageDOM(true);
      const interactiveEls = result.elements.filter(e => e.interactive);
      expect(interactiveEls.length).toBe(0);
    });
  });

  describe('Scenario: Form validation attributes are included in tree output', () => {
    it('should extract validation-relevant attributes', () => {
      // @step Given a page with an email input having required, pattern, and type attributes
      clearBody();
      const input = document.createElement('input');
      input.type = 'email';
      input.required = true;
      input.setAttribute('pattern', '.+@.+');
      input.setAttribute('aria-label', 'Email');
      document.body.appendChild(input);

      // @step When I call browser_scan_page
      const attrs = getRelevantAttributes(input);

      // @step Then the tree output should include the type, required, and pattern attributes for that element
      expect(attrs.type).toBe('email');
      expect(attrs.required).toBeDefined();
      expect(attrs.pattern).toBe('.+@.+');

      // Also verify via scanPageDOM end-to-end
      const result = scanPageDOM(true);
      const inputEl = result.elements.find(e => e.role === 'textbox');
      expect(inputEl).toBeDefined();
      expect(inputEl!.attributes.type).toBe('email');
      expect(inputEl!.attributes.required).toBeDefined();
      expect(inputEl!.attributes.pattern).toBe('.+@.+');
    });
  });

  describe('Scenario: Cursor pointer heuristic detects interactive divs', () => {
    it('should detect cursor:pointer and onclick as interactive', () => {
      // @step Given a page with a plain div styled with cursor pointer via CSS
      clearBody();
      const div = document.createElement('div');
      div.style.cursor = 'pointer';
      div.textContent = 'Clickable Div';
      document.body.appendChild(div);

      // @step When I call browser_scan_page
      // @step Then the div should be detected as interactive and receive a ref
      expect(isInteractiveElement(div)).toBe(true);

      const div2 = document.createElement('div');
      div2.setAttribute('onclick', 'doSomething()');
      div2.textContent = 'OnClick Div';
      document.body.appendChild(div2);
      expect(isInteractiveElement(div2)).toBe(true);
    });
  });

  describe('Scenario: Scan results are stored in ref state for later resolution', () => {
    it('should store refs via setTabScanState after scan', async () => {
      // @step Given a page with interactive elements
      buildLoginPage();
      const deps = createDeps({ useLiveScan: true });
      const handler = createBrowserTools(deps).getHandler('browser_scan_page');

      // @step When I call browser_scan_page
      await handler!({});

      // @step Then the scan state should be stored via setTabScanState
      const state = getTabScanState(1);
      expect(state).toBeDefined();
      expect(state!.refs.size).toBe(4);
      expect(state!.treeText.length).toBeGreaterThan(0);

      // @step And resolveRef with the tab ID and ref "e1" should return the correct RefEntry
      const entry = resolveRef(1, 'e1');
      expect(entry).toBeDefined();
      expect(entry!.role).toBe('textbox');
      expect(entry!.name).toBe('Email');
    });
  });

  describe('Scenario: Error handling for invalid tab ID', () => {
    it('should return MCP error for invalid tab', async () => {
      // @step Given an invalid tab ID that does not exist
      const deps = createDeps();
      (
        deps.scripting.executeScript as ReturnType<typeof vi.fn>
      ).mockRejectedValue(new Error('No tab with id: 9999'));
      const handler = createBrowserTools(deps).getHandler('browser_scan_page');

      // @step When I call browser_scan_page with that tab ID
      const result = await handler!({ tabId: 9999 });

      // @step Then the result should be an MCP error indicating the tab was not found
      expect(result.isError).toBe(true);
      expect(
        result.content[0].type === 'text'
          ? result.content[0].text.toLowerCase()
          : ''
      ).toContain('scan failed');
    });
  });

  describe('Scenario: CSS selector ranking by reliability', () => {
    it('should prefer data-testid > id > attribute combo', () => {
      // @step Given a page with an element having data-testid "email", another with only id "email-input", and a third with no id
      clearBody();
      const el1 = document.createElement('input');
      el1.setAttribute('data-testid', 'email');
      document.body.appendChild(el1);

      const el2 = document.createElement('input');
      el2.id = 'email-input';
      document.body.appendChild(el2);

      const el3 = document.createElement('input');
      el3.type = 'email';
      el3.name = 'email';
      document.body.appendChild(el3);

      // @step When I call browser_scan_page
      // @step Then the first element should use selector with data-testid attribute
      expect(generateSelector(el1)).toBe('[data-testid="email"]');

      // @step And the second element should use selector "#email-input"
      expect(generateSelector(el2)).toBe('#email-input');

      // @step And the third element should use an attribute combination or nth-child selector
      const sel3 = generateSelector(el3);
      expect(sel3).not.toContain('#');
      expect(sel3.length).toBeGreaterThan(0);
    });
  });

  /* ── Additional unit tests for formatAccessibilityTree ────── */

  describe('formatAccessibilityTree', () => {
    it('should format elements into indented tree notation', () => {
      const elements: RawElement[] = [
        {
          tagName: 'H1',
          role: 'heading',
          name: 'Sign In',
          selector: '',
          interactive: false,
          depth: 0,
          attributes: { level: '1' },
          ref: undefined,
        },
        {
          tagName: 'INPUT',
          role: 'textbox',
          name: 'Email',
          selector: '#email',
          interactive: true,
          depth: 1,
          attributes: { type: 'email', required: '' },
          ref: 'e1',
        },
      ];
      const tree = formatAccessibilityTree(elements);
      expect(tree).toContain('heading "Sign In"');
      expect(tree).toContain('[level=1]');
      expect(tree).toContain('textbox "Email" [ref=e1]');
      expect(tree).toContain('[type=email]');
    });
  });

  /* ── Additional unit test for scanPageDOM directly ──────────── */

  describe('scanPageDOM — TreeWalker skips SCRIPT/STYLE/NOSCRIPT', () => {
    it('should not include script or style elements', () => {
      clearBody();
      const script = document.createElement('script');
      script.textContent = 'console.log("ignored")';
      document.body.appendChild(script);

      const style = document.createElement('style');
      style.textContent = 'body { color: red; }';
      document.body.appendChild(style);

      const noscript = document.createElement('noscript');
      noscript.textContent = 'Enable JS';
      document.body.appendChild(noscript);

      const btn = document.createElement('button');
      btn.textContent = 'Real Button';
      document.body.appendChild(btn);

      const result = scanPageDOM(true);
      const scriptEls = result.elements.filter(
        e =>
          e.tagName === 'SCRIPT' ||
          e.tagName === 'STYLE' ||
          e.tagName === 'NOSCRIPT'
      );
      expect(scriptEls.length).toBe(0);
      expect(result.elements.some(e => e.name === 'Real Button')).toBe(true);
    });
  });

  describe('scanPageDOM — IFRAME elements appear in scan results', () => {
    it('should emit an element with tagName IFRAME and src attribute', () => {
      clearBody();
      const h1 = document.createElement('h1');
      h1.textContent = 'Checkout';
      document.body.appendChild(h1);

      const iframe = document.createElement('iframe');
      iframe.src = 'https://js.stripe.com/v3/';
      document.body.appendChild(iframe);

      const footer = document.createElement('footer');
      footer.textContent = '© 2026';
      document.body.appendChild(footer);

      const result = scanPageDOM(true);

      const iframeEl = result.elements.find(e => e.tagName === 'IFRAME');
      expect(iframeEl).toBeDefined();
      expect(iframeEl!.role).toBe('iframe');
      expect(iframeEl!.attributes.src).toBe('https://js.stripe.com/v3/');
      expect(iframeEl!.interactive).toBe(false);

      // IFRAME must appear between heading and footer in document order
      const tags = result.elements.map(e => e.tagName);
      const h1Idx = tags.indexOf('H1');
      const iframeIdx = tags.indexOf('IFRAME');
      const footerIdx = tags.indexOf('FOOTER');
      expect(h1Idx).toBeLessThan(iframeIdx);
      expect(iframeIdx).toBeLessThan(footerIdx);
    });
  });
});
