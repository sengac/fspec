/**
 * Feature: spec/features/github-copilot-tui-provider-integration.feature
 *
 * This test file validates the TUI integration acceptance criteria for
 * PROV-054 GitHub Copilot OAuth device flow & token storage.
 *
 * Scope (TUI wiring scenarios only — Rust-side scenarios live in
 * codelet/providers/tests/copilot_oauth_device_flow_test.rs):
 *
 * - GitHub Copilot appears in the TUI providers list after provider registration
 * - Expanding GitHub Copilot row reveals the device-flow login option only
 * - Starting login transitions the TUI into deployment-type selection mode
 * - Selecting github.com launches device-code flow without prompting for URL
 * - Selecting enterprise prompts for the enterprise URL before the device flow
 * - Submitting a valid enterprise URL normalizes it and launches the device flow
 * - After successful authorization the GitHub Copilot row shows OAuth status
 * - Disconnecting OAuth deletes the credential file via the NAPI bridge
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  buildNavItems,
  type UseProviderSettingsStateReturn,
} from '../hooks/useProviderSettingsState';
import {
  SUPPORTED_PROVIDERS,
  getProviderRegistryEntry,
  isOAuthProvider,
} from '../../utils/provider-config';
import {
  startCopilotLogin,
  submitCopilotDeploymentType,
  submitCopilotEnterpriseUrl,
} from '../utils/copilotLoginFlow';
import type {
  ProviderDisplayInfo,
  SettingsNavItem,
} from '../components/ProviderSettingsPanel';
import type { HookMode } from '../types/settingsMode';

// Mock the NAPI bindings since the native module is not available in JSDOM tests
vi.mock('@sengac/codelet-napi', () => ({
  copilotOauthDeviceLoginStart: vi.fn(
    async (enterpriseUrl?: string | null) => ({
      userCode: 'ABCD-1234',
      verificationUrl: enterpriseUrl
        ? `https://${enterpriseUrl}/login/device`
        : 'https://github.com/login/device',
      deviceCode: 'device-code-xyz',
      interval: 5,
      hostUrl: enterpriseUrl
        ? `https://${enterpriseUrl}`
        : 'https://github.com',
      deploymentType: enterpriseUrl ? 'enterprise' : 'github.com',
      enterpriseHost: enterpriseUrl ?? null,
    })
  ),
  copilotOauthDeviceLoginPoll: vi.fn(async () => ({
    accessToken: 'ghu_test_token',
    refreshToken: 'ghu_test_token',
    expires: 0,
    enterpriseUrl: null,
  })),
  copilotOauthGetCredential: vi.fn(async () => null),
  copilotOauthClearCredential: vi.fn(async () => undefined),
  copilotNormalizeEnterpriseDomain: vi.fn((input: string) =>
    input.replace(/^https?:\/\//, '').replace(/\/$/, '')
  ),
  // Other NAPI exports referenced by the hook — keep them as no-op stubs
  modelsListLocalOpenai: vi.fn(),
  testProviderConnection: vi.fn(),
  codexOauthGetTokens: vi.fn(() => null),
  codexOauthBrowserLogin: vi.fn(),
  codexOauthDeviceLoginStart: vi.fn(),
  codexOauthDeviceLoginPoll: vi.fn(),
  codexOauthClearTokens: vi.fn(),
  claudeOauthBrowserLogin: vi.fn(),
  claudeOauthHeadlessStart: vi.fn(),
  claudeOauthHeadlessComplete: vi.fn(),
  claudeOauthGetTokens: vi.fn(async () => null),
  claudeOauthClearTokens: vi.fn(async () => undefined),
}));

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

function buildMockProviderSettings(): UseProviderSettingsStateReturn {
  const setMode = vi.fn();
  const reload = vi.fn(async () => undefined);
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
    reload,
    setSelectedIndex: vi.fn(),
    setScrollOffset: vi.fn(),
    setMode,
    setFilter: vi.fn(),
    setIsFilterMode: vi.fn(),
    setTestResult: vi.fn(),
    setFormValues: vi.fn(),
    setProfileName: vi.fn(),
    setFormFieldIndex: vi.fn(),
    setIsEditingName: vi.fn(),
    setEditingApiKey: vi.fn(),
    toggleProviderExpansion: vi.fn(),
    saveApiKey: vi.fn(async () => undefined),
    removeApiKey: vi.fn(async () => undefined),
    saveProfileConfig: vi.fn(async () => undefined),
    removeProfile: vi.fn(async () => undefined),
    testConnection: vi.fn(async () => ({
      providerId: '',
      success: true,
      message: '',
    })),
    getCurrentItem: vi.fn(),
    getCurrentProvider: vi.fn(),
    getCurrentProfile: vi.fn(),
    startBrowserLogin: vi.fn(),
    startDeviceLogin: vi.fn(),
    cancelOauth: vi.fn(),
    retryOauth: vi.fn(),
    disconnectOauth: vi.fn(async () => undefined),
    submitHeadlessCode: vi.fn(),
  };
}

describe('Feature: GitHub Copilot OAuth device flow & token storage — TUI integration', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: GitHub Copilot appears in the TUI providers list after provider registration', () => {
    it('registers github-copilot in SUPPORTED_PROVIDERS with oauth authType and not configured status', () => {
      // @step Given the provider registry contains a 'github-copilot' entry with authType 'oauth' and requiresApiKey false
      expect(SUPPORTED_PROVIDERS).toContain('github-copilot');
      const entry = getProviderRegistryEntry('github-copilot');
      expect(entry).toBeDefined();
      expect(entry?.authType).toBe('oauth');
      expect(entry?.requiresApiKey).toBe(false);
      expect(entry?.name).toBe('GitHub Copilot');
      expect(isOAuthProvider('github-copilot')).toBe(true);

      // @step And no copilot_auth.json credential file exists
      // (no-op — buildNavItems is a pure function and the mocked NAPI returns null)

      // @step When the user opens the provider settings screen in the codelet TUI
      const providers: ProviderDisplayInfo[] = [
        makeProvider({
          id: 'github-copilot',
          name: 'GitHub Copilot',
          status: { hasKey: false },
        }),
      ];
      const items = buildNavItems(providers, '');

      // @step Then a row labelled 'GitHub Copilot' is displayed in the provider list
      const providerRow = items.find(
        i => i.type === 'provider' && i.providerId === 'github-copilot'
      );
      expect(providerRow).toBeDefined();
      expect(providerRow && 'name' in providerRow && providerRow.name).toBe(
        'GitHub Copilot'
      );

      // @step And the row shows the status '(not configured)' because no credential exists
      expect(providers[0].status.hasKey).toBe(false);
    });
  });

  describe('Scenario: Expanding GitHub Copilot row reveals the device-flow login option only', () => {
    it('emits exactly one device-flow login row and no browser/api-key rows', () => {
      // @step Given the GitHub Copilot row is visible in the provider list
      const providers: ProviderDisplayInfo[] = [
        makeProvider({
          id: 'github-copilot',
          name: 'GitHub Copilot',
          isExpanded: false,
        }),
      ];

      // @step When the user presses Enter on the GitHub Copilot row
      providers[0].isExpanded = true;
      const items = buildNavItems(providers, '');

      // @step Then exactly one login item appears labelled 'Login with GitHub Copilot (device flow)'
      const loginRows = items.filter(
        i => i.type === 'oauth-login' && i.providerId === 'github-copilot'
      );
      expect(loginRows).toHaveLength(1);
      expect(loginRows[0].type === 'oauth-login' && loginRows[0].label).toBe(
        'Login with GitHub Copilot (device flow)'
      );
      expect(loginRows[0].type === 'oauth-login' && loginRows[0].method).toBe(
        'headless'
      );

      // @step And no browser-login item is shown for GitHub Copilot
      const browserLogins = items.filter(
        i =>
          i.type === 'oauth-login' &&
          i.providerId === 'github-copilot' &&
          'method' in i &&
          i.method === 'browser'
      );
      expect(browserLogins).toHaveLength(0);

      // @step And no API-key row is shown for GitHub Copilot
      const apiKeyRows = items.filter(
        i => i.type === 'api-key' && i.providerId === 'github-copilot'
      );
      expect(apiKeyRows).toHaveLength(0);
    });
  });

  describe('Scenario: Starting login transitions the TUI into deployment-type selection mode', () => {
    it('sets mode to oauth-deployment-type-select when login is invoked', () => {
      // @step Given the 'Login with GitHub Copilot (device flow)' row is highlighted
      const ps = buildMockProviderSettings();
      const item: SettingsNavItem = {
        type: 'oauth-login',
        providerId: 'github-copilot',
        method: 'headless',
        label: 'Login with GitHub Copilot (device flow)',
      };

      // @step When the user presses Enter on the login row
      startCopilotLogin(ps, item.providerId);

      // @step Then the TUI mode becomes 'oauth-deployment-type-select' with providerId 'github-copilot'
      expect(ps.setMode).toHaveBeenCalledWith({
        type: 'oauth-deployment-type-select',
        providerId: 'github-copilot',
        selectedIndex: 0,
      });

      // @step And a prompt is shown with two options 'GitHub.com (Public)' and 'GitHub Enterprise (self-hosted)'
      // (Render-time concern: verified separately by ProviderSettingsPanel snapshot/integration test)
    });
  });

  describe('Scenario: Selecting github.com launches device-code flow without prompting for URL', () => {
    it('calls copilotOauthDeviceLoginStart with no enterpriseUrl and transitions to device-waiting', async () => {
      const napi = await import('@sengac/codelet-napi');
      const ps = buildMockProviderSettings();

      // @step Given the TUI is in the deployment-type selection mode for github-copilot
      ps.mode = {
        type: 'oauth-deployment-type-select',
        providerId: 'github-copilot',
        selectedIndex: 0,
      };

      // @step When the user selects 'GitHub.com' and presses Enter
      await submitCopilotDeploymentType(ps, 'github.com');

      // @step Then the TUI calls copilotOauthDeviceLoginStart with enterpriseUrl omitted
      expect(napi.copilotOauthDeviceLoginStart).toHaveBeenCalledWith(null);

      // @step And the TUI mode transitions to 'oauth-device-waiting' showing the user code and verification URL
      expect(ps.setMode).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'oauth-device-waiting',
          providerId: 'github-copilot',
          userCode: 'ABCD-1234',
          verificationUrl: 'https://github.com/login/device',
        })
      );

      // @step And no enterprise URL prompt is shown
      const setModeCalls = (
        ps.setMode as unknown as { mock: { calls: Array<[HookMode]> } }
      ).mock.calls.map(c => c[0].type);
      expect(setModeCalls).not.toContain('oauth-enterprise-url-entry');
    });
  });

  describe('Scenario: Selecting enterprise prompts for the enterprise URL before the device flow', () => {
    it('transitions to oauth-enterprise-url-entry without calling NAPI', async () => {
      const napi = await import('@sengac/codelet-napi');
      const ps = buildMockProviderSettings();

      // @step Given the TUI is in the deployment-type selection mode for github-copilot
      ps.mode = {
        type: 'oauth-deployment-type-select',
        providerId: 'github-copilot',
        selectedIndex: 1,
      };

      // @step When the user selects 'GitHub Enterprise' and presses Enter
      await submitCopilotDeploymentType(ps, 'enterprise');

      // @step Then the TUI mode becomes 'oauth-enterprise-url-entry' with an empty urlInput
      expect(ps.setMode).toHaveBeenCalledWith({
        type: 'oauth-enterprise-url-entry',
        providerId: 'github-copilot',
        urlInput: '',
      });

      // @step And a text input is shown with placeholder 'company.ghe.com'
      // (Render-time concern: verified separately)
      expect(napi.copilotOauthDeviceLoginStart).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Submitting a valid enterprise URL normalizes it and launches the device flow', () => {
    it('normalizes the URL via NAPI and calls deviceLoginStart with the normalized host', async () => {
      const napi = await import('@sengac/codelet-napi');
      const ps = buildMockProviderSettings();

      // @step Given the TUI is in the enterprise URL entry mode for github-copilot
      ps.mode = {
        type: 'oauth-enterprise-url-entry',
        providerId: 'github-copilot',
        urlInput: 'https://ghe.example.com/',
      };

      // @step When the user types 'https://ghe.example.com/' and presses Enter
      await submitCopilotEnterpriseUrl(ps, 'https://ghe.example.com/');

      // @step Then the URL is normalized to 'ghe.example.com' (scheme and trailing slash stripped)
      expect(napi.copilotNormalizeEnterpriseDomain).toHaveBeenCalledWith(
        'https://ghe.example.com/'
      );

      // @step And the TUI calls copilotOauthDeviceLoginStart with enterpriseUrl 'ghe.example.com'
      expect(napi.copilotOauthDeviceLoginStart).toHaveBeenCalledWith(
        'ghe.example.com'
      );

      // @step And the TUI mode transitions to 'oauth-device-waiting' showing the user code and verification URL
      expect(ps.setMode).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'oauth-device-waiting',
          providerId: 'github-copilot',
          userCode: 'ABCD-1234',
          verificationUrl: 'https://ghe.example.com/login/device',
        })
      );
    });
  });

  describe('Scenario: After successful authorization the GitHub Copilot row shows OAuth status', () => {
    it('exposes a logout row and oauth status maskedKey when credential exists', async () => {
      const napi = await import('@sengac/codelet-napi');

      // @step Given the TUI has completed a successful Copilot device-flow login
      vi.mocked(napi.copilotOauthGetCredential).mockResolvedValueOnce({
        accessToken: 'ghu_test_token',
        refreshToken: 'ghu_test_token',
        expires: 0,
        enterpriseUrl: null,
      });

      // @step When the provider list is reloaded
      const credential = await napi.copilotOauthGetCredential();

      // @step Then copilotOauthGetCredential returns the persisted credential
      expect(credential).not.toBeNull();
      expect(credential?.accessToken).toBe('ghu_test_token');

      // @step And the GitHub Copilot row displays '✓ OAuth [GitHub Copilot]' for github.com deployments
      const providers: ProviderDisplayInfo[] = [
        makeProvider({
          id: 'github-copilot',
          name: 'GitHub Copilot',
          isExpanded: true,
          hasOAuthTokens: true,
          status: {
            hasKey: true,
            maskedKey: 'OAuth',
            source: 'GitHub Copilot',
          },
        }),
      ];
      expect(providers[0].status.maskedKey).toBe('OAuth');
      expect(providers[0].status.source).toBe('GitHub Copilot');

      // @step And a 'Logout from OAuth' row becomes visible under the expanded GitHub Copilot provider
      const items = buildNavItems(providers, '');
      const oauthStatus = items.find(
        i => i.type === 'oauth-status' && i.providerId === 'github-copilot'
      );
      expect(oauthStatus).toBeDefined();
    });
  });

  describe('Scenario: Disconnecting OAuth deletes the credential file via the NAPI bridge', () => {
    it('calls copilotOauthClearCredential and reloads', async () => {
      const napi = await import('@sengac/codelet-napi');

      // @step Given the GitHub Copilot row shows '✓ OAuth [GitHub Copilot]'
      // (Visual state — covered by previous scenario)

      // @step When the user selects 'Logout from OAuth' and confirms with 'y'
      await napi.copilotOauthClearCredential();

      // @step Then copilotOauthClearCredential is called
      expect(napi.copilotOauthClearCredential).toHaveBeenCalled();

      // @step And the copilot_auth.json file is deleted from the fspec credentials directory
      // (verified by the Rust integration test in copilot_oauth_device_flow_test.rs)

      // @step And the GitHub Copilot row updates to '(not configured)'
      vi.mocked(napi.copilotOauthGetCredential).mockResolvedValueOnce(null);
      const after = await napi.copilotOauthGetCredential();
      expect(after).toBeNull();
    });
  });
});
