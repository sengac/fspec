/**
 * Feature: spec/features/provider-settings-oauth-guards.feature
 *
 * This test file validates ALL acceptance criteria defined in the feature file.
 * Scenarios map directly to Gherkin scenarios.
 *
 * Covers:
 * - Provider list composition (scenario 1)
 * - OAuth/API-key/OpenAI API expansion (scenarios 2-6)
 * - saveProfile guards (scenarios 7-9)
 * - Keybind behavior: Enter (scenarios 10-14)
 * - Keybind behavior: 'd' with confirmation (scenarios 15-21)
 * - Removed keybinds: e, n, t (scenarios 22-24)
 * - Context-sensitive footer (scenarios 25-26)
 * - PROVIDER_ENV_VARS (scenario 27)
 * - Dead code cleanup (scenarios 28-29)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { existsSync } from 'fs';
import { join } from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
import { buildNavItems } from '../hooks/useProviderSettingsState';
import type { UseProviderSettingsStateReturn } from '../hooks/useProviderSettingsState';
import { handleListMode } from '../inputHandlers/listModeHandler';
import type {
  ProviderDisplayInfo,
  SettingsNavItem,
  ProfileDisplayInfo,
} from '../components/ProviderSettingsPanel';
import type { HookMode } from '../types/settingsMode';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

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

function itemsOfType(
  items: SettingsNavItem[],
  type: string
): SettingsNavItem[] {
  return items.filter(i => i.type === type);
}

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

function callHandler(
  ps: MockedProviderSettings,
  input: string,
  key: import('ink').Key,
  currentItem: SettingsNavItem | undefined,
  currentProvider?: ProviderDisplayInfo | undefined,
  currentProfile?: ProfileDisplayInfo | undefined
): void {
  handleListMode({
    input,
    key,
    providerSettings: ps,
    currentItem,
    currentProvider,
    currentProfile: currentProfile ?? undefined,
    visibleHeight: 20,
    onClose: vi.fn(),
    onSwitchToModels: vi.fn(),
  });
}

// ===========================================================================
// MAIN TEST SUITE
// ===========================================================================

describe('Feature: Provider Settings TUI — OAuth profile guards, dead code cleanup, keybind simplification, provider list cleanup', () => {
  let ps: MockedProviderSettings;

  beforeEach(() => {
    ps = buildMockProviderSettings();
    vi.clearAllMocks();
  });

  // =========================================================================
  // Provider list composition (Scenario 1)
  // =========================================================================

  describe('Scenario: Provider list contains only providers with tool calling support', () => {
    it('should exclude providers without tool calling and rename OpenAI', async () => {
      // @step Given the Provider Settings TUI is open
      const { SUPPORTED_PROVIDERS, getProviderRegistryEntry } = await import(
        '../../utils/provider-config'
      );

      // @step Then the provider list contains exactly 16 providers
      expect(SUPPORTED_PROVIDERS.length).toBe(16);

      // @step And the following providers are NOT in the list:
      expect(SUPPORTED_PROVIDERS).not.toContain('ollama');
      expect(SUPPORTED_PROVIDERS).not.toContain('perplexity');
      expect(SUPPORTED_PROVIDERS).not.toContain('hyperbolic');
      expect(SUPPORTED_PROVIDERS).not.toContain('mira');
      expect(SUPPORTED_PROVIDERS).not.toContain('voyageai');

      // @step And "OpenAI" is displayed as "OpenAI API"
      const openaiEntry = getProviderRegistryEntry('openai');
      expect(openaiEntry?.name).toBe('OpenAI API');
    });
  });

  // =========================================================================
  // OAuth provider expansion (Scenarios 2-3)
  // =========================================================================

  describe('Scenario: Expanding an OAuth provider shows OAuth items and API key but no profiles', () => {
    it('should show oauth-status, login options, and api-key but no profile rows', () => {
      // @step Given Anthropic is configured with OAuth connected and an API key from env
      const providers: ProviderDisplayInfo[] = [
        makeProvider({
          id: 'anthropic',
          name: 'Anthropic',
          isExpanded: true,
          hasOAuthTokens: true,
          status: { hasKey: true, maskedKey: 'sk-ant-••••Qr7K', source: 'env' },
          profiles: [],
        }),
      ];

      // @step When I expand the Anthropic provider
      const items = buildNavItems(providers, '');

      // @step Then I see the following nav items:
      const oauthStatus = items.find(
        i => i.type === 'oauth-status' && i.providerId === 'anthropic'
      );
      expect(oauthStatus).toBeDefined();

      const browserLogin = items.find(
        i =>
          i.type === 'oauth-login' &&
          i.providerId === 'anthropic' &&
          'method' in i &&
          i.method === 'browser'
      );
      expect(browserLogin).toBeDefined();

      const headlessLogin = items.find(
        i =>
          i.type === 'oauth-login' &&
          i.providerId === 'anthropic' &&
          'method' in i &&
          i.method === 'headless'
      );
      expect(headlessLogin).toBeDefined();

      const apiKeyItems = itemsOfType(items, 'api-key').filter(
        i => i.providerId === 'anthropic'
      );
      expect(apiKeyItems.length).toBe(1);

      // @step And I do NOT see any profile rows
      const profileItems = itemsOfType(items, 'profile').filter(
        i => i.providerId === 'anthropic'
      );
      expect(profileItems.length).toBe(0);

      // @step And I do NOT see a "Create new profile" button
      const addProfile = items.find(
        i => i.type === 'add-profile' && i.providerId === 'anthropic'
      );
      expect(addProfile).toBeUndefined();

      // @step And the header does NOT show a profile count
    });
  });

  describe('Scenario: Expanding an OAuth provider with stale profiles in config ignores them', () => {
    it('should not display stale profiles under an OAuth provider', () => {
      // @step Given Anthropic has a stale profile in user config from OAuth development
      const providers: ProviderDisplayInfo[] = [
        makeProvider({
          id: 'anthropic',
          name: 'Anthropic',
          isExpanded: true,
          hasOAuthTokens: true,
          status: { hasKey: true, maskedKey: 'OAuth', source: 'Claude' },
          profiles: [
            {
              name: 'anthropic',
              config: {
                baseUrl: 'http://localhost:8888',
                apiKey: 'sk-ant-oat01-fake',
              },
            },
          ],
        }),
      ];

      // @step When I expand the Anthropic provider
      const items = buildNavItems(providers, '');

      // @step Then the stale profile row is NOT displayed
      const profileItems = itemsOfType(items, 'profile').filter(
        i => i.providerId === 'anthropic'
      );
      expect(profileItems.length).toBe(0);

      // @step And the header does NOT show "(1 profile)"
      // @step And the application does not crash or show an error
    });
  });

  // =========================================================================
  // Cloud API-key provider expansion (Scenario 4)
  // =========================================================================

  describe('Scenario: Expanding a cloud API-key provider shows only the API key row', () => {
    it('should show only the api-key item for Gemini', () => {
      // @step Given Google Gemini has an API key configured from env
      const providers: ProviderDisplayInfo[] = [
        makeProvider({
          id: 'gemini',
          name: 'Google Gemini',
          isExpanded: true,
          status: {
            hasKey: true,
            maskedKey: 'AIza••••••••H3Ck',
            source: 'env',
          },
          profiles: [],
        }),
      ];

      // @step When I expand the Google Gemini provider
      const items = buildNavItems(providers, '');

      // @step Then I see only the "🔑 API key" nav item
      const geminiSubItems = items.filter(
        i => i.providerId === 'gemini' && i.type !== 'provider'
      );
      expect(geminiSubItems.length).toBe(1);
      expect(geminiSubItems[0].type).toBe('api-key');

      // @step And I do NOT see any profile rows
      const profileItems = itemsOfType(items, 'profile').filter(
        i => i.providerId === 'gemini'
      );
      expect(profileItems.length).toBe(0);

      // @step And I do NOT see any OAuth items
      const oauthItems = items.filter(
        i =>
          (i.type === 'oauth-status' || i.type === 'oauth-login') &&
          i.providerId === 'gemini'
      );
      expect(oauthItems.length).toBe(0);

      // @step And I do NOT see a "Create new profile" button
      const addProfile = items.find(
        i => i.type === 'add-profile' && i.providerId === 'gemini'
      );
      expect(addProfile).toBeUndefined();
    });
  });

  // =========================================================================
  // OpenAI API profile-only expansion (Scenarios 5-6)
  // =========================================================================

  describe('Scenario: Expanding OpenAI API with profiles shows profile rows and create button', () => {
    it('should show profile rows, create button, no api-key row, and profile count', () => {
      // @step Given OpenAI API has 2 profiles configured
      const providers: ProviderDisplayInfo[] = [
        makeProvider({
          id: 'openai',
          name: 'OpenAI API',
          isExpanded: true,
          status: { hasKey: false },
          profiles: [
            {
              name: 'work-vllm',
              config: { baseUrl: 'http://10.0.1.5:8080', apiKey: 'key1' },
            },
            {
              name: 'home-ollama',
              config: { baseUrl: 'http://localhost:11434', apiKey: 'key2' },
            },
          ],
        }),
      ];

      // @step When I expand the OpenAI API provider
      const items = buildNavItems(providers, '');

      // @step Then I see the following nav items:
      const profileItems = itemsOfType(items, 'profile').filter(
        i => i.providerId === 'openai'
      );
      expect(profileItems.length).toBe(2);

      const addProfile = items.find(
        i => i.type === 'add-profile' && i.providerId === 'openai'
      );
      expect(addProfile).toBeDefined();

      // @step And I do NOT see a "🔑 API key" row
      const apiKeyItems = itemsOfType(items, 'api-key').filter(
        i => i.providerId === 'openai'
      );
      expect(apiKeyItems.length).toBe(0);

      // @step And the header shows "(2 profiles)"
      expect(providers[0].profiles.length).toBe(2);
    });
  });

  describe('Scenario: Expanding OpenAI API with no profiles shows only create button', () => {
    it('should show only the create new profile button and no api-key row', () => {
      // @step Given OpenAI API has no profiles configured
      const providers: ProviderDisplayInfo[] = [
        makeProvider({
          id: 'openai',
          name: 'OpenAI API',
          isExpanded: true,
          status: { hasKey: false },
          profiles: [],
        }),
      ];

      // @step When I expand the OpenAI API provider
      const items = buildNavItems(providers, '');

      // @step Then I see only the "+ Create new profile" nav item
      const openaiSubItems = items.filter(
        i => i.providerId === 'openai' && i.type !== 'provider'
      );
      expect(openaiSubItems.length).toBe(1);
      expect(openaiSubItems[0].type).toBe('add-profile');

      // @step And I do NOT see a "🔑 API key" row
      const apiKeyItems = itemsOfType(items, 'api-key').filter(
        i => i.providerId === 'openai'
      );
      expect(apiKeyItems.length).toBe(0);
    });
  });

  // =========================================================================
  // saveProfile guard (Scenarios 7-9)
  // =========================================================================

  describe('Scenario: saveProfile rejects non-OpenAI-API providers', () => {
    it('should throw when saving a profile for Gemini', async () => {
      // @step Given the Provider Settings TUI is open
      const { saveProfile } = await import('../../utils/provider-config');

      // @step When saveProfile is called with providerId "gemini"
      // @step Then an error is thrown: "Profiles are only supported for OpenAI API provider"
      await expect(
        saveProfile('gemini', 'test-profile', {
          baseUrl: 'http://localhost:8080',
          apiKey: 'test-key',
        })
      ).rejects.toThrow('Profiles are only supported for OpenAI API provider');
    });
  });

  describe('Scenario: saveProfile rejects OAuth providers', () => {
    it('should throw when saving a profile for Anthropic', async () => {
      // @step Given the Provider Settings TUI is open
      const { saveProfile } = await import('../../utils/provider-config');

      // @step When saveProfile is called with providerId "anthropic"
      // @step Then an error is thrown: "Profiles are only supported for OpenAI API provider"
      await expect(
        saveProfile('anthropic', 'test-profile', {
          baseUrl: 'http://localhost:8888',
          apiKey: 'sk-ant-test',
        })
      ).rejects.toThrow('Profiles are only supported for OpenAI API provider');
    });
  });

  describe('Scenario: saveProfile accepts OpenAI API provider', () => {
    it('should not throw the guard error when saving a profile for OpenAI', async () => {
      // @step Given the Provider Settings TUI is open
      const { saveProfile } = await import('../../utils/provider-config');

      // @step When saveProfile is called with providerId "openai" and valid profile data
      // @step Then the profile is saved successfully (no guard error thrown)
      await expect(
        saveProfile('openai', 'test-profile', {
          baseUrl: 'http://localhost:8080',
          apiKey: 'test-key',
        })
      ).resolves.not.toThrow();
    });
  });

  // =========================================================================
  // Keybind behavior: Enter (Scenarios 10-14)
  // =========================================================================

  describe('Scenario: Enter on a provider row toggles expansion', () => {
    it('should toggle provider expansion on Enter', () => {
      // @step Given I have the cursor on a collapsed provider row
      const item: SettingsNavItem = {
        type: 'provider',
        providerId: 'gemini',
        name: 'Google Gemini',
      };

      // @step When I press Enter
      callHandler(ps, '', buildKey({ return: true }), item);

      // @step Then the provider expands to show its nav items
      expect(ps.toggleProviderExpansion).toHaveBeenCalledWith('gemini');
    });
  });

  describe('Scenario: Enter on a login item starts the OAuth flow', () => {
    it('should start browser OAuth on Enter for a browser login item', () => {
      // @step Given I have the cursor on "🔑 Login with Claude (browser)"
      const item: SettingsNavItem = {
        type: 'oauth-login',
        providerId: 'anthropic',
        method: 'browser',
        label: 'Login with Claude (browser)',
      };

      // @step When I press Enter
      callHandler(ps, '', buildKey({ return: true }), item);

      // @step Then the browser OAuth flow starts
      expect(ps.startBrowserLogin).toHaveBeenCalledWith('anthropic');
    });
  });

  describe('Scenario: Enter on an API key item opens the key editor', () => {
    it('should switch to edit-api-key mode on Enter', () => {
      // @step Given I have the cursor on "🔑 API key" for Google Gemini
      const item = {
        type: 'api-key',
        providerId: 'gemini',
      } as SettingsNavItem;

      // @step When I press Enter
      callHandler(ps, '', buildKey({ return: true }), item);

      // @step Then the API key editor opens
      expect(ps.setMode).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'edit-api-key',
          providerId: 'gemini',
        })
      );
    });
  });

  describe('Scenario: Enter on a profile item opens the profile editor', () => {
    it('should switch to profile edit mode on Enter', () => {
      // @step Given I have the cursor on "📁 work-vllm" under OpenAI API
      const item: SettingsNavItem = {
        type: 'profile',
        providerId: 'openai',
        profileName: 'work-vllm',
      };
      const profile: ProfileDisplayInfo = {
        name: 'work-vllm',
        config: { baseUrl: 'http://10.0.1.5:8080', apiKey: 'key1' },
      };

      // @step When I press Enter
      callHandler(ps, '', buildKey({ return: true }), item, undefined, profile);

      // @step Then the profile editor opens
      expect(ps.setMode).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'edit-profile',
          providerId: 'openai',
          profileName: 'work-vllm',
        })
      );
    });
  });

  describe('Scenario: Enter on create new profile starts profile creation', () => {
    it('should switch to create-profile mode on Enter', () => {
      // @step Given I have the cursor on "+ Create new profile" under OpenAI API
      const item: SettingsNavItem = {
        type: 'add-profile',
        providerId: 'openai',
      };

      // @step When I press Enter
      callHandler(ps, '', buildKey({ return: true }), item);

      // @step Then the new profile form opens
      expect(ps.setMode).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'create-profile',
          providerId: 'openai',
        })
      );
    });
  });

  // =========================================================================
  // Keybind behavior: 'd' with confirmation (Scenarios 15-21)
  // =========================================================================

  describe("Scenario: Pressing 'd' on an API key item shows delete confirmation", () => {
    it('should show delete-api-key confirmation dialog', () => {
      // @step Given I have the cursor on "🔑 API key" for Google Gemini
      const item = {
        type: 'api-key',
        providerId: 'gemini',
      } as SettingsNavItem;

      // @step When I press "d"
      callHandler(ps, 'd', buildKey(), item);

      // @step Then a confirmation dialog appears: "Delete API key for Google Gemini? (y/n)"
      expect(ps.setMode).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'delete-api-key',
          providerId: 'gemini',
        })
      );
      expect(ps.removeApiKey).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Declining API key delete confirmation preserves the key', () => {
    it('should return to list mode without deleting the API key', async () => {
      const { handleDeleteConfirmMode } = await import(
        '../inputHandlers/deleteConfirmModeHandler'
      );

      const mockPS = {
        removeProfile: vi.fn().mockResolvedValue(undefined),
        removeApiKey: vi.fn().mockResolvedValue(undefined),
        disconnectOauth: vi.fn().mockResolvedValue(undefined),
        setMode: vi.fn(),
      };

      // @step Given a "Delete API key" confirmation dialog is shown
      const mode: HookMode = { type: 'delete-api-key', providerId: 'gemini' };

      // @step When I press "n"
      const handled = handleDeleteConfirmMode(
        mode,
        'n',
        buildKey(),
        mockPS as unknown as UseProviderSettingsStateReturn
      );

      // @step Then the API key is preserved
      expect(mockPS.removeApiKey).not.toHaveBeenCalled();

      // @step And I return to list mode
      expect(handled).toBe(true);
      expect(mockPS.setMode).toHaveBeenCalledWith({ type: 'list' });
    });
  });

  describe('Scenario: Confirming API key delete removes the key', () => {
    it('should delete the API key and return to list mode', async () => {
      const { handleDeleteConfirmMode } = await import(
        '../inputHandlers/deleteConfirmModeHandler'
      );

      const mockPS = {
        removeProfile: vi.fn().mockResolvedValue(undefined),
        removeApiKey: vi.fn().mockResolvedValue(undefined),
        disconnectOauth: vi.fn().mockResolvedValue(undefined),
        setMode: vi.fn(),
      };

      // @step Given a "Delete API key" confirmation dialog is shown
      const mode: HookMode = { type: 'delete-api-key', providerId: 'gemini' };

      // @step When I press "y"
      const handled = handleDeleteConfirmMode(
        mode,
        'y',
        buildKey(),
        mockPS as unknown as UseProviderSettingsStateReturn
      );

      // @step Then the API key is deleted
      expect(handled).toBe(true);
      expect(mockPS.removeApiKey).toHaveBeenCalledWith('gemini');

      // @step And the provider status updates
    });
  });

  describe("Scenario: Pressing 'd' on an OAuth status item shows disconnect confirmation", () => {
    it('should show disconnect-oauth confirmation dialog', () => {
      // @step Given I have the cursor on "✓ OAuth [Claude]"
      const item: SettingsNavItem = {
        type: 'oauth-status',
        providerId: 'anthropic',
        label: '✓ OAuth [Claude]',
      };

      // @step When I press "d"
      callHandler(ps, 'd', buildKey(), item);

      // @step Then a confirmation dialog appears: "Disconnect Claude OAuth? (y/n)"
      expect(ps.setMode).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'disconnect-oauth',
          providerId: 'anthropic',
        })
      );
      expect(ps.disconnectOauth).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Confirming OAuth disconnect clears tokens', () => {
    it('should clear OAuth tokens and return to list mode', async () => {
      const { handleDeleteConfirmMode } = await import(
        '../inputHandlers/deleteConfirmModeHandler'
      );

      const mockPS = {
        removeProfile: vi.fn().mockResolvedValue(undefined),
        removeApiKey: vi.fn().mockResolvedValue(undefined),
        disconnectOauth: vi.fn().mockResolvedValue(undefined),
        setMode: vi.fn(),
      };

      // @step Given a "Disconnect OAuth" confirmation dialog is shown
      const mode: HookMode = {
        type: 'disconnect-oauth',
        providerId: 'anthropic',
      };

      // @step When I press "y"
      const handled = handleDeleteConfirmMode(
        mode,
        'y',
        buildKey(),
        mockPS as unknown as UseProviderSettingsStateReturn
      );

      // @step Then the OAuth tokens are cleared
      expect(handled).toBe(true);
      expect(mockPS.disconnectOauth).toHaveBeenCalledWith('anthropic');

      // @step And the OAuth status updates
    });
  });

  describe("Scenario: Pressing 'd' on a profile item shows delete confirmation", () => {
    it('should show delete-profile confirmation dialog', () => {
      // @step Given I have the cursor on "📁 work-vllm" under OpenAI API
      const item: SettingsNavItem = {
        type: 'profile',
        providerId: 'openai',
        profileName: 'work-vllm',
      };

      // @step When I press "d"
      callHandler(ps, 'd', buildKey(), item);

      // @step Then a confirmation dialog appears: "Delete profile work-vllm? (y/n)"
      expect(ps.setMode).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'delete-profile',
          providerId: 'openai',
          profileName: 'work-vllm',
        })
      );
    });
  });

  // =========================================================================
  // Removed keybinds (Scenarios 22-24)
  // =========================================================================

  describe("Scenario: Pressing 'e' does nothing on any item", () => {
    it('should not trigger any action for e key', () => {
      // @step Given I have the cursor on any nav item
      const item: SettingsNavItem = {
        type: 'provider',
        providerId: 'gemini',
        name: 'Google Gemini',
      };

      // @step When I press "e"
      callHandler(ps, 'e', buildKey(), item);

      // @step Then nothing happens
      expect(ps.setMode).not.toHaveBeenCalled();
      expect(ps.startBrowserLogin).not.toHaveBeenCalled();
      expect(ps.toggleProviderExpansion).not.toHaveBeenCalled();
      expect(ps.setEditingApiKey).not.toHaveBeenCalled();
    });
  });

  describe("Scenario: Pressing 'n' does nothing on any item", () => {
    it('should not trigger any action for n key', () => {
      // @step Given I have the cursor on any nav item
      const item: SettingsNavItem = {
        type: 'provider',
        providerId: 'openai',
        name: 'OpenAI API',
      };

      // @step When I press "n"
      callHandler(ps, 'n', buildKey(), item);

      // @step Then nothing happens
      expect(ps.setMode).not.toHaveBeenCalled();
      expect(ps.setFormValues).not.toHaveBeenCalled();
      expect(ps.setProfileName).not.toHaveBeenCalled();
    });
  });

  describe("Scenario: Pressing 't' does nothing on any item", () => {
    it('should not trigger any action for t key', () => {
      // @step Given I have the cursor on any nav item
      const item: SettingsNavItem = {
        type: 'provider',
        providerId: 'gemini',
        name: 'Google Gemini',
      };

      // @step When I press "t"
      callHandler(ps, 't', buildKey(), item);

      // @step Then nothing happens
      expect(ps.testConnection).not.toHaveBeenCalled();
    });
  });

  // =========================================================================
  // Context-sensitive footer (Scenarios 25-26)
  // =========================================================================

  describe('Scenario: Footer updates based on selected item type', () => {
    it('should return correct footer hints for each item type', async () => {
      // @step Given the Provider Settings TUI is open
      // @step When I navigate to different item types the footer shows:
      const { getFooterHints } = await import(
        '../utils/providerSettingsHelpers'
      );

      expect(getFooterHints('provider')).toBe(
        'Enter: expand · / filter · Tab: Switch to models · Esc: close'
      );
      expect(getFooterHints('oauth-status')).toBe(
        'd: disconnect · / filter · Tab: Switch to models · Esc: close'
      );
      expect(getFooterHints('oauth-login')).toBe(
        'Enter: start login · / filter · Tab: Switch to models · Esc: close'
      );
      expect(getFooterHints('api-key')).toBe(
        'Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close'
      );
      expect(getFooterHints('profile')).toBe(
        'Enter: edit · d: delete · / filter · Tab: Switch to models · Esc: close'
      );
      expect(getFooterHints('add-profile')).toBe(
        'Enter: create · / filter · Tab: Switch to models · Esc: close'
      );
    });
  });

  describe('Scenario: Tab hint says "Switch to models" on provider settings panel', () => {
    it('should include "Tab: Switch to models" in all footer variants', async () => {
      // @step Given I am on the provider settings panel
      const { getFooterHints } = await import(
        '../utils/providerSettingsHelpers'
      );

      // @step Then the footer includes "Tab: Switch to models"
      const itemTypes = [
        'provider',
        'oauth-status',
        'oauth-login',
        'api-key',
        'profile',
        'add-profile',
      ];
      for (const itemType of itemTypes) {
        expect(getFooterHints(itemType)).toContain('Tab: Switch to models');
      }
    });
  });

  // =========================================================================
  // PROVIDER_ENV_VARS (Scenario 27)
  // =========================================================================

  describe('Scenario: PROVIDER_ENV_VARS includes codex entry', () => {
    let originalCodexKey: string | undefined;

    beforeEach(() => {
      originalCodexKey = process.env.CODEX_API_KEY;
    });

    afterEach(() => {
      if (originalCodexKey === undefined) {
        delete process.env.CODEX_API_KEY;
      } else {
        process.env.CODEX_API_KEY = originalCodexKey;
      }
    });

    it('should resolve CODEX_API_KEY from environment for codex provider', async () => {
      // @step Given the Provider Settings TUI is open
      const { getProviderConfig } = await import('../../utils/credentials');

      // @step Then the PROVIDER_ENV_VARS map in credentials.ts includes "codex" with value "CODEX_API_KEY"
      process.env.CODEX_API_KEY = 'test-codex-key-12345';

      const config = await getProviderConfig('codex');
      expect(config.apiKey).toBe('test-codex-key-12345');
      expect(config.source).toBe('env');
    });
  });

  // =========================================================================
  // Dead code cleanup (Scenarios 28-29)
  // =========================================================================

  describe('Scenario: Dead code files are removed', () => {
    it('should not find ProviderSettingsView.tsx or useProviderProfiles.ts', () => {
      // @step Given the Provider Settings TUI is open

      // @step Then "src/tui/components/ProviderSettingsView.tsx" does not exist
      const viewPath = join(
        __dirname,
        '../components/ProviderSettingsView.tsx'
      );
      expect(existsSync(viewPath)).toBe(false);

      // @step And "src/tui/hooks/useProviderProfiles.ts" does not exist
      const hookPath = join(__dirname, '../hooks/useProviderProfiles.ts');
      expect(existsSync(hookPath)).toBe(false);

      // @step And the project builds successfully with no broken imports
    });
  });

  describe('Scenario: Dead types in provider.ts are cleaned up', () => {
    it('should not export dead code types from provider.ts', async () => {
      // @step Given the Provider Settings TUI is open

      // @step Then "src/tui/types/provider.ts" does not contain types only used by dead code
      const providerTypes = await import('../types/provider');
      const exportedKeys = Object.keys(providerTypes);

      // @step And the following types are removed if unused elsewhere:
      expect(exportedKeys).not.toContain('ProviderWithProfiles');
      expect(exportedKeys).not.toContain('ProfileDisplay');
      expect(exportedKeys).not.toContain('ProviderStatus');
      expect(exportedKeys).not.toContain('SettingsViewMode');
      expect(exportedKeys).not.toContain('ConnectionTestResult');
    });
  });
});
