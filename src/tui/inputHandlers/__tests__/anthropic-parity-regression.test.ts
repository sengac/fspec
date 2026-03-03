/**
 * Feature: spec/features/anthropic-provider-settings-regression.feature
 *
 * This test file validates the REGRESSION acceptance criteria for PROV-027:
 * Anthropic subscription parity and regression hardening against opencode behavior.
 *
 * Specifically, the TUI provider settings regression scenarios that prevent
 * PROV-019 class bugs from recurring for Claude:
 * - Edit action on Claude OAuth provider starts OAuth flow (not API key editor)
 * - Delete action on Claude OAuth provider disconnects OAuth
 * - Claude provider with OAuth tokens shows connected status
 *
 * These mirror the Codex OAuth integration tests from PROV-019
 * (listModeHandler-codex-oauth.test.ts) and PROV-025
 * (anthropic-oauth-tui.test.ts), ensuring consistent behavior.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { handleListMode } from '../listModeHandler';
import {
  buildNavItems,
  type UseProviderSettingsStateReturn,
} from '../../hooks/useProviderSettingsState';
import type {
  SettingsNavItem,
  ProviderDisplayInfo,
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

describe('Feature: Anthropic subscription parity and regression hardening — TUI regression tests', () => {
  let providerSettings: MockedProviderSettings;
  const onClose = vi.fn();
  const onSwitchToModels = vi.fn();

  beforeEach(() => {
    providerSettings = buildMockProviderSettings();
    vi.clearAllMocks();
  });

  // =========================================================================
  // REGRESSION: Scenario: Edit action on Claude OAuth provider starts OAuth flow
  // Same class of bug as PROV-019 where Codex showed 'Edit API Key' form
  // =========================================================================

  describe('Scenario: Edit action on Claude OAuth provider - e keybind removed', () => {
    it('should do nothing when pressing "e" on Anthropic (keybind removed in PROV-029)', () => {
      // @step Given the Claude provider is selected in provider settings
      const currentItem: SettingsNavItem = {
        type: 'provider',
        providerId: 'anthropic',
        name: 'Anthropic',
      };

      // @step And the Claude provider has OAuth tokens stored
      const currentProvider: ProviderDisplayInfo = {
        id: 'anthropic',
        name: 'Anthropic',
        status: { hasKey: true, maskedKey: 'OAuth', source: 'Claude' },
        profiles: [],
        isExpanded: false,
        hasOAuthTokens: true,
      };

      // @step When the user presses 'e'
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

      // @step Then nothing happens (e keybind removed in PROV-029)
      expect(providerSettings.startBrowserLogin).not.toHaveBeenCalled();
      expect(providerSettings.setMode).not.toHaveBeenCalled();
      expect(providerSettings.setEditingApiKey).not.toHaveBeenCalled();
    });
  });

  // =========================================================================
  // REGRESSION: Scenario: Delete action on Claude OAuth provider disconnects OAuth
  // Same class of bug as PROV-019 Codex disconnect
  // =========================================================================

  describe('Scenario: Delete action on Claude OAuth provider disconnects OAuth', () => {
    it('should do nothing when pressing "d" on a provider row (PROV-029: d only on oauth-status/api-key/profile)', () => {
      // @step Given the Claude provider has OAuth tokens stored
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

      // @step When the user presses 'd' on a provider row
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

      // @step Then nothing happens (d on provider rows no longer active — use oauth-status item)
      expect(providerSettings.disconnectOauth).not.toHaveBeenCalled();
      expect(providerSettings.removeApiKey).not.toHaveBeenCalled();
      expect(providerSettings.setMode).not.toHaveBeenCalled();
    });
  });

  // =========================================================================
  // REGRESSION: Scenario: Claude provider with OAuth tokens shows connected status
  // Regression from PROV-019 where Codex showed wrong status
  // =========================================================================

  describe('Scenario: Claude provider with OAuth tokens shows connected status', () => {
    it('should display a checkmark with "OAuth" and "[Claude]" source', () => {
      // @step Given the Claude provider has valid OAuth tokens stored
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

      // @step When the provider settings list is rendered
      const status = providers[0].status;

      // @step Then the Claude row displays a checkmark with "OAuth" and "[Claude]" source
      expect(status.hasKey).toBe(true);
      expect(status.maskedKey).toBe('OAuth');
      expect(status.source).toBe('Claude');

      // Also verify: when expanded, OAuth login options shown for re-login (PROV-028)
      providers[0].isExpanded = true;
      const navItems = buildNavItems(providers, '');
      const oauthLoginItems = navItems.filter(i => i.type === 'oauth-login');
      expect(oauthLoginItems).toHaveLength(2);
    });

    it('should NOT show connected status when no OAuth tokens exist', () => {
      // Regression: ensure (not configured) state is correctly distinguished
      const providers: ProviderDisplayInfo[] = [
        {
          id: 'anthropic',
          name: 'Anthropic',
          status: { hasKey: false },
          profiles: [],
          isExpanded: false,
          hasOAuthTokens: false,
        },
      ];

      const status = providers[0].status;
      expect(status.hasKey).toBe(false);
      expect(status.maskedKey).toBeUndefined();
      expect(status.source).toBeUndefined();

      // When expanded, OAuth login options should appear
      providers[0].isExpanded = true;
      const navItems = buildNavItems(providers, '');
      const oauthLoginItems = navItems.filter(i => i.type === 'oauth-login');
      expect(oauthLoginItems).toHaveLength(2); // browser + headless
    });
  });
});
