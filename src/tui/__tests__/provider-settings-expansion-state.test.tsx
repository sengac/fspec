/**
 * Feature: spec/features/provider-settings-expansion-state.feature
 *
 * This test file validates the acceptance criteria for PROV-036:
 * Provider settings tree collapses after OAuth logout confirmation.
 *
 * Tests exercise the ACTUAL hook lifecycle via ink-testing-library render:
 * - reload() preserves expansion state via expandedProviderIds ref
 * - disconnectOauth/removeApiKey set navigateToProviderRef which repositions
 *   selectedIndex to the parent provider row after reload
 * - Cancel path leaves cursor and expansion untouched
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Text } from 'ink';

// ---------------------------------------------------------------------------
// Mutable state knobs — controlled from tests, read by mocks
// ---------------------------------------------------------------------------

let claudeHasTokens = true;

// ---------------------------------------------------------------------------
// Mocks — hoisted, network boundary only
// ---------------------------------------------------------------------------

vi.mock('@sengac/codelet-napi', () => ({
  codexOauthGetTokens: vi.fn(() => null),
  codexOauthBrowserLogin: vi.fn(),
  codexOauthDeviceLoginStart: vi.fn(),
  codexOauthDeviceLoginPoll: vi.fn(),
  codexOauthClearTokens: vi.fn(),
  claudeOauthBrowserLogin: vi.fn(),
  claudeOauthHeadlessStart: vi.fn(),
  claudeOauthHeadlessComplete: vi.fn(),
  claudeOauthGetTokens: vi.fn(async () => {
    if (claudeHasTokens) {
      return { accessToken: 'fake', refreshToken: 'fake' };
    }
    return null;
  }),
  claudeOauthClearTokens: vi.fn(async () => {
    claudeHasTokens = false;
  }),
  modelsListLocalOpenai: vi.fn(async () => []),
  testProviderConnection: vi.fn(async () => ({ success: true })),
}));

vi.mock('../../utils/provider-config', async importOriginal => {
  const actual =
    await importOriginal<typeof import('../../utils/provider-config')>();
  return {
    ...actual,
    getProviderRegistry: vi.fn(() => ['anthropic', 'gemini']),
    getProviderRegistryEntry: vi.fn((id: string) => {
      if (id === 'anthropic') {
        return {
          id: 'anthropic',
          name: 'Anthropic',
          baseUrl: 'https://api.anthropic.com/v1',
          envVar: 'ANTHROPIC_API_KEY',
          authMethod: 'x-api-key',
          authType: 'oauth',
          requiresApiKey: true,
          description: 'Anthropic Claude models',
        };
      }
      if (id === 'gemini') {
        return {
          id: 'gemini',
          name: 'Google Gemini',
          baseUrl: 'https://generativelanguage.googleapis.com/v1beta',
          envVar: 'GOOGLE_GENERATIVE_AI_API_KEY',
          authMethod: 'query_param',
          authType: 'api-key',
          requiresApiKey: true,
          description: 'Google Gemini models',
        };
      }
      return undefined;
    }),
    isOAuthProvider: (providerId: string) => providerId === 'anthropic',
    loadProviderProfiles: vi.fn(async () => ({})),
    saveProfile: vi.fn(),
    deleteProfile: vi.fn(async () => undefined),
    getProfile: vi.fn(async () => null),
  };
});

vi.mock('../../utils/credentials', () => ({
  getProviderConfig: vi.fn(async () => ({})),
  setProviderCredential: vi.fn(),
  deleteProviderCredential: vi.fn(async () => undefined),
  maskApiKey: vi.fn((key: string) => `••••${key.slice(-4)}`),
}));

vi.mock('../../utils/logger', () => ({
  logger: { error: vi.fn(), warn: vi.fn(), info: vi.fn(), debug: vi.fn() },
}));

// ---------------------------------------------------------------------------
// Static import — vi.mock hoisting ensures mocks are applied
// ---------------------------------------------------------------------------

import {
  useProviderSettingsState,
  type UseProviderSettingsStateReturn,
} from '../hooks/useProviderSettingsState';
import type { ProviderDisplayInfo } from '../components/ProviderSettingsPanel';

// ---------------------------------------------------------------------------
// Hook capture component (same pattern as useModelSelectorState tests)
// ---------------------------------------------------------------------------

let hookState: UseProviderSettingsStateReturn | null = null;

function TestComponent(): React.ReactElement {
  const state = useProviderSettingsState();
  hookState = state;

  return (
    <Text>
      {`loading:${String(state.isLoading)}|index:${state.selectedIndex}|providers:${state.providers.length}|navItems:${state.navItems.length}`}
    </Text>
  );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function findProviderIndex(
  items: { type: string; providerId?: string }[],
  providerId: string
): number {
  return items.findIndex(
    i => i.type === 'provider' && i.providerId === providerId
  );
}

function findOauthStatusIndex(
  items: { type: string; providerId?: string }[],
  providerId: string
): number {
  return items.findIndex(
    i => i.type === 'oauth-status' && i.providerId === providerId
  );
}

/** Wait for hook to finish initial load */
async function waitForLoaded(): Promise<void> {
  await vi.waitFor(() => {
    expect(hookState).not.toBeNull();
    expect(hookState!.isLoading).toBe(false);
  });
}

/**
 * Wait for Ink reconciler to flush all pending state updates from reload().
 * React batches setState calls across microtasks — a small delay ensures
 * the providers state from reload() has been committed before assertions.
 */
async function waitForReloadFlush(): Promise<void> {
  await vi.waitFor(() => {
    expect(hookState!.isLoading).toBe(false);
  });
  await new Promise(r => setTimeout(r, 50));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('Feature: Provider settings expansion state preservation', () => {
  let unmountComponent: (() => void) | null = null;

  const renderWithCleanup = () => {
    const result = render(<TestComponent />);
    unmountComponent = result.unmount;
    return result;
  };

  beforeEach(() => {
    vi.clearAllMocks();
    claudeHasTokens = true;
    hookState = null;
    unmountComponent = null;
  });

  afterEach(() => {
    if (unmountComponent) {
      unmountComponent();
      unmountComponent = null;
    }
    hookState = null;
  });

  describe('Scenario: Expansion state preserved after OAuth disconnect confirmation', () => {
    it('should keep tree expanded and move cursor to provider row after disconnect', async () => {
      // @step Given Anthropic provider is expanded with an OAuth logout row visible
      renderWithCleanup();
      await waitForLoaded();

      hookState!.toggleProviderExpansion('anthropic');

      await vi.waitFor(() => {
        const p = hookState!.providers.find(
          (pv: ProviderDisplayInfo) => pv.id === 'anthropic'
        );
        expect(p?.isExpanded).toBe(true);
        expect(p?.hasOAuthTokens).toBe(true);
      });

      // Verify oauth-status row exists and move cursor to it
      const oauthIdx = findOauthStatusIndex(
        hookState!.navItems,
        'anthropic'
      );
      expect(oauthIdx).toBeGreaterThan(-1);
      hookState!.setSelectedIndex(oauthIdx);

      await vi.waitFor(() => {
        expect(hookState!.selectedIndex).toBe(oauthIdx);
      });

      // @step When the user confirms the OAuth disconnect
      await hookState!.disconnectOauth('anthropic');
      await waitForReloadFlush();

      // @step Then the Anthropic provider tree remains expanded
      await vi.waitFor(() => {
        const afterProvider = hookState!.providers.find(
          (pv: ProviderDisplayInfo) => pv.id === 'anthropic'
        );
        expect(afterProvider?.isExpanded).toBe(true);
      });

      // Expanded children visible (login options still show after disconnect)
      const loginItems = hookState!.navItems.filter(
        (i: { type: string; providerId?: string }) =>
          i.type === 'oauth-login' && i.providerId === 'anthropic'
      );
      expect(loginItems.length).toBe(2);

      // @step Then the selected index points to the Anthropic provider row
      await vi.waitFor(() => {
        const providerIdx = findProviderIndex(
          hookState!.navItems,
          'anthropic'
        );
        expect(providerIdx).toBeGreaterThanOrEqual(0);
        expect(hookState!.selectedIndex).toBe(providerIdx);
      });
    });
  });

  describe('Scenario: Cancel disconnect keeps cursor on logout row', () => {
    it('should keep tree expanded and cursor on logout row after cancel', async () => {
      // @step Given Anthropic provider is expanded with an OAuth logout row visible
      renderWithCleanup();
      await waitForLoaded();

      hookState!.toggleProviderExpansion('anthropic');

      await vi.waitFor(() => {
        const p = hookState!.providers.find(
          (pv: ProviderDisplayInfo) => pv.id === 'anthropic'
        );
        expect(p?.isExpanded).toBe(true);
      });

      const oauthIdx = findOauthStatusIndex(
        hookState!.navItems,
        'anthropic'
      );
      expect(oauthIdx).toBeGreaterThan(-1);

      // Set cursor on the logout row and enter disconnect-oauth mode
      hookState!.setSelectedIndex(oauthIdx);

      await vi.waitFor(() => {
        expect(hookState!.selectedIndex).toBe(oauthIdx);
      });

      hookState!.setMode({
        type: 'disconnect-oauth',
        providerId: 'anthropic',
      });

      // @step When the user cancels the OAuth disconnect confirmation
      hookState!.setMode({ type: 'list' });

      // @step Then the Anthropic provider tree remains expanded
      await vi.waitFor(() => {
        const provider = hookState!.providers.find(
          (pv: ProviderDisplayInfo) => pv.id === 'anthropic'
        );
        expect(provider?.isExpanded).toBe(true);
      });

      const loginItems = hookState!.navItems.filter(
        (i: { type: string; providerId?: string }) =>
          i.type === 'oauth-login' && i.providerId === 'anthropic'
      );
      expect(loginItems.length).toBe(2);

      // @step Then the cursor remains on the OAuth logout row
      // No reload happened — navItems and selectedIndex unchanged
      await vi.waitFor(() => {
        expect(hookState!.selectedIndex).toBe(oauthIdx);
      });
      const stillExists = findOauthStatusIndex(
        hookState!.navItems,
        'anthropic'
      );
      expect(stillExists).toBe(oauthIdx);
    });
  });

  describe('Scenario: Expansion state preserved after API key deletion', () => {
    it('should keep tree expanded and move cursor to provider row after API key delete', async () => {
      // @step Given a provider is expanded with an API key configured
      renderWithCleanup();
      await waitForLoaded();

      hookState!.toggleProviderExpansion('gemini');

      await vi.waitFor(() => {
        const gp = hookState!.providers.find(
          (pv: ProviderDisplayInfo) => pv.id === 'gemini'
        );
        expect(gp?.isExpanded).toBe(true);
      });

      // Should have an api-key row visible
      const apiKeyIdx = hookState!.navItems.findIndex(
        (i: { type: string; providerId?: string }) =>
          i.type === 'api-key' && i.providerId === 'gemini'
      );
      expect(apiKeyIdx).toBeGreaterThan(-1);

      hookState!.setSelectedIndex(apiKeyIdx);

      await vi.waitFor(() => {
        expect(hookState!.selectedIndex).toBe(apiKeyIdx);
      });

      // @step When the user confirms the API key deletion
      await hookState!.removeApiKey('gemini');
      await waitForReloadFlush();

      // @step Then the provider tree remains expanded
      await vi.waitFor(() => {
        const afterProvider = hookState!.providers.find(
          (pv: ProviderDisplayInfo) => pv.id === 'gemini'
        );
        expect(afterProvider?.isExpanded).toBe(true);
      });

      // api-key row still visible (provider expanded)
      const apiKeyStillVisible = hookState!.navItems.some(
        (i: { type: string; providerId?: string }) =>
          i.type === 'api-key' && i.providerId === 'gemini'
      );
      expect(apiKeyStillVisible).toBe(true);

      // @step Then the selected index points to the provider row
      await vi.waitFor(() => {
        const providerIdx = findProviderIndex(
          hookState!.navItems,
          'gemini'
        );
        expect(providerIdx).toBeGreaterThanOrEqual(0);
        expect(hookState!.selectedIndex).toBe(providerIdx);
      });
    });
  });

  describe('Regression: reload without PROV-036 fix collapses expansion', () => {
    it('should survive multiple reload cycles with expansion intact', async () => {
      renderWithCleanup();
      await waitForLoaded();

      hookState!.toggleProviderExpansion('anthropic');

      await vi.waitFor(() => {
        expect(
          hookState!.providers.find(
            (pv: ProviderDisplayInfo) => pv.id === 'anthropic'
          )?.isExpanded
        ).toBe(true);
      });

      // First reload — expansion must survive
      await hookState!.reload();
      await waitForReloadFlush();

      await vi.waitFor(() => {
        expect(
          hookState!.providers.find(
            (pv: ProviderDisplayInfo) => pv.id === 'anthropic'
          )?.isExpanded
        ).toBe(true);
      });

      // Second reload — still expanded (ref is durable)
      await hookState!.reload();
      await waitForReloadFlush();

      await vi.waitFor(() => {
        expect(
          hookState!.providers.find(
            (pv: ProviderDisplayInfo) => pv.id === 'anthropic'
          )?.isExpanded
        ).toBe(true);
      });

      // Children are visible
      const loginItems = hookState!.navItems.filter(
        (i: { type: string; providerId?: string }) =>
          i.type === 'oauth-login' && i.providerId === 'anthropic'
      );
      expect(loginItems.length).toBe(2);
    });
  });

  describe('Regression: toggle collapse then reload respects collapse', () => {
    it('should keep provider collapsed after toggle-off + reload', async () => {
      renderWithCleanup();
      await waitForLoaded();

      // Expand then collapse
      hookState!.toggleProviderExpansion('anthropic');

      await vi.waitFor(() => {
        expect(
          hookState!.providers.find(
            (pv: ProviderDisplayInfo) => pv.id === 'anthropic'
          )?.isExpanded
        ).toBe(true);
      });

      hookState!.toggleProviderExpansion('anthropic');

      await vi.waitFor(() => {
        expect(
          hookState!.providers.find(
            (pv: ProviderDisplayInfo) => pv.id === 'anthropic'
          )?.isExpanded
        ).toBe(false);
      });

      // Reload — should stay collapsed
      await hookState!.reload();
      await waitForReloadFlush();

      await vi.waitFor(() => {
        expect(
          hookState!.providers.find(
            (pv: ProviderDisplayInfo) => pv.id === 'anthropic'
          )?.isExpanded
        ).toBe(false);
      });

      // No children visible
      const childItems = hookState!.navItems.filter(
        (i: { type: string; providerId?: string }) =>
          i.providerId === 'anthropic' && i.type !== 'provider'
      );
      expect(childItems.length).toBe(0);
    });
  });
});
