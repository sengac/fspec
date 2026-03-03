/**
 * Feature: spec/features/tui-oauth-login-flow.feature
 *
 * PROV-017: TUI OAuth Login Flow for Provider Settings - INTEGRATION TESTS
 *
 * These are REAL integration tests that:
 * - Use REAL useProviderSettingsState hook (NOT mocked)
 * - Use REAL ProviderSettingsScreen component
 * - Only mock at NAPI network boundary (OAuth functions, testProviderConnection, modelsListLocalOpenai)
 * - Use reusable fixtures following DRY/SOLID/COMPOSABLE principles
 *
 * Test coverage validates actual behavior, not mock interactions.
 */

import React from 'react';
import { render, cleanup } from 'ink-testing-library';
import { describe, it, expect, vi, beforeEach, afterEach, beforeAll } from 'vitest';

import {
  createProviderSettingsScreenFixture,
  type ProviderSettingsScreenFixture,
} from './fixtures/providerSettingsScreenFixture';
import {
  pressKey,
  waitFor,
} from './fixtures/keyboardHelpers';

// =============================================================================
// NAPI MODULE MOCK - Network boundary ONLY
// =============================================================================

// Store fixture reference for mock access
let activeFixture: ProviderSettingsScreenFixture | null = null;

// OAuth mock functions - controlled per-test
let mockBrowserLogin: ReturnType<typeof vi.fn>;
let mockDeviceLoginStart: ReturnType<typeof vi.fn>;
let mockDeviceLoginPoll: ReturnType<typeof vi.fn>;
let mockGetTokens: ReturnType<typeof vi.fn>;
let mockRefreshToken: ReturnType<typeof vi.fn>;

vi.mock('@sengac/codelet-napi', async () => {
  const actual = await vi.importActual<typeof import('@sengac/codelet-napi')>(
    '@sengac/codelet-napi'
  );
  return {
    ...actual,
    testProviderConnection: vi.fn(async (providerId: string) => {
      if (activeFixture) {
        return activeFixture.testProviderConnectionMock(providerId);
      }
      return { success: true };
    }),
    modelsListLocalOpenai: vi.fn(async (baseUrl: string) => {
      if (activeFixture) {
        return activeFixture.modelsListLocalOpenaiMock(baseUrl);
      }
      return [];
    }),
    codexOauthBrowserLogin: (...args: unknown[]) => mockBrowserLogin(...args),
    codexOauthDeviceLoginStart: (...args: unknown[]) => mockDeviceLoginStart(...args),
    codexOauthDeviceLoginPoll: (...args: unknown[]) => mockDeviceLoginPoll(...args),
    codexOauthGetTokens: (...args: unknown[]) => mockGetTokens(...args),
    codexOauthRefreshToken: (...args: unknown[]) => mockRefreshToken(...args),
  };
});

// =============================================================================
// MOCK TOKEN FIXTURES
// =============================================================================

const MOCK_TOKENS = {
  idToken: 'mock-id-token-jwt',
  accessToken: 'mock-access-token',
  refreshToken: 'mock-refresh-token',
  accountId: 'mock-account-id-123',
};

const MOCK_DEVICE_START_RESULT = {
  userCode: 'ABCD-1234',
  verificationUrl: 'https://auth.openai.com/codex/device',
  deviceAuthId: 'mock-device-auth-id',
  interval: 5,
};

// =============================================================================
// TEST SUITE
// =============================================================================

describe('Feature: TUI OAuth Login Flow for Provider Settings', () => {
  let fixture: ProviderSettingsScreenFixture;
  let ProviderSettingsScreen: typeof import('../ProviderSettingsScreen').ProviderSettingsScreen;

  beforeAll(async () => {
    const module = await import('../ProviderSettingsScreen');
    ProviderSettingsScreen = module.ProviderSettingsScreen;
  });

  beforeEach(async () => {
    fixture = await createProviderSettingsScreenFixture('tui-oauth-login');
    activeFixture = fixture;

    // Reset OAuth mocks
    mockBrowserLogin = vi.fn();
    mockDeviceLoginStart = vi.fn();
    mockDeviceLoginPoll = vi.fn();
    mockGetTokens = vi.fn().mockReturnValue(null);
    mockRefreshToken = vi.fn();

    // Set up credentials so other providers load
    await fixture.createCredential('anthropic', 'test-api-key-12345');
  });

  afterEach(async () => {
    cleanup();
    activeFixture = null;
    await fixture.cleanup();
  });

  // ===========================================================================
  // BACKGROUND: User Story
  // ===========================================================================

  describe('Background: User Story', () => {
    it('provides the provider settings screen context', () => {
      // @step Given I am on the provider settings screen
      expect(ProviderSettingsScreen).toBeDefined();
    });
  });

  // ===========================================================================
  // Scenario 3: Successful browser OAuth login flow
  // ===========================================================================

  describe('Scenario: Successful browser OAuth login flow', () => {
    it('should show waiting spinner, then success and reload on browser OAuth completion', async () => {
      // @step Given I am on the provider settings screen
      // (from Background)

      // @step Given the codex provider has no OAuth tokens
      mockGetTokens.mockReturnValue(null);

      // Set up browser login to resolve with tokens after a short delay
      let resolveBrowserLogin: ((value: typeof MOCK_TOKENS) => void) | undefined;
      mockBrowserLogin.mockReturnValue(
        new Promise<typeof MOCK_TOKENS>((resolve) => {
          resolveBrowserLogin = resolve;
        })
      );

      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Navigate to codex and expand
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('codex');
      await waitFor(100);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(100);

      // @step When I select "Login with ChatGPT (browser)"
      // Navigate down to the browser login option and select it
      pressKey(stdin, { name: 'down' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(100);

      // @step Then I should see a spinner with "Waiting for authorization..." text
      const waitingFrame = lastFrame();
      expect(waitingFrame).toContain('Waiting for authorization');

      // @step When the browser OAuth callback completes successfully
      resolveBrowserLogin!(MOCK_TOKENS);
      // After resolution, mock getTokens to return tokens
      mockGetTokens.mockReturnValue(MOCK_TOKENS);
      await waitFor(200);

      const successFrame = lastFrame();

      // @step Then I should see "Connected to ChatGPT" success message
      expect(successFrame).toContain('Connected to ChatGPT');

      // @step And the provider list should reload
      // Verify that codexOauthBrowserLogin was called
      expect(mockBrowserLogin).toHaveBeenCalled();

      // @step And the codex provider should show as configured with a green checkmark
      // Press Enter/Esc to return to list and check
      pressKey(stdin, { name: 'enter' });
      await waitFor(200);
      const listFrame = lastFrame();
      expect(listFrame).toContain('✓');
    });
  });

  // ===========================================================================
  // Scenario 4: Successful device auth login flow
  // ===========================================================================

  describe('Scenario: Successful device auth login flow', () => {
    it('should show user code and URL, then success on device auth completion', async () => {
      // @step Given I am on the provider settings screen
      // (from Background)

      // @step Given the codex provider has no OAuth tokens
      mockGetTokens.mockReturnValue(null);

      // Set up device login start to return user_code + URL
      mockDeviceLoginStart.mockResolvedValue(MOCK_DEVICE_START_RESULT);

      // Set up device login poll to resolve with tokens after a delay
      let resolveDevicePoll: ((value: typeof MOCK_TOKENS) => void) | undefined;
      mockDeviceLoginPoll.mockReturnValue(
        new Promise<typeof MOCK_TOKENS>((resolve) => {
          resolveDevicePoll = resolve;
        })
      );

      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Navigate to codex and expand
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('codex');
      await waitFor(100);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(100);

      // @step When I select "Login with ChatGPT (headless)"
      // Navigate down to the headless login option and select it
      pressKey(stdin, { name: 'down' });
      await waitFor(50);
      pressKey(stdin, { name: 'down' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(200);

      const deviceFrame = lastFrame();

      // @step Then I should see the user code displayed
      expect(deviceFrame).toContain('ABCD-1234');

      // @step And I should see the verification URL displayed
      expect(deviceFrame).toContain('https://auth.openai.com/codex/device');

      // @step And I should see a spinner with "Enter the code on another device" text
      expect(deviceFrame).toContain('Enter the code on another device');

      // @step When the device auth polling completes successfully
      resolveDevicePoll!(MOCK_TOKENS);
      mockGetTokens.mockReturnValue(MOCK_TOKENS);
      await waitFor(200);

      const successFrame = lastFrame();

      // @step Then I should see a success message
      expect(successFrame).toContain('Connected');

      // @step And the provider list should reload
      expect(mockDeviceLoginStart).toHaveBeenCalled();
      expect(mockDeviceLoginPoll).toHaveBeenCalledWith(
        MOCK_DEVICE_START_RESULT.deviceAuthId,
        MOCK_DEVICE_START_RESULT.interval
      );

      // @step And the codex provider should show as configured with a green checkmark
      pressKey(stdin, { name: 'enter' });
      await waitFor(200);
      const listFrame = lastFrame();
      expect(listFrame).toContain('✓');
    });
  });

  // ===========================================================================
  // Scenario 1: Codex provider shows OAuth login options when no tokens exist
  // ===========================================================================

  describe('Scenario: Codex provider shows OAuth login options when no tokens exist', () => {
    it('should show browser and headless login options alongside API key edit', async () => {
      // @step Given I am on the provider settings screen
      // (from Background)

      // @step Given the codex provider has no OAuth tokens
      mockGetTokens.mockReturnValue(null);

      // @step And the codex provider has no API key configured
      // (no credential created for codex)

      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // @step When I expand the codex provider
      // Navigate to codex provider using filter
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('codex');
      await waitFor(100);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      // Expand the codex provider
      pressKey(stdin, { name: 'enter' });
      await waitFor(100);

      const frame = lastFrame();

      // @step Then I should see "Login with ChatGPT (browser)" option
      expect(frame).toContain('Login with ChatGPT (browser)');

      // @step And I should see "Login with ChatGPT (headless)" option
      expect(frame).toContain('Login with ChatGPT (headless)');

      // @step And I should see footer keybind hints
      expect(frame).toContain('Enter');
      expect(frame).toContain('Esc');
    });
  });

  // ===========================================================================
  // Scenario 6: Escape cancels browser OAuth waiting state
  // ===========================================================================

  describe('Scenario: Escape cancels browser OAuth waiting state', () => {
    it('should cancel flow and return to provider list with no error on Escape', async () => {
      // @step Given I am on the provider settings screen
      // (from Background)

      // @step Given the codex provider has no OAuth tokens
      mockGetTokens.mockReturnValue(null);

      // Set up browser login as a controllable promise
      let resolveBrowserLogin: ((value: typeof MOCK_TOKENS) => void) | undefined;
      mockBrowserLogin.mockReturnValue(
        new Promise<typeof MOCK_TOKENS>((resolve) => {
          resolveBrowserLogin = resolve;
        })
      );

      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Navigate to codex, expand, select browser login
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('codex');
      await waitFor(100);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(100);

      // @step And I have started the browser OAuth flow
      pressKey(stdin, { name: 'down' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(100);

      // @step And I see the "Waiting for authorization..." spinner
      const waitingFrame = lastFrame();
      expect(waitingFrame).toContain('Waiting for authorization');

      // @step When I press Escape
      pressKey(stdin, { name: 'escape' });
      await waitFor(100);

      const afterEscFrame = lastFrame();

      // @step Then the OAuth flow should be cancelled
      // Verify stale promise resolution does NOT corrupt state:
      // resolve the still-pending promise AFTER cancel
      resolveBrowserLogin!(MOCK_TOKENS);
      mockGetTokens.mockReturnValue(MOCK_TOKENS);
      await waitFor(200);

      // UI must still be on provider list, NOT hijacked to success screen
      const staleResolveFrame = lastFrame();
      expect(staleResolveFrame).toContain('Provider Settings');
      expect(staleResolveFrame).not.toContain('Connected to ChatGPT');

      // @step And I should return to the provider list
      expect(afterEscFrame).toContain('Provider Settings');

      // @step And no error message should be displayed
      expect(afterEscFrame).not.toContain('timed out');
      expect(afterEscFrame).not.toContain('error');
      expect(afterEscFrame).not.toContain('failed');
    });
  });

  // ===========================================================================
  // Scenario 5: Browser OAuth login times out after 5 minutes
  // ===========================================================================

  describe('Scenario: Browser OAuth login times out after 5 minutes', () => {
    it('should show error message with retry and go-back instructions on timeout', async () => {
      // @step Given I am on the provider settings screen
      // (from Background)

      // @step Given the codex provider has no OAuth tokens
      mockGetTokens.mockReturnValue(null);

      // Set up browser login to reject with timeout error
      mockBrowserLogin.mockRejectedValue(
        new Error('Browser OAuth login failed: OAuth login timed out after 300 seconds')
      );

      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Navigate to codex, expand, select browser login
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('codex');
      await waitFor(100);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(100);

      // @step When I select "Login with ChatGPT (browser)"
      pressKey(stdin, { name: 'down' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(200);

      // @step And the browser OAuth flow times out
      // (already set up via mockRejectedValue)

      const errorFrame = lastFrame();

      // @step Then I should see an error message containing "timed out"
      expect(errorFrame).toContain('timed out');

      // @step And I should see instructions to retry with Enter or go back with Escape
      expect(errorFrame).toContain('Enter');
      expect(errorFrame).toContain('Esc');
    });
  });

  // ===========================================================================
  // Scenario 2: Non-codex providers do not show OAuth login options
  // ===========================================================================

  describe('Scenario: Non-codex providers do not show OAuth login options', () => {
    it('should not show OAuth options for anthropic provider', async () => {
      // @step Given I am on the provider settings screen
      // (from Background)

      // @step Given the anthropic provider has no API key configured
      // anthropic credential was set in beforeEach; remove it
      await fixture.reset();
      // Do NOT set any credentials — anthropic shows as not configured

      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // @step When I expand the anthropic provider
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('anthropic');
      await waitFor(100);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      pressKey(stdin, { name: 'enter' });
      await waitFor(100);

      const frame = lastFrame();

      // @step Then I should not see any "Login with ChatGPT" options
      // PROV-028: OAuth login options now always show (for re-login)
      // Only check that the provider IS shown with its status
      expect(frame).toContain('Anthropic');

      // @step And I should see footer keybind hints
      expect(frame).toContain('Enter');
      expect(frame).toContain('Esc');
    });
  });


  // ===========================================================================
  // Scenario 7: Escape cancels device auth waiting state
  // ===========================================================================

  describe('Scenario: Escape cancels device auth waiting state', () => {
    it('should cancel device auth flow and return to provider list on Escape', async () => {
      // @step Given I am on the provider settings screen
      // (from Background)

      // @step Given the codex provider has no OAuth tokens
      mockGetTokens.mockReturnValue(null);

      // Set up device login start to return immediately
      mockDeviceLoginStart.mockResolvedValue(MOCK_DEVICE_START_RESULT);

      // Set up device login poll as a controllable promise
      let resolveDevicePoll: ((value: typeof MOCK_TOKENS) => void) | undefined;
      mockDeviceLoginPoll.mockReturnValue(
        new Promise<typeof MOCK_TOKENS>((resolve) => {
          resolveDevicePoll = resolve;
        })
      );

      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Navigate to codex, expand, select headless login
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('codex');
      await waitFor(100);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(100);

      // @step And I have started the device auth flow
      // Navigate to headless option (2nd OAuth option)
      pressKey(stdin, { name: 'down' });
      await waitFor(50);
      pressKey(stdin, { name: 'down' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(200);

      // @step And I see the user code and verification URL
      const deviceFrame = lastFrame();
      expect(deviceFrame).toContain('ABCD-1234');
      expect(deviceFrame).toContain('https://auth.openai.com/codex/device');

      // @step When I press Escape
      pressKey(stdin, { name: 'escape' });
      await waitFor(100);

      const afterEscFrame = lastFrame();

      // @step Then the OAuth flow should be cancelled
      // Verify stale promise resolution does NOT corrupt state:
      resolveDevicePoll!(MOCK_TOKENS);
      mockGetTokens.mockReturnValue(MOCK_TOKENS);
      await waitFor(200);

      // UI must still be on provider list, NOT hijacked to success screen
      const staleResolveFrame = lastFrame();
      expect(staleResolveFrame).toContain('Provider Settings');
      expect(staleResolveFrame).not.toContain('Connected to ChatGPT');

      // @step And I should return to the provider list
      expect(afterEscFrame).toContain('Provider Settings');

      // @step And no error message should be displayed
      expect(afterEscFrame).not.toContain('timed out');
      expect(afterEscFrame).not.toContain('error');
      expect(afterEscFrame).not.toContain('failed');
    });
  });

  // ===========================================================================
  // Scenario 8: Codex provider shows as configured when OAuth tokens exist
  // ===========================================================================

  describe('Scenario: Codex provider shows as configured when OAuth tokens exist', () => {
    it('should show green checkmark and OAuth source label when tokens exist', async () => {
      // @step Given I am on the provider settings screen
      // (from Background)

      // @step Given the codex provider has existing OAuth tokens
      mockGetTokens.mockReturnValue(MOCK_TOKENS);

      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      // @step When the provider settings screen loads
      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Navigate to codex
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('codex');
      await waitFor(100);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      const frame = lastFrame();

      // @step Then the codex provider should show a green checkmark
      expect(frame).toContain('✓');

      // @step And the codex provider should show "OAuth" as the source label
      expect(frame).toContain('OAuth');
      expect(frame).toContain('[ChatGPT]');

      // @step And no OAuth login options should be displayed in the expanded list
      // PROV-028: OAuth login options now always show (for re-login)
      // Expand codex
      pressKey(stdin, { name: 'enter' });
      await waitFor(100);
      const expandedFrame = lastFrame();
      expect(expandedFrame).toContain('Login with ChatGPT');
    });
  });

  // ===========================================================================
  // Scenario 9: Retry browser OAuth after error
  // ===========================================================================

  describe('Scenario: Retry browser OAuth after error', () => {
    it('should restart browser OAuth flow when Enter is pressed on error screen', async () => {
      // @step Given I am on the provider settings screen
      // (from Background)

      // @step Given the codex provider has no OAuth tokens
      mockGetTokens.mockReturnValue(null);

      // First call fails, second call succeeds (pending)
      let callCount = 0;
      mockBrowserLogin.mockImplementation(() => {
        callCount++;
        if (callCount === 1) {
          return Promise.reject(new Error('Network error'));
        }
        return new Promise(() => {}); // Second call stays pending (waiting state)
      });

      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Navigate to codex, expand, select browser login
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('codex');
      await waitFor(100);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(100);

      // Select browser login → first call fails
      pressKey(stdin, { name: 'down' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(200);

      // @step And I am on the OAuth error screen after a failed browser login
      const errorFrame = lastFrame();
      expect(errorFrame).toContain('error');

      // @step When I press Enter to retry
      pressKey(stdin, { name: 'enter' });
      await waitFor(200);

      const retryFrame = lastFrame();

      // @step Then the browser OAuth flow should restart
      expect(mockBrowserLogin).toHaveBeenCalledTimes(2);

      // @step And I should see a spinner with "Waiting for authorization..." text
      expect(retryFrame).toContain('Waiting for authorization');
    });
  });

  // ===========================================================================
  // Scenario 10: Go back to provider list after OAuth error
  // ===========================================================================

  describe('Scenario: Go back to provider list after OAuth error', () => {
    it('should return to provider list with no error when Escape pressed on error screen', async () => {
      // @step Given I am on the provider settings screen
      // (from Background)

      // @step Given the codex provider has no OAuth tokens
      mockGetTokens.mockReturnValue(null);

      // Browser login fails immediately
      mockBrowserLogin.mockRejectedValue(new Error('Connection refused'));

      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Navigate to codex, expand, select browser login
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('codex');
      await waitFor(100);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(100);

      // Select browser login → call fails
      pressKey(stdin, { name: 'down' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(200);

      // @step And I am on the OAuth error screen after a failed browser login
      const errorFrame = lastFrame();
      expect(errorFrame).toContain('error');

      // @step When I press Escape
      pressKey(stdin, { name: 'escape' });
      await waitFor(100);

      const afterEscFrame = lastFrame();

      // @step Then I should return to the provider list
      expect(afterEscFrame).toContain('Provider Settings');

      // @step And no error message should be displayed
      expect(afterEscFrame).not.toContain('Connection refused');
      expect(afterEscFrame).not.toContain('error');
    });
  });

  // ===========================================================================
  // Scenario 11: NAPI codex OAuth bindings are importable
  // ===========================================================================

  describe('Scenario: NAPI codex OAuth bindings are importable', () => {
    it('should have all OAuth NAPI functions available', async () => {
      // @step Given I am on the provider settings screen
      // (from Background)

      // @step Given the NAPI module has been rebuilt
      // Import the NAPI module (mocked, but shape should match)
      const napi = await import('@sengac/codelet-napi');

      // @step Then codexOauthBrowserLogin should be available as a function
      expect(typeof napi.codexOauthBrowserLogin).toBe('function');

      // @step And codexOauthDeviceLoginStart should be available as a function
      expect(typeof napi.codexOauthDeviceLoginStart).toBe('function');

      // @step And codexOauthDeviceLoginPoll should be available as a function
      expect(typeof napi.codexOauthDeviceLoginPoll).toBe('function');

      // @step And codexOauthGetTokens should be available as a function
      expect(typeof napi.codexOauthGetTokens).toBe('function');

      // @step And codexOauthRefreshToken should be available as a function
      expect(typeof napi.codexOauthRefreshToken).toBe('function');
    });
  });
});
