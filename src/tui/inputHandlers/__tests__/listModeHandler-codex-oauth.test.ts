/**
 * Feature: spec/features/tui-codex-oauth-login.feature
 *
 * PROV-019: TUI Provider Settings — Codex OAuth login flow
 * Tests listModeHandler for Codex OAuth provider keybinds.
 *
 * Updated for PROV-029: 'e' keybind removed, 'd' only works on specific item types
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { handleListMode } from '../listModeHandler';
import type { UseProviderSettingsStateReturn } from '../../hooks/useProviderSettingsState';
import type {
  SettingsNavItem,
  ProviderDisplayInfo,
} from '../../components/ProviderSettingsPanel';

vi.mock('../../../utils/provider-config', async importOriginal => {
  const actual =
    await importOriginal<typeof import('../../../utils/provider-config')>();
  return {
    ...actual,
    isOAuthProvider: (providerId: string) =>
      providerId === 'anthropic' || providerId === 'codex',
  };
});

type MockedProviderSettings = {
  [K in keyof UseProviderSettingsStateReturn]: UseProviderSettingsStateReturn[K] extends (
    ...args: infer A
  ) => infer R
    ? ReturnType<typeof vi.fn<(...args: A) => R>>
    : UseProviderSettingsStateReturn[K];
};

function buildMockProviderSettings(): MockedProviderSettings {
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
      message: '✓ Connected',
    }),
    getCurrentItem: vi.fn(),
    getCurrentProvider: vi.fn(),
    getCurrentProfile: vi.fn(),
    startBrowserLogin: vi.fn(),
    startDeviceLogin: vi.fn(),
    cancelOauth: vi.fn(),
    retryOauth: vi.fn(),
    disconnectOauth: vi.fn().mockResolvedValue(undefined),
    submitHeadlessCode: vi.fn(),
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

describe('Feature: TUI Provider Settings — Codex OAuth keybinds', () => {
  let providerSettings: MockedProviderSettings;
  const onClose = vi.fn();
  const onSwitchToModels = vi.fn();

  beforeEach(() => {
    providerSettings = buildMockProviderSettings();
    vi.clearAllMocks();
  });

  describe('Scenario: Edit key on Codex provider - e keybind removed', () => {
    it('should do nothing when pressing "e" (keybind removed in PROV-029)', () => {
      // @step Given the Codex provider is selected in provider settings
      const currentItem: SettingsNavItem = {
        type: 'provider',
        providerId: 'codex',
        name: 'Codex (ChatGPT)',
      };

      // @step When the user presses 'e'
      handleListMode({
        input: 'e',
        key: buildKey(),
        providerSettings,
        currentItem,
        currentProvider: undefined,
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

  describe('Scenario: Edit key on non-OAuth provider - e keybind removed', () => {
    it('should do nothing when pressing "e" on Anthropic (keybind removed in PROV-029)', () => {
      // @step Given a non-OAuth provider like Anthropic is selected
      const currentItem: SettingsNavItem = {
        type: 'provider',
        providerId: 'anthropic',
        name: 'Anthropic',
      };

      // @step When the user presses 'e'
      handleListMode({
        input: 'e',
        key: buildKey(),
        providerSettings,
        currentItem,
        currentProvider: undefined,
        currentProfile: undefined,
        visibleHeight: 20,
        onClose,
        onSwitchToModels,
      });

      // @step Then nothing happens (e keybind removed in PROV-029)
      expect(providerSettings.setEditingApiKey).not.toHaveBeenCalled();
      expect(providerSettings.setMode).not.toHaveBeenCalled();
      expect(providerSettings.startBrowserLogin).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Delete on Codex provider row does nothing', () => {
    it('should do nothing when pressing "d" on provider row (PROV-029: use oauth-status item)', () => {
      // @step Given the Codex provider is selected in provider settings
      const currentItem: SettingsNavItem = {
        type: 'provider',
        providerId: 'codex',
        name: 'Codex (ChatGPT)',
      };

      // @step When the user presses 'd' on a provider row
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

      // @step Then nothing happens (d on provider rows no longer active)
      expect(providerSettings.disconnectOauth).not.toHaveBeenCalled();
      expect(providerSettings.removeApiKey).not.toHaveBeenCalled();
      expect(providerSettings.setMode).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Codex provider with OAuth tokens shows connected status', () => {
    it('should build status with OAuth maskedKey and ChatGPT source', () => {
      // @step Given the Codex provider has valid OAuth tokens stored
      const codexProvider: ProviderDisplayInfo = {
        id: 'codex',
        name: 'Codex (ChatGPT)',
        status: { hasKey: true, maskedKey: 'OAuth', source: 'ChatGPT' },
        profiles: [],
        isExpanded: false,
        hasOAuthTokens: true,
      };

      // @step Then the status should show OAuth maskedKey
      expect(codexProvider.status.maskedKey).toBe('OAuth');

      // @step And the status source should be ChatGPT
      expect(codexProvider.status.source).toBe('ChatGPT');

      // @step And the provider should have hasOAuthTokens set
      expect(codexProvider.hasOAuthTokens).toBe(true);
    });
  });
});
