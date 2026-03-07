/**
 * Feature: spec/features/page-diff.feature
 *
 * This test file validates all acceptance criteria for LOCATE-006:
 * Page Diff Tool — browser_diff_page.
 *
 * All 8 scenarios test through the browser_diff_page handler integration
 * path (scan DOM → diff → format output → state update) to match the
 * feature file contract.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { createBrowserTools } from '../browser-tools';
import type {
  BrowserToolsDeps,
  ChromeTabsForTools,
  ChromeScriptingForTools,
  ChromeWindowsForTools,
  ChromeUserScriptsForTools,
} from '../browser-tools';
import { getTabScanState, _resetForTesting } from '../ref-state';
import { scanPageDOM } from '../scan-page-dom';

/* ── Mock factories ───────────────────────────────────────────── */

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

function createDeps(opts?: {
  tabs?: Partial<ChromeTabsForTools>;
}): BrowserToolsDeps {
  const executeScript = vi
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

  const scripting: ChromeScriptingForTools = { executeScript };
  const windows: ChromeWindowsForTools = {
    update: vi.fn().mockResolvedValue(undefined),
  };
  const userScripts: ChromeUserScriptsForTools = {
    configureWorld: vi.fn().mockResolvedValue(undefined),
    execute: vi.fn().mockResolvedValue([{ result: null }]),
  };

  return {
    tabs: createMockTabs(opts?.tabs),
    scripting,
    windows,
    userScripts,
  };
}

/* ── DOM helpers ──────────────────────────────────────────────── */

function clearBody(): void {
  document.body.innerHTML = '';
}

function buildLoginPage(): void {
  clearBody();
  const h1 = document.createElement('h1');
  h1.textContent = 'Sign In';
  document.body.appendChild(h1);

  const form = document.createElement('form');

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
}

/* ── Tests ────────────────────────────────────────────────────── */

describe('Feature: Page Diff Tool — browser_diff_page', () => {
  beforeEach(() => {
    _resetForTesting();
    clearBody();
  });

  afterEach(() => {
    clearBody();
  });

  describe('Scenario: Detect single element change after interaction', () => {
    it('should detect button text change, show diff output, and update scan state', async () => {
      // @step Given a page with a heading "Sign In", an email input, a password input, and a submit button
      buildLoginPage();
      const deps = createDeps();
      const tools = createBrowserTools(deps);

      // @step And I have previously scanned the page with browser_scan_page
      const scanHandler = tools.getHandler('browser_scan_page');
      expect(scanHandler).toBeDefined();
      await scanHandler!({});
      const prevState = getTabScanState(1);
      expect(prevState).toBeDefined();
      expect(prevState!.treeText).toContain('button "Sign In"');

      // @step When the button text changes from "Sign In" to "Signing in..." with disabled attribute added
      const btn = document.getElementById('submit-btn')!;
      btn.textContent = 'Signing in...';
      btn.setAttribute('disabled', '');

      // @step And I call browser_diff_page
      const diffHandler = tools.getHandler('browser_diff_page');
      expect(diffHandler).toBeDefined();
      const result = await diffHandler!({});
      const text =
        result.content[0].type === 'text' ? result.content[0].text : '';

      // @step Then the diff output should show the old button line prefixed with "- "
      expect(text).toContain('- ');

      // @step And the diff output should show the new button line prefixed with "+ "
      expect(text).toContain('+ ');

      // @step And the diff stats should show 1 addition and 1 removal
      expect(text).toMatch(/1 addition/);
      expect(text).toMatch(/1 removal/);

      // @step And the changed flag should be true
      // (verified by diff output containing changes)

      // @step And the scan state should be updated with the new tree text
      const newState = getTabScanState(1);
      expect(newState).toBeDefined();
      expect(newState!.treeText).toContain('Signing in...');
      expect(newState!.timestamp).toBeGreaterThanOrEqual(prevState!.timestamp);
    });
  });

  describe('Scenario: No changes detected between identical scans', () => {
    it('should report zero changes when page is unchanged', async () => {
      // @step Given a page with a heading and two interactive elements
      buildLoginPage();
      const deps = createDeps();
      const tools = createBrowserTools(deps);

      // @step And I have previously scanned the page with browser_scan_page
      await tools.getHandler('browser_scan_page')!({});

      // @step When the page content has not changed
      // (no DOM modifications)

      // @step And I call browser_diff_page
      const result = await tools.getHandler('browser_diff_page')!({});
      const text =
        result.content[0].type === 'text' ? result.content[0].text : '';

      // @step Then the diff stats should show 0 additions and 0 removals
      expect(text).toContain('0 addition');
      expect(text).toContain('0 removal');

      // @step And the changed flag should be false
      expect(text).toContain('unchanged');
    });
  });

  describe('Scenario: New elements added to the page', () => {
    it('should show new elements as additions', async () => {
      // @step Given a page with a form containing an email input
      clearBody();
      const input = document.createElement('input');
      input.type = 'email';
      input.id = 'email';
      input.setAttribute('aria-label', 'Email');
      document.body.appendChild(input);

      const deps = createDeps();
      const tools = createBrowserTools(deps);

      // @step And I have previously scanned the page with browser_scan_page
      await tools.getHandler('browser_scan_page')!({});

      // @step When new validation error elements appear on the page
      // Use a heading + button which appear in the scan tree
      // (the scanner includes headings as structural context and buttons as interactive)
      const errorHeading = document.createElement('h2');
      errorHeading.textContent = 'Validation Errors';
      document.body.appendChild(errorHeading);

      const dismissBtn = document.createElement('button');
      dismissBtn.textContent = 'Dismiss';
      document.body.appendChild(dismissBtn);

      // @step And I call browser_diff_page
      const result = await tools.getHandler('browser_diff_page')!({});
      const text =
        result.content[0].type === 'text' ? result.content[0].text : '';

      // @step Then the diff output should show the new elements prefixed with "+ "
      expect(text).toContain('+ ');
      expect(text).toContain('Validation Errors');

      // @step And the diff stats should show additions greater than 0 and 0 removals
      expect(text).not.toMatch(/0 addition/);
      expect(text).toContain('0 removals');
    });
  });

  describe('Scenario: Elements removed from the page', () => {
    it('should show removed elements as removals', async () => {
      // @step Given a page with a modal dialog containing interactive elements
      clearBody();
      const heading = document.createElement('h1');
      heading.textContent = 'Page';
      document.body.appendChild(heading);

      const dialog = document.createElement('div');
      dialog.setAttribute('role', 'dialog');
      dialog.setAttribute('aria-label', 'Confirm');
      const btn = document.createElement('button');
      btn.textContent = 'OK';
      dialog.appendChild(btn);
      document.body.appendChild(dialog);

      const deps = createDeps();
      const tools = createBrowserTools(deps);

      // @step And I have previously scanned the page with browser_scan_page
      await tools.getHandler('browser_scan_page')!({});

      // @step When the modal dialog is removed from the page
      dialog.remove();

      // @step And I call browser_diff_page
      const result = await tools.getHandler('browser_diff_page')!({});
      const text =
        result.content[0].type === 'text' ? result.content[0].text : '';

      // @step Then the diff output should show the removed elements prefixed with "- "
      expect(text).toContain('- ');

      // @step And the diff stats should show 0 additions and removals greater than 0
      expect(text).toContain('0 additions');
      expect(text).toMatch(/removal/);
    });
  });

  describe('Scenario: First diff without previous scan', () => {
    it('should show all current lines as additions with explanatory note', async () => {
      // @step Given a page with interactive elements
      buildLoginPage();
      const deps = createDeps();
      const tools = createBrowserTools(deps);

      // @step And no previous browser_scan_page has been called for this tab
      expect(getTabScanState(1)).toBeUndefined();

      // @step When I call browser_diff_page
      const diffHandler = tools.getHandler('browser_diff_page');
      expect(diffHandler).toBeDefined();
      const result = await diffHandler!({});
      const text =
        result.content[0].type === 'text' ? result.content[0].text : '';

      // @step Then the output should contain "No previous scan to compare against"
      expect(text.toLowerCase()).toContain('no previous scan');

      // @step And all current tree lines should be shown as additions
      expect(text).toContain('+ ');

      // @step And the scan state should be stored for future diffs
      const state = getTabScanState(1);
      expect(state).toBeDefined();
      expect(state!.treeText.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Complete page change after navigation', () => {
    it('should show all old lines as removals and all new lines as additions', async () => {
      // @step Given a page with a login form
      clearBody();
      const h1 = document.createElement('h1');
      h1.textContent = 'Login';
      document.body.appendChild(h1);
      const emailInput = document.createElement('input');
      emailInput.type = 'email';
      emailInput.setAttribute('aria-label', 'Email');
      document.body.appendChild(emailInput);
      const signInBtn = document.createElement('button');
      signInBtn.textContent = 'Sign In';
      document.body.appendChild(signInBtn);

      const deps = createDeps();
      const tools = createBrowserTools(deps);

      // @step And I have previously scanned the page with browser_scan_page
      await tools.getHandler('browser_scan_page')!({});

      // @step When the page content changes completely to a different page
      clearBody();
      const dashH1 = document.createElement('h1');
      dashH1.textContent = 'Dashboard';
      document.body.appendChild(dashH1);
      const profileLink = document.createElement('a');
      profileLink.href = '/profile';
      profileLink.textContent = 'Profile';
      document.body.appendChild(profileLink);
      const logoutBtn = document.createElement('button');
      logoutBtn.textContent = 'Logout';
      document.body.appendChild(logoutBtn);

      // @step And I call browser_diff_page
      const result = await tools.getHandler('browser_diff_page')!({});
      const text =
        result.content[0].type === 'text' ? result.content[0].text : '';

      // @step Then the diff should show all old lines as removals and all new lines as additions
      expect(text).toContain('- ');
      expect(text).toContain('+ ');
      // Old page elements removed
      expect(text).toContain('Login');
      expect(text).toContain('Sign In');
      // New page elements added
      expect(text).toContain('Dashboard');
      expect(text).toContain('Logout');
      expect(text).toMatch(/\d+ addition/);
      expect(text).toMatch(/\d+ removal/);

      // @step And the changed flag should be true
      // (verified by presence of additions and removals — no "No changes detected" message)
      expect(text).not.toContain('No changes detected');
    });
  });

  describe('Scenario: Empty page produces no diff changes', () => {
    it('should handle empty trees gracefully', async () => {
      // @step Given a page with no visible elements
      clearBody();
      const deps = createDeps();
      const tools = createBrowserTools(deps);

      // @step And I have previously scanned the page with browser_scan_page producing an empty tree
      await tools.getHandler('browser_scan_page')!({});

      // @step When I call browser_diff_page
      const result = await tools.getHandler('browser_diff_page')!({});
      const text =
        result.content[0].type === 'text' ? result.content[0].text : '';

      // @step Then the diff stats should show 0 additions and 0 removals
      expect(text).toContain('0 addition');
      expect(text).toContain('0 removal');

      // @step And the changed flag should be false
      expect(text).toContain('unchanged');
    });
  });

  describe('Scenario: Context lines included around changes for readability', () => {
    it('should include context lines and separate distant changes with ellipsis', async () => {
      // @step Given a page with many elements where only one element in the middle changed
      clearBody();
      // Build a page with many elements — headings, nav links, main content, footer
      const h1 = document.createElement('h1');
      h1.textContent = 'Page';
      document.body.appendChild(h1);

      const nav = document.createElement('nav');
      nav.setAttribute('aria-label', 'Main');
      const homeLink = document.createElement('a');
      homeLink.href = '/home';
      homeLink.textContent = 'Home';
      nav.appendChild(homeLink);
      const aboutLink = document.createElement('a');
      aboutLink.href = '/about';
      aboutLink.textContent = 'About';
      nav.appendChild(aboutLink);
      const contactLink = document.createElement('a');
      contactLink.href = '/contact';
      contactLink.textContent = 'Contact';
      nav.appendChild(contactLink);
      document.body.appendChild(nav);

      const main = document.createElement('main');
      const h2 = document.createElement('h2');
      h2.textContent = 'Content';
      main.appendChild(h2);
      const actionBtn = document.createElement('button');
      actionBtn.id = 'action-btn';
      actionBtn.textContent = 'Old Action';
      main.appendChild(actionBtn);
      const searchInput = document.createElement('input');
      searchInput.type = 'search';
      searchInput.setAttribute('aria-label', 'Search');
      main.appendChild(searchInput);
      document.body.appendChild(main);

      const footer = document.createElement('footer');
      const privacyLink = document.createElement('a');
      privacyLink.href = '/privacy';
      privacyLink.textContent = 'Privacy';
      footer.appendChild(privacyLink);
      document.body.appendChild(footer);

      const deps = createDeps();
      const tools = createBrowserTools(deps);

      // @step And I have previously scanned the page with browser_scan_page
      await tools.getHandler('browser_scan_page')!({});

      // Change only the middle button
      const btn = document.getElementById('action-btn')!;
      btn.textContent = 'New Action';

      // @step When I call browser_diff_page
      const result = await tools.getHandler('browser_diff_page')!({});
      const text =
        result.content[0].type === 'text' ? result.content[0].text : '';

      // @step Then unchanged lines adjacent to changes should be included for context
      // The button change is in the middle — context should include surrounding lines
      expect(text).toContain('Old Action');
      expect(text).toContain('New Action');

      // @step And non-adjacent unchanged lines should be omitted with "..." separator
      expect(text).toContain('...');
      // The heading "Page" and footer link "Privacy" are far from the change — should be omitted
      expect(text).not.toMatch(/^\s*-?\s*heading "Page"/m);
    });
  });
});
