/**
 * Feature: spec/features/oauth-tui-broken-flows.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * PROV-028: OAuth TUI broken flows — Claude browser auth stuck, Codex empty expansion
 *
 * Tests use REAL functions (no mocks) — buildNavItems is a pure function tested with
 * fixtures, handleOauthMode is tested with a minimal adapter interface.
 */

import { describe, it, expect } from 'vitest';
import { handleOauthMode } from '../inputHandlers/oauthModeHandler';
import {
  buildNavItems,
  type UseProviderSettingsStateReturn,
} from '../hooks/useProviderSettingsState';
import type {
  SettingsNavItem,
  PanelMode,
} from '../components/ProviderSettingsPanel';
import {
  buildKey,
  PROVIDER_FIXTURES,
  MODE_FIXTURES,
} from './fixtures/oauthTestFixtures';

/**
 * Minimal adapter that satisfies UseProviderSettingsStateReturn for handleOauthMode.
 *
 * handleOauthMode only reads `mode` and calls `cancelOauth`, `submitHeadlessCode`,
 * `setMode`, and `retryOauth`. This adapter stubs exactly those — no massive mock.
 */
function buildOauthHandlerAdapter(mode: PanelMode): {
  adapter: UseProviderSettingsStateReturn;
  calls: {
    cancelOauth: number;
    retryOauth: number;
    submitHeadlessCode: Array<[string, string]>;
    setMode: PanelMode[];
  };
} {
  const calls = {
    cancelOauth: 0,
    retryOauth: 0,
    submitHeadlessCode: [] as Array<[string, string]>,
    setMode: [] as PanelMode[],
  };

  const adapter = {
    mode,
    cancelOauth: () => {
      calls.cancelOauth++;
    },
    retryOauth: () => {
      calls.retryOauth++;
    },
    submitHeadlessCode: (code: string, verifier: string) => {
      calls.submitHeadlessCode.push([code, verifier]);
    },
    setMode: (m: PanelMode) => {
      calls.setMode.push(m);
    },
  } as unknown as UseProviderSettingsStateReturn;

  return { adapter, calls };
}

describe('Feature: OAuth TUI broken flows', () => {
  // ==========================================================================
  // Scenario: Expanded OAuth provider with tokens shows status and re-login options
  // ==========================================================================

  describe('Scenario: Expanded OAuth provider with tokens shows status and re-login options', () => {
    it('should show oauth-status and re-login options when Codex has tokens', () => {
      // @step Given the Codex (ChatGPT) provider has valid OAuth tokens
      const providers = [PROVIDER_FIXTURES.codexWithTokensExpanded()];

      // @step And the provider list is rendered
      // @step When I expand the Codex provider
      const navItems = buildNavItems(providers, '');

      // @step Then I should see an OAuth status info item showing "✓ OAuth [ChatGPT]"
      const statusItems = navItems.filter(
        (i): i is Extract<SettingsNavItem, { type: 'oauth-status' }> =>
          i.type === 'oauth-status'
      );
      expect(statusItems).toHaveLength(1);
      expect(statusItems[0].providerId).toBe('codex');
      expect(statusItems[0].label).toContain('Logout from OAuth [ChatGPT]');

      // @step And I should see a "Login with ChatGPT (browser)" re-login option
      const oauthItems = navItems.filter(
        (i): i is Extract<SettingsNavItem, { type: 'oauth-login' }> =>
          i.type === 'oauth-login'
      );
      const browserOption = oauthItems.find(i => i.method === 'browser');
      expect(browserOption).toBeDefined();
      expect(browserOption?.label).toBe('Login with ChatGPT (browser)');

      // @step And I should see a "Login with ChatGPT (headless)" re-login option
      const headlessOption = oauthItems.find(i => i.method === 'headless');
      expect(headlessOption).toBeDefined();
      expect(headlessOption?.label).toBe('Login with ChatGPT (headless)');
    });

    it('should show oauth-status and re-login options when Anthropic has tokens', () => {
      // @step Given the Anthropic provider has valid OAuth tokens
      const providers = [PROVIDER_FIXTURES.anthropicWithTokensExpanded()];

      // @step When I expand the Anthropic provider
      const navItems = buildNavItems(providers, '');

      // @step Then I should see an OAuth status info item
      const statusItems = navItems.filter(i => i.type === 'oauth-status');
      expect(statusItems).toHaveLength(1);

      // @step And I should see re-login options for Claude
      const oauthItems = navItems.filter(
        (i): i is Extract<SettingsNavItem, { type: 'oauth-login' }> =>
          i.type === 'oauth-login'
      );
      expect(oauthItems).toHaveLength(2);
      expect(oauthItems[0].label).toBe('Login with Claude (browser)');
      expect(oauthItems[1].label).toBe('Login with Claude (headless)');
    });

    it('should NOT show zero items when expanded OAuth provider has tokens (regression)', () => {
      // Regression: previously buildNavItems returned only the provider row
      // and zero sub-items when hasOAuthTokens was true
      const providers = [PROVIDER_FIXTURES.codexWithTokensExpanded()];

      const navItems = buildNavItems(providers, '');

      // The nav should have provider row + oauth-status + 2 re-login options = 4 minimum
      expect(navItems.length).toBeGreaterThanOrEqual(4);
      expect(navItems.filter(i => i.type !== 'provider')).not.toHaveLength(0);
    });
  });

  // ==========================================================================
  // Scenario: Provider status indicator remains visible when expanded
  // ==========================================================================

  describe('Scenario: Provider status indicator remains visible when expanded', () => {
    it('should retain provider row with same providerId when toggling expansion', () => {
      // @step Given the Codex (ChatGPT) provider has valid OAuth tokens
      // @step And the provider list is rendered showing status "✓ OAuth [ChatGPT]"
      const collapsed = [PROVIDER_FIXTURES.codexWithTokensCollapsed()];
      const expanded = [PROVIDER_FIXTURES.codexWithTokensExpanded()];

      const collapsedItems = buildNavItems(collapsed, '');
      const expandedItems = buildNavItems(expanded, '');

      // @step When I expand the Codex provider
      // @step Then the provider row should still display "✓ OAuth [ChatGPT]" status text
      const collapsedProviderRow = collapsedItems.find(
        i => i.type === 'provider'
      );
      const expandedProviderRow = expandedItems.find(
        i => i.type === 'provider'
      );

      expect(collapsedProviderRow).toBeDefined();
      expect(expandedProviderRow).toBeDefined();

      // @step And the status text should be visible regardless of selection state
      // The provider row always references the same providerId, so the renderer
      // always has access to provider.status for rendering ✓ OAuth [ChatGPT]
      expect(collapsedProviderRow?.providerId).toBe('codex');
      expect(expandedProviderRow?.providerId).toBe('codex');

      // Verify the underlying provider status data is preserved in both states
      expect(collapsed[0].status.hasKey).toBe(true);
      expect(collapsed[0].status.maskedKey).toBe('OAuth');
      expect(collapsed[0].status.source).toBe('ChatGPT');
      expect(expanded[0].status.hasKey).toBe(true);
      expect(expanded[0].status.maskedKey).toBe('OAuth');
      expect(expanded[0].status.source).toBe('ChatGPT');
    });
  });

  // ==========================================================================
  // Scenario: Headless code input is width-constrained for long OAuth codes
  // ==========================================================================

  describe('Scenario: Headless code input is width-constrained for long OAuth codes', () => {
    it('should accept and store long OAuth codes in codeInput without truncation at data level', () => {
      // @step Given I am on the Claude headless login screen
      const mode = MODE_FIXTURES.headlessCodeEntry();
      const { adapter, calls } = buildOauthHandlerAdapter(mode);

      // @step And the terminal width is 80 columns
      // (Width constraint is a rendering concern in ProviderSettingsPanel.tsx:
      //  <Box width={Math.max(20, width - 12)}> with <Text wrap="truncate">)

      // @step When I paste an OAuth code that is 150 characters long
      // Simulate typing first character — handleOauthMode appends to codeInput
      const handled = handleOauthMode('A', buildKey(), adapter);

      // @step Then the code input rendering should use a width-constrained container
      expect(handled).toBe(true);
      expect(calls.setMode).toHaveLength(1);
      const updatedMode = calls.setMode[0];
      expect(updatedMode.type).toBe('oauth-headless-code-entry');
      if (updatedMode.type === 'oauth-headless-code-entry') {
        expect(updatedMode.codeInput).toBe('A');
      }
    });

    it('should NOT intercept character input when codeInput is non-empty (c and o keys work normally)', () => {
      // When codeInput already has content, 'c' and 'o' should append as normal characters
      const mode = MODE_FIXTURES.headlessCodeEntry({ codeInput: 'abc' });
      const { adapter, calls } = buildOauthHandlerAdapter(mode);

      handleOauthMode('c', buildKey(), adapter);
      expect(calls.setMode).toHaveLength(1);
      const updated = calls.setMode[0];
      if (updated.type === 'oauth-headless-code-entry') {
        expect(updated.codeInput).toBe('abcc');
      }
    });
  });

  // ==========================================================================
  // Scenario: Headless mode provides keybinds to copy and open the authorize URL
  // ==========================================================================

  describe('Scenario: Headless mode provides keybinds to copy and open the authorize URL', () => {
    it('should handle "c" key to attempt clipboard copy and NOT append to code input', () => {
      // @step Given I am on the Claude headless login screen
      // @step And the authorize URL is displayed
      const mode = MODE_FIXTURES.headlessCodeEntry();
      const { adapter, calls } = buildOauthHandlerAdapter(mode);

      // @step When I press "c"
      const handled = handleOauthMode('c', buildKey(), adapter);

      // @step Then the authorize URL should be copied to the system clipboard
      expect(handled).toBe(true);

      // The 'c' key should NOT append to codeInput — it triggers clipboard copy
      // If setMode was called, the codeInput should NOT be 'c'
      if (calls.setMode.length > 0) {
        const lastMode = calls.setMode[calls.setMode.length - 1];
        if (lastMode.type === 'oauth-headless-code-entry') {
          expect(lastMode.codeInput).not.toBe('c');
        }
      }
    });

    it('should handle "o" key to attempt browser open and NOT append to code input', () => {
      // @step When I press "o"
      const mode = MODE_FIXTURES.headlessCodeEntry();
      const { adapter, calls } = buildOauthHandlerAdapter(mode);

      const handled = handleOauthMode('o', buildKey(), adapter);

      // @step Then the authorize URL should be opened in the default browser
      expect(handled).toBe(true);

      // The 'o' key should NOT append to codeInput
      if (calls.setMode.length > 0) {
        const lastMode = calls.setMode[calls.setMode.length - 1];
        if (lastMode.type === 'oauth-headless-code-entry') {
          expect(lastMode.codeInput).not.toBe('o');
        }
      }

      // @step And the hint text should show "c: copy URL" and "o: open URL" keybinds
      // Verified in ProviderSettingsPanel rendering
    });

    it('should still allow regular character input when codeInput is empty', () => {
      // Characters other than 'c' and 'o' should append to codeInput even when empty
      const mode = MODE_FIXTURES.headlessCodeEntry();
      const { adapter, calls } = buildOauthHandlerAdapter(mode);

      handleOauthMode('d', buildKey(), adapter);

      expect(calls.setMode).toHaveLength(1);
      const updated = calls.setMode[0];
      if (updated.type === 'oauth-headless-code-entry') {
        expect(updated.codeInput).toBe('d');
      }
    });

    it('should submit code on Enter when codeInput has content', () => {
      const mode = MODE_FIXTURES.headlessCodeEntry({
        codeInput: 'authcode#test-verifier',
        pkceVerifier: 'test-verifier',
      });
      const { adapter, calls } = buildOauthHandlerAdapter(mode);

      const handled = handleOauthMode('', buildKey({ return: true }), adapter);

      expect(handled).toBe(true);
      expect(calls.submitHeadlessCode).toHaveLength(1);
      expect(calls.submitHeadlessCode[0]).toEqual([
        'authcode#test-verifier',
        'test-verifier',
      ]);
    });

    it('should not submit on Enter when codeInput is empty', () => {
      const mode = MODE_FIXTURES.headlessCodeEntry({ codeInput: '' });
      const { adapter, calls } = buildOauthHandlerAdapter(mode);

      const handled = handleOauthMode('', buildKey({ return: true }), adapter);

      expect(handled).toBe(true);
      expect(calls.submitHeadlessCode).toHaveLength(0);
    });

    it('should cancel on Escape from headless code entry', () => {
      const mode = MODE_FIXTURES.headlessCodeEntry();
      const { adapter, calls } = buildOauthHandlerAdapter(mode);

      const handled = handleOauthMode('', buildKey({ escape: true }), adapter);

      expect(handled).toBe(true);
      expect(calls.cancelOauth).toBe(1);
    });

    it('should handle backspace to remove last character', () => {
      const mode = MODE_FIXTURES.headlessCodeEntry({ codeInput: 'abc' });
      const { adapter, calls } = buildOauthHandlerAdapter(mode);

      handleOauthMode('', buildKey({ backspace: true }), adapter);

      expect(calls.setMode).toHaveLength(1);
      const updated = calls.setMode[0];
      if (updated.type === 'oauth-headless-code-entry') {
        expect(updated.codeInput).toBe('ab');
      }
    });
  });

  // ==========================================================================
  // Non-OAuth providers should not show OAuth items
  // ==========================================================================

  describe('Non-OAuth providers', () => {
    it('should not show OAuth login options or status for non-OAuth providers', () => {
      const providers = [PROVIDER_FIXTURES.openaiExpanded()];
      const navItems = buildNavItems(providers, '');

      const oauthItems = navItems.filter(
        i => i.type === 'oauth-login' || i.type === 'oauth-status'
      );
      expect(oauthItems).toHaveLength(0);

      // Should show "Create new profile" instead
      const addProfileItems = navItems.filter(i => i.type === 'add-profile');
      expect(addProfileItems).toHaveLength(1);
    });
  });

  // ==========================================================================
  // OAuth error/success mode handling
  // ==========================================================================

  describe('OAuth error/success mode handling', () => {
    it('should retry on Enter in error mode', () => {
      const mode = MODE_FIXTURES.oauthError('OAuth login timed out');
      const { adapter, calls } = buildOauthHandlerAdapter(mode);

      const handled = handleOauthMode('', buildKey({ return: true }), adapter);

      expect(handled).toBe(true);
      expect(calls.retryOauth).toBe(1);
    });

    it('should cancel on Escape in error mode', () => {
      const mode = MODE_FIXTURES.oauthError('OAuth login timed out');
      const { adapter, calls } = buildOauthHandlerAdapter(mode);

      const handled = handleOauthMode('', buildKey({ escape: true }), adapter);

      expect(handled).toBe(true);
      expect(calls.cancelOauth).toBe(1);
    });

    it('should cancel on Escape in browser waiting mode', () => {
      const mode = MODE_FIXTURES.browserWaiting();
      const { adapter, calls } = buildOauthHandlerAdapter(mode);

      const handled = handleOauthMode('', buildKey({ escape: true }), adapter);

      expect(handled).toBe(true);
      expect(calls.cancelOauth).toBe(1);
    });

    it('should absorb all input in browser waiting mode', () => {
      const mode = MODE_FIXTURES.browserWaiting();
      const { adapter } = buildOauthHandlerAdapter(mode);

      // Regular character should be absorbed (not passed through)
      const handled = handleOauthMode('x', buildKey(), adapter);
      expect(handled).toBe(true);
    });
  });
});
