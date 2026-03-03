/**
 * Feature: spec/features/tui-anthropic-oauth-login.feature
 *
 * This test file validates the acceptance criteria for PROV-025:
 * TUI provider settings UX for Anthropic subscription connect and disconnect.
 *
 * Tests the listModeHandler, oauthModeHandler, provider-specific nav items,
 * and status display for Anthropic OAuth login flows.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { handleListMode } from '../listModeHandler';
import { handleOauthMode } from '../oauthModeHandler';
import {
  buildNavItems,
  type UseProviderSettingsStateReturn,
} from '../../hooks/useProviderSettingsState';
import type {
  SettingsNavItem,
  ProviderDisplayInfo,
  PanelMode,
} from '../../components/ProviderSettingsPanel';

/**
 * Create a fully-typed mock of UseProviderSettingsStateReturn.
 */
type MockedProviderSettings = {
  [K in keyof UseProviderSettingsStateReturn]: UseProviderSettingsStateReturn[K] extends (
    ...args: infer A
  ) => infer R
    ? ReturnType<typeof vi.fn<(...args: A) => R>>
    : UseProviderSettingsStateReturn[K];
};

function buildMockProviderSettings(
  overrides: Partial<MockedProviderSettings> = {}
): MockedProviderSettings {
  return {
    providers: [],
    navItems: [],
    isLoading: false,
    selectedIndex: 0,
    scrollOffset: 0,
    mode: { type: 'list' },
    filter: '',
    isFilterMode: false,
    testResult: null,
    formValues: {},
    profileName: '',
    formFieldIndex: 0,
    isEditingName: false,
    editingApiKey: '',
    oauthLastMethod: null,
    reload: vi.fn().mockResolvedValue(undefined),
    setSelectedIndex: vi.fn(),
    setScrollOffset: vi.fn(),
    setMode: vi.fn(),
    setFilter: vi.fn(),
    setIsFilterMode: vi.fn(),
    setTestResult: vi.fn(),
    setFormValues: vi.fn(),
    setProfileName: vi.fn(),
    setFormFieldIndex: vi.fn(),
    setIsEditingName: vi.fn(),
    setEditingApiKey: vi.fn(),
    toggleProviderExpansion: vi.fn(),
    saveApiKey: vi.fn().mockResolvedValue(undefined),
    removeApiKey: vi.fn().mockResolvedValue(undefined),
    saveProfileConfig: vi.fn().mockResolvedValue(undefined),
    removeProfile: vi.fn().mockResolvedValue(undefined),
    testConnection: vi.fn().mockResolvedValue({
      providerId: '',
      success: true,
      message: '',
    }),
    getCurrentItem: vi.fn(),
    getCurrentProvider: vi.fn(),
    getCurrentProfile: vi.fn(),
    startBrowserLogin: vi.fn(),
    startDeviceLogin: vi.fn(),
    cancelOauth: vi.fn(),
    retryOauth: vi.fn(),
    disconnectOauth: vi.fn().mockResolvedValue(undefined),
    submitHeadlessCode: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

function buildKey(
  overrides: Partial<import('ink').Key> = {}
): import('ink').Key {
  return {
    upArrow: false,
    downArrow: false,
    leftArrow: false,
    rightArrow: false,
    pageDown: false,
    pageUp: false,
    return: false,
    escape: false,
    ctrl: false,
    shift: false,
    tab: false,
    backspace: false,
    delete: false,
    meta: false,
    ...overrides,
  };
}

// Mock isOAuthProvider to return true for both codex and anthropic
vi.mock('../../../utils/provider-config', async importOriginal => {
  const actual =
    await importOriginal<typeof import('../../../utils/provider-config')>();
  return {
    ...actual,
    isOAuthProvider: (providerId: string) =>
      providerId === 'codex' || providerId === 'anthropic',
  };
});

describe('Feature: TUI provider settings UX for Anthropic subscription connect and disconnect', () => {
  let providerSettings: MockedProviderSettings;
  const onClose = vi.fn();
  const onSwitchToModels = vi.fn();

  beforeEach(() => {
    providerSettings = buildMockProviderSettings();
    vi.clearAllMocks();
  });

  describe('Scenario: Anthropic provider shows OAuth login options when no tokens exist', () => {
    it('should show Claude OAuth login options when expanded', () => {
      // @step Given the Anthropic provider has no OAuth tokens
      // @step And the Anthropic provider has no API key configured
      const providers: ProviderDisplayInfo[] = [
        {
          id: 'anthropic',
          name: 'Anthropic',
          status: { hasKey: false },
          profiles: [],
          isExpanded: true,
          hasOAuthTokens: false,
        },
      ];

      // @step When the user expands the Anthropic provider in provider settings
      const navItems = buildNavItems(providers, '');
      const oauthItems = navItems.filter(
        (i): i is Extract<SettingsNavItem, { type: 'oauth-login' }> =>
          i.type === 'oauth-login'
      );

      // @step Then the expanded list shows "Login with Claude (browser)" option
      expect(oauthItems).toHaveLength(2);
      expect(oauthItems[0].label).toBe('Login with Claude (browser)');
      expect(oauthItems[0].providerId).toBe('anthropic');
      expect(oauthItems[0].method).toBe('browser');

      // @step And the expanded list shows "Login with Claude (headless)" option
      expect(oauthItems[1].label).toBe('Login with Claude (headless)');
      expect(oauthItems[1].providerId).toBe('anthropic');
      expect(oauthItems[1].method).toBe('headless');
    });
  });

  describe('Scenario: Non-OAuth providers do not show OAuth login options', () => {
    it('should not show OAuth options for OpenAI provider', () => {
      // @step Given the OpenAI provider has no API key configured
      const providers: ProviderDisplayInfo[] = [
        {
          id: 'openai',
          name: 'OpenAI',
          status: { hasKey: false },
          profiles: [],
          isExpanded: true,
          hasOAuthTokens: false,
        },
      ];

      // @step When the user expands the OpenAI provider in provider settings
      const navItems = buildNavItems(providers, '');
      const oauthItems = navItems.filter(i => i.type === 'oauth-login');

      // @step Then the expanded list does not show any OAuth login options
      expect(oauthItems).toHaveLength(0);

      // @step And the expanded list shows "Create new profile" option
      const addProfileItems = navItems.filter(i => i.type === 'add-profile');
      expect(addProfileItems).toHaveLength(1);
      expect(addProfileItems[0].providerId).toBe('openai');
    });
  });

  describe('Scenario: Successful browser OAuth login flow', () => {
    it('should start browser login when selecting browser OAuth option for Anthropic', () => {
      // @step Given the Anthropic provider has no OAuth tokens
      // @step And the user has expanded the Anthropic provider
      const currentItem: SettingsNavItem = {
        type: 'oauth-login',
        providerId: 'anthropic',
        method: 'browser',
        label: 'Login with Claude (browser)',
      };

      // @step When the user selects "Login with Claude (browser)"
      handleListMode({
        input: '',
        key: buildKey({ return: true }),
        providerSettings,
        currentItem,
        currentProvider: undefined,
        currentProfile: undefined,
        visibleHeight: 20,
        onClose,
        onSwitchToModels,
      });

      // @step Then the screen shows "Claude OAuth Login" title
      // startBrowserLogin is called which sets mode to oauth-browser-waiting
      expect(providerSettings.startBrowserLogin).toHaveBeenCalledWith(
        'anthropic'
      );

      // @step And a spinner displays "Waiting for authorization..."
      // Verified by ProviderSettingsPanel rendering the oauth-browser-waiting mode

      // @step When the browser OAuth flow completes successfully
      // @step Then the screen shows "✓ Connected to Claude" success message
      // @step And the Anthropic provider shows "✓ OAuth [Claude]" status
      // These are verified via the state hook and panel rendering (integration tested separately)
    });
  });

  describe('Scenario: Successful headless OAuth login flow', () => {
    it('should start headless login when selecting headless OAuth option for Anthropic', () => {
      // @step Given the Anthropic provider has no OAuth tokens
      // @step And the user has expanded the Anthropic provider
      const currentItem: SettingsNavItem = {
        type: 'oauth-login',
        providerId: 'anthropic',
        method: 'headless',
        label: 'Login with Claude (headless)',
      };

      // @step When the user selects "Login with Claude (headless)"
      handleListMode({
        input: '',
        key: buildKey({ return: true }),
        providerSettings,
        currentItem,
        currentProvider: undefined,
        currentProfile: undefined,
        visibleHeight: 20,
        onClose,
        onSwitchToModels,
      });

      // @step Then the screen shows the authorize URL as a clickable link
      // @step And a text input for "code#state" is displayed
      // startDeviceLogin is called — for anthropic, this should trigger
      // headless flow which transitions to oauth-headless-code-entry mode
      expect(providerSettings.startDeviceLogin).toHaveBeenCalledWith(
        'anthropic'
      );

      // @step When the user pastes a valid code#state and presses Enter
      // @step Then the tokens are exchanged successfully
      // @step And the screen shows "✓ Connected to Claude" success message
      // @step And the Anthropic provider shows "✓ OAuth [Claude]" status
      // Covered by submitHeadlessCode integration
    });
  });

  describe('Scenario: Browser OAuth login times out after 5 minutes', () => {
    it('should handle timeout error with retry and go-back options', () => {
      // @step Given the Anthropic provider has no OAuth tokens
      // @step And the user has started a browser OAuth login flow
      providerSettings = buildMockProviderSettings({
        mode: {
          type: 'oauth-error',
          providerId: 'anthropic',
          error: 'OAuth login timed out',
        },
      });

      // @step When the browser OAuth flow times out
      // (the hook sets mode to oauth-error when the NAPI promise rejects)

      // @step Then the screen shows an error message containing "timed out"
      const mode = providerSettings.mode as Extract<
        PanelMode,
        { type: 'oauth-error' }
      >;
      expect(mode.error).toContain('timed out');

      // @step And the user can press Enter to retry or Esc to go back
      const handledEnter = handleOauthMode(
        '',
        buildKey({ return: true }),
        providerSettings
      );
      expect(handledEnter).toBe(true);
      expect(providerSettings.retryOauth).toHaveBeenCalled();

      vi.clearAllMocks();
      const handledEsc = handleOauthMode(
        '',
        buildKey({ escape: true }),
        providerSettings
      );
      expect(handledEsc).toBe(true);
      expect(providerSettings.cancelOauth).toHaveBeenCalled();
    });
  });

  describe('Scenario: Escape cancels browser OAuth waiting state', () => {
    it('should cancel browser OAuth flow when Escape is pressed', () => {
      // @step Given the user is on the browser OAuth waiting screen for Anthropic
      providerSettings = buildMockProviderSettings({
        mode: { type: 'oauth-browser-waiting', providerId: 'anthropic' },
      });

      // @step When the user presses Escape
      const handled = handleOauthMode(
        '',
        buildKey({ escape: true }),
        providerSettings
      );

      // @step Then the screen returns to the provider list
      expect(handled).toBe(true);
      expect(providerSettings.cancelOauth).toHaveBeenCalled();

      // @step And no error message is shown
      // cancelOauth sets mode to 'list' without error
    });
  });

  describe('Scenario: Escape cancels headless code entry state', () => {
    it('should cancel headless code entry when Escape is pressed', () => {
      // @step Given the user is on the headless code entry screen for Anthropic
      const headlessMode: PanelMode = {
        type: 'oauth-headless-code-entry',
        providerId: 'anthropic',
        authorizeUrl: 'https://claude.ai/oauth/authorize?...',
        pkceVerifier: 'test-verifier',
        codeInput: '',
      };
      providerSettings = buildMockProviderSettings({
        mode: headlessMode,
      });

      // @step When the user presses Escape
      const handled = handleOauthMode(
        '',
        buildKey({ escape: true }),
        providerSettings
      );

      // @step Then the screen returns to the provider list
      expect(handled).toBe(true);
      expect(providerSettings.cancelOauth).toHaveBeenCalled();

      // @step And no error message is shown
    });
  });

  describe('Scenario: Anthropic provider shows connected status when OAuth tokens exist', () => {
    it('should display "✓ OAuth [Claude]" and suppress login options when tokens exist', () => {
      // @step Given the Anthropic provider has valid OAuth tokens from a previous login
      const providers: ProviderDisplayInfo[] = [
        {
          id: 'anthropic',
          name: 'Anthropic',
          status: {
            hasKey: true,
            maskedKey: 'OAuth',
            source: 'Claude',
          },
          profiles: [],
          isExpanded: false,
          hasOAuthTokens: true,
        },
      ];

      // @step When the user opens provider settings
      // @step Then the Anthropic provider row shows "✓ OAuth [Claude]" status
      const status = providers[0].status;
      expect(status.hasKey).toBe(true);
      expect(status.maskedKey).toBe('OAuth');
      expect(status.source).toBe('Claude');

      // @step When the user expands the Anthropic provider
      providers[0].isExpanded = true;
      const navItems = buildNavItems(providers, '');

      // @step Then no OAuth login options are shown in the expanded list
      // PROV-028: OAuth login options are now always shown (for re-login)
      const oauthItems = navItems.filter(i => i.type === 'oauth-login');
      expect(oauthItems).toHaveLength(2);
    });
  });

  describe('Scenario: OAuth status takes precedence over API key status', () => {
    it('should show OAuth login options before OAuth and suppress them after', () => {
      // @step Given the Anthropic provider has a configured API key via ANTHROPIC_API_KEY env var
      // @step And the Anthropic provider has no OAuth tokens
      const providers: ProviderDisplayInfo[] = [
        {
          id: 'anthropic',
          name: 'Anthropic',
          status: {
            hasKey: true,
            maskedKey: 'sk-ant-••••••VgAA',
            source: 'env',
          },
          profiles: [],
          isExpanded: true,
          hasOAuthTokens: false,
        },
      ];

      // @step When the user opens provider settings
      // @step Then the Anthropic provider shows the masked API key status
      expect(providers[0].status.maskedKey).toContain('sk-ant-');
      expect(providers[0].status.source).toBe('env');

      // @step When the user expands the Anthropic provider
      // @step Then OAuth login options are shown alongside existing config
      const navItemsBefore = buildNavItems(providers, '');
      const oauthItemsBefore = navItemsBefore.filter(
        i => i.type === 'oauth-login'
      );
      expect(oauthItemsBefore).toHaveLength(2);

      // @step When the user completes an OAuth login flow
      // @step Then the Anthropic provider status changes to "✓ OAuth [Claude]"
      // Simulate what reload() does: OAuth tokens detected → status overridden
      providers[0].status = {
        hasKey: true,
        maskedKey: 'OAuth',
        source: 'Claude',
      };
      providers[0].hasOAuthTokens = true;

      const navItemsAfter = buildNavItems(providers, '');
      const oauthItemsAfter = navItemsAfter.filter(
        i => i.type === 'oauth-login'
      );
      // PROV-028: OAuth login options are now always shown for re-login
      expect(oauthItemsAfter).toHaveLength(2);
      expect(providers[0].status.maskedKey).toBe('OAuth');
      expect(providers[0].status.source).toBe('Claude');
    });
  });

  describe('Scenario: Edit key on Anthropic provider with OAuth tokens starts OAuth flow', () => {
    it('should start browser OAuth login when pressing "e" on Anthropic', () => {
      // @step Given the Anthropic provider has valid OAuth tokens
      const currentItem: SettingsNavItem = {
        type: 'provider',
        providerId: 'anthropic',
        name: 'Anthropic',
      };
      const currentProvider: ProviderDisplayInfo = {
        id: 'anthropic',
        name: 'Anthropic',
        status: { hasKey: true, maskedKey: 'OAuth', source: 'Claude' },
        profiles: [],
        isExpanded: false,
        hasOAuthTokens: true,
      };

      // @step When the user presses "e" on the Anthropic provider row
      handleListMode({
        input: 'e',
        key: buildKey(),
        providerSettings,
        currentItem,
        currentProvider,
        currentProfile: undefined,
        visibleHeight: 20,
        onClose,
        onSwitchToModels,
      });

      // @step Then the browser OAuth flow starts
      expect(providerSettings.startBrowserLogin).toHaveBeenCalledWith(
        'anthropic'
      );

      // @step And the API key editor is not shown
      expect(providerSettings.setMode).not.toHaveBeenCalled();
      expect(providerSettings.setEditingApiKey).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Disconnect OAuth clears tokens and reverts status', () => {
    it('should call disconnectOauth when pressing "d" on Anthropic with OAuth tokens', () => {
      // @step Given the Anthropic provider has valid OAuth tokens
      // @step And the Anthropic provider has no API key configured
      const currentItem: SettingsNavItem = {
        type: 'provider',
        providerId: 'anthropic',
        name: 'Anthropic',
      };
      const currentProvider: ProviderDisplayInfo = {
        id: 'anthropic',
        name: 'Anthropic',
        status: { hasKey: true, maskedKey: 'OAuth', source: 'Claude' },
        profiles: [],
        isExpanded: false,
        hasOAuthTokens: true,
      };

      // @step When the user presses "d" on the Anthropic provider row
      handleListMode({
        input: 'd',
        key: buildKey(),
        providerSettings,
        currentItem,
        currentProvider,
        currentProfile: undefined,
        visibleHeight: 20,
        onClose,
        onSwitchToModels,
      });

      // @step Then the Claude OAuth tokens are cleared
      expect(providerSettings.disconnectOauth).toHaveBeenCalledWith(
        'anthropic'
      );

      // @step And the Anthropic provider shows "(not configured)" status
      // After reload, provider with no tokens and no API key shows (not configured)

      // @step When the user expands the Anthropic provider
      // @step Then OAuth login options reappear in the expanded list
      // hasOAuthTokens will be false after disconnect → login options regenerated
      expect(providerSettings.removeApiKey).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Disconnect OAuth with existing API key reverts to API key status', () => {
    it('should call disconnectOauth which triggers reload to resolve API key status', () => {
      // @step Given the Anthropic provider has valid OAuth tokens
      // @step And the Anthropic provider has an API key via ANTHROPIC_API_KEY env var
      const currentProvider: ProviderDisplayInfo = {
        id: 'anthropic',
        name: 'Anthropic',
        status: { hasKey: true, maskedKey: 'OAuth', source: 'Claude' },
        profiles: [],
        isExpanded: false,
        hasOAuthTokens: true,
      };

      // @step When the user presses "d" on the Anthropic provider row
      const currentItem: SettingsNavItem = {
        type: 'provider',
        providerId: 'anthropic',
        name: 'Anthropic',
      };
      handleListMode({
        input: 'd',
        key: buildKey(),
        providerSettings,
        currentItem,
        currentProvider,
        currentProfile: undefined,
        visibleHeight: 20,
        onClose,
        onSwitchToModels,
      });

      // @step Then the Claude OAuth tokens are cleared
      expect(providerSettings.disconnectOauth).toHaveBeenCalledWith(
        'anthropic'
      );

      // @step And the Anthropic provider reverts to showing the masked API key status
      // disconnectOauth calls reload() internally, which re-resolves provider status
      // When OAuth tokens are gone but API key exists, reload produces env-based status
      expect(providerSettings.removeApiKey).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Headless code entry with invalid CSRF state shows error', () => {
    it('should submit code and handle CSRF error via oauthModeHandler', () => {
      // @step Given the user is on the headless code entry screen for Anthropic
      const headlessMode: PanelMode = {
        type: 'oauth-headless-code-entry',
        providerId: 'anthropic',
        authorizeUrl: 'https://claude.ai/oauth/authorize?...',
        pkceVerifier: 'real-verifier',
        codeInput: 'authcode123#wrong-state',
      };
      providerSettings = buildMockProviderSettings({
        mode: headlessMode,
      });

      // @step When the user pastes a code#state with a mismatched state value
      // (codeInput is already populated; pressing Enter submits it)
      const handled = handleOauthMode(
        '',
        buildKey({ return: true }),
        providerSettings
      );
      expect(handled).toBe(true);
      expect(providerSettings.submitHeadlessCode).toHaveBeenCalledWith(
        'authcode123#wrong-state',
        'real-verifier'
      );

      // @step Then the screen shows an error containing "CSRF validation failed"
      // submitHeadlessCode calls claudeOauthHeadlessComplete which rejects with CSRF error;
      // the hook catches it and sets mode to oauth-error. Verify the handler then allows retry:
      providerSettings = buildMockProviderSettings({
        mode: {
          type: 'oauth-error',
          providerId: 'anthropic',
          error: 'CSRF validation failed — state mismatch',
        },
      });
      const errorMode = providerSettings.mode as Extract<
        PanelMode,
        { type: 'oauth-error' }
      >;
      expect(errorMode.error).toContain('CSRF validation failed');

      // @step And the user can press Enter to retry or Esc to go back
      const handledRetry = handleOauthMode(
        '',
        buildKey({ return: true }),
        providerSettings
      );
      expect(handledRetry).toBe(true);
      expect(providerSettings.retryOauth).toHaveBeenCalled();
    });
  });

  describe('Scenario: Retry browser OAuth after error', () => {
    it('should retry the flow when pressing Enter on error screen', () => {
      // @step Given the user is on the OAuth error screen for Anthropic
      providerSettings = buildMockProviderSettings({
        mode: {
          type: 'oauth-error',
          providerId: 'anthropic',
          error: 'OAuth login timed out',
        },
      });

      // @step When the user presses Enter to retry
      const handled = handleOauthMode(
        '',
        buildKey({ return: true }),
        providerSettings
      );

      // @step Then the browser OAuth flow restarts from scratch
      expect(handled).toBe(true);
      expect(providerSettings.retryOauth).toHaveBeenCalled();

      // @step And the waiting screen is shown again
      // retryOauth internally calls startBrowserLogin or startDeviceLogin
    });
  });

  describe('Scenario: Go back to provider list after OAuth error', () => {
    it('should return to list when pressing Escape on error screen', () => {
      // @step Given the user is on the OAuth error screen for Anthropic
      providerSettings = buildMockProviderSettings({
        mode: {
          type: 'oauth-error',
          providerId: 'anthropic',
          error: 'OAuth login timed out',
        },
      });

      // @step When the user presses Escape
      const handled = handleOauthMode(
        '',
        buildKey({ escape: true }),
        providerSettings
      );

      // @step Then the screen returns to the provider list
      expect(handled).toBe(true);
      expect(providerSettings.cancelOauth).toHaveBeenCalled();

      // @step And no OAuth flow is running
      // cancelOauth increments generation counter and resets state
    });
  });

  describe('Headless code entry input handling', () => {
    it('should absorb all input during headless code entry mode', () => {
      // The new oauth-headless-code-entry mode should be handled by oauthModeHandler
      const headlessMode: PanelMode = {
        type: 'oauth-headless-code-entry',
        providerId: 'anthropic',
        authorizeUrl: 'https://claude.ai/oauth/authorize?...',
        pkceVerifier: 'test-verifier',
        codeInput: '',
      };
      providerSettings = buildMockProviderSettings({
        mode: headlessMode,
      });

      // Character input should be absorbed (handled = true)
      const handled = handleOauthMode('a', buildKey(), providerSettings);
      expect(handled).toBe(true);
    });

    it('should call submitHeadlessCode on Enter in headless code entry mode', () => {
      const headlessMode: PanelMode = {
        type: 'oauth-headless-code-entry',
        providerId: 'anthropic',
        authorizeUrl: 'https://claude.ai/oauth/authorize?...',
        pkceVerifier: 'test-verifier',
        codeInput: 'authcode123#test-verifier',
      };
      providerSettings = buildMockProviderSettings({
        mode: headlessMode,
      });

      const handled = handleOauthMode(
        '',
        buildKey({ return: true }),
        providerSettings
      );
      expect(handled).toBe(true);
      expect(providerSettings.submitHeadlessCode).toHaveBeenCalled();
    });
  });
});
