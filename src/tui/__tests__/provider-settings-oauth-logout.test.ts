/**
 * Feature: spec/features/provider-settings-oauth-logout.feature
 *
 * This test file validates the acceptance criteria for PROV-035:
 * Replace OAuth status echo with actionable Logout line in Provider Settings.
 *
 * Covers:
 * - OAuth status label changes (scenarios 1-2)
 * - Enter key triggers disconnect confirmation (scenario 3)
 * - Backward compat: 'd' still works (scenario 4)
 * - Footer hint updates (scenario 5)
 * - Negative cases: no logout for non-OAuth / no tokens (scenarios 6-7)
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { buildNavItems } from '../hooks/useProviderSettingsState';
import { handleListMode } from '../inputHandlers/listModeHandler';
import type { UseProviderSettingsStateReturn } from '../hooks/useProviderSettingsState';
import type {
  ProviderDisplayInfo,
  SettingsNavItem,
} from '../components/ProviderSettingsPanel';

// Mock isOAuthProvider: anthropic and codex are OAuth, everything else is not
vi.mock('../../utils/provider-config', async importOriginal => {
  const actual =
    await importOriginal<typeof import('../../utils/provider-config')>();
  return {
    ...actual,
    isOAuthProvider: (providerId: string) =>
      providerId === 'anthropic' || providerId === 'codex',
  };
});

// --- Shared helpers ---

function makeProvider(
  overrides: Partial<ProviderDisplayInfo> & { id: string; name: string }
): ProviderDisplayInfo {
  return {
    status: { hasKey: false },
    profiles: [],
    isExpanded: false,
    hasOAuthTokens: false,
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
    saveProfileConfig: vi.fn().mockResolvedValue(undefined),
    removeProfile: vi.fn().mockResolvedValue(undefined),
    saveApiKey: vi.fn().mockResolvedValue(undefined),
    removeApiKey: vi.fn().mockResolvedValue(undefined),
    testConnection: vi
      .fn()
      .mockResolvedValue({ providerId: '', success: true, message: '' }),
    getCurrentItem: vi.fn().mockReturnValue(undefined),
    getCurrentProvider: vi.fn().mockReturnValue(undefined),
    getCurrentProfile: vi.fn().mockReturnValue(undefined),
    startBrowserLogin: vi.fn(),
    startDeviceLogin: vi.fn(),
    cancelOauth: vi.fn(),
    retryOauth: vi.fn(),
    disconnectOauth: vi.fn().mockResolvedValue(undefined),
    submitHeadlessCode: vi.fn(),
    ...overrides,
  };
}

describe('Feature: Replace OAuth status echo with actionable Logout line in Provider Settings', () => {
  let providerSettings: MockedProviderSettings;
  const onClose = vi.fn();
  const onSwitchToModels = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    providerSettings = buildMockProviderSettings();
  });

  // --- OAuth status label ---

  describe('Scenario: Expanding Anthropic with OAuth shows logout line', () => {
    it('should show "Logout from OAuth [Claude]" as a child item', () => {
      // @step Given the Provider Settings TUI is open
      // @step Given Anthropic has valid OAuth tokens connected
      const providers = [
        makeProvider({
          id: 'anthropic',
          name: 'Anthropic',
          isExpanded: true,
          hasOAuthTokens: true,
          status: { hasKey: true, maskedKey: 'OAuth', source: 'Claude' },
        }),
      ];

      // @step When I expand the Anthropic provider
      const navItems = buildNavItems(providers, '');

      // @step Then I see "Logout from OAuth [Claude]" as a child item
      const oauthStatusItems = navItems.filter(
        (i): i is Extract<SettingsNavItem, { type: 'oauth-status' }> =>
          i.type === 'oauth-status'
      );
      expect(oauthStatusItems).toHaveLength(1);
      expect(oauthStatusItems[0].label).toBe('Logout from OAuth [Claude]');
    });
  });

  describe('Scenario: Expanding Codex with OAuth shows logout line', () => {
    it('should show "Logout from OAuth [ChatGPT]" as a child item', () => {
      // @step Given the Provider Settings TUI is open
      // @step Given Codex (ChatGPT) has valid OAuth tokens connected
      const providers = [
        makeProvider({
          id: 'codex',
          name: 'Codex',
          isExpanded: true,
          hasOAuthTokens: true,
          status: { hasKey: true, maskedKey: 'OAuth', source: 'ChatGPT' },
        }),
      ];

      // @step When I expand the Codex provider
      const navItems = buildNavItems(providers, '');

      // @step Then I see "Logout from OAuth [ChatGPT]" as a child item
      const oauthStatusItems = navItems.filter(
        (i): i is Extract<SettingsNavItem, { type: 'oauth-status' }> =>
          i.type === 'oauth-status'
      );
      expect(oauthStatusItems).toHaveLength(1);
      expect(oauthStatusItems[0].label).toBe('Logout from OAuth [ChatGPT]');
    });
  });

  // --- Enter triggers disconnect ---

  describe('Scenario: Enter on logout line triggers disconnect confirmation', () => {
    it('should show disconnect-oauth confirmation dialog when pressing Enter', () => {
      // @step Given the Provider Settings TUI is open
      // @step Given Anthropic has valid OAuth tokens connected
      // @step And I have the cursor on "Logout from OAuth [Claude]"
      const currentItem: SettingsNavItem = {
        type: 'oauth-status',
        providerId: 'anthropic',
        label: 'Logout from OAuth [Claude]',
      };

      // @step When I press Enter
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

      // @step Then a confirmation dialog appears: "Disconnect Claude OAuth? (y/n)"
      expect(providerSettings.setMode).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'disconnect-oauth',
          providerId: 'anthropic',
        })
      );
    });
  });

  // --- Backward compat: 'd' still works ---

  describe("Scenario: Pressing 'd' on logout line triggers disconnect confirmation", () => {
    it('should show disconnect-oauth confirmation dialog when pressing d', () => {
      // @step Given the Provider Settings TUI is open
      // @step Given Anthropic has valid OAuth tokens connected
      // @step And I have the cursor on "Logout from OAuth [Claude]"
      const currentItem: SettingsNavItem = {
        type: 'oauth-status',
        providerId: 'anthropic',
        label: 'Logout from OAuth [Claude]',
      };

      // @step When I press "d"
      handleListMode({
        input: 'd',
        key: buildKey(),
        providerSettings,
        currentItem,
        currentProvider: undefined,
        currentProfile: undefined,
        visibleHeight: 20,
        onClose,
        onSwitchToModels,
      });

      // @step Then a confirmation dialog appears: "Disconnect Claude OAuth? (y/n)"
      expect(providerSettings.setMode).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'disconnect-oauth',
          providerId: 'anthropic',
        })
      );
    });
  });

  // --- Footer hint ---

  describe('Scenario: Footer shows Enter logout hint when logout line is selected', () => {
    it('should return "Enter: logout" footer when oauth-status is selected', async () => {
      // @step Given the Provider Settings TUI is open
      // @step Given Anthropic has valid OAuth tokens connected
      // @step And I have the cursor on "Logout from OAuth [Claude]"
      const { getFooterHints } = await import(
        '../utils/providerSettingsHelpers'
      );

      // @step Then the footer shows "Enter: logout · / filter · Tab: Switch to models · Esc: close"
      expect(getFooterHints('oauth-status')).toBe(
        'Enter: logout · / filter · Tab: Switch to models · Esc: close'
      );
    });
  });

  // --- Negative cases: no change ---

  describe('Scenario: Non-OAuth provider has no logout line when expanded', () => {
    it('should not show any OAuth logout or status items for Gemini', () => {
      // @step Given the Provider Settings TUI is open
      // @step Given Google Gemini has an API key configured
      const providers = [
        makeProvider({
          id: 'gemini',
          name: 'Google Gemini',
          isExpanded: true,
          hasOAuthTokens: false,
          status: { hasKey: true, maskedKey: 'AI••••za', source: 'env' },
        }),
      ];

      // @step When I expand the Google Gemini provider
      const navItems = buildNavItems(providers, '');

      // @step Then I do NOT see any OAuth logout or status items
      const oauthItems = navItems.filter(
        i => i.type === 'oauth-status' || i.type === 'oauth-login'
      );
      expect(oauthItems).toHaveLength(0);
    });
  });

  describe('Scenario: OAuth provider without tokens has no logout line', () => {
    it('should not show a logout line but should show login options', () => {
      // @step Given the Provider Settings TUI is open
      // @step Given Anthropic has no OAuth tokens stored
      const providers = [
        makeProvider({
          id: 'anthropic',
          name: 'Anthropic',
          isExpanded: true,
          hasOAuthTokens: false,
          status: { hasKey: false },
        }),
      ];

      // @step When I expand the Anthropic provider
      const navItems = buildNavItems(providers, '');

      // @step Then I do NOT see a logout line
      const oauthStatusItems = navItems.filter(i => i.type === 'oauth-status');
      expect(oauthStatusItems).toHaveLength(0);

      // @step And I see OAuth login options for Claude
      const oauthLoginItems = navItems.filter(
        (i): i is Extract<SettingsNavItem, { type: 'oauth-login' }> =>
          i.type === 'oauth-login'
      );
      expect(oauthLoginItems.length).toBeGreaterThan(0);
      expect(oauthLoginItems[0].label).toContain('Claude');
    });
  });
});
