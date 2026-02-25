/**
 * ProviderSettingsScreen Integration Test Fixture
 *
 * TUI-074: Full integration test fixture for ProviderSettingsScreen component.
 *
 * This fixture COMPOSES the HomeDirectoryFixture and adds:
 * - NAPI network boundary mocking only (testProviderConnection, modelsListLocalOpenai)
 * - Callback tracking for assertions
 * - Proper ink-testing-library integration
 *
 * SOLID: Single Responsibility - ProviderSettingsScreen test setup only
 * DRY: Composes HomeDirectoryFixture instead of duplicating HOME directory logic
 * COMPOSABLE: Can be extended for specific test scenarios
 */

import { vi } from 'vitest';

import {
  createHomeDirectoryFixture,
  type HomeDirectoryFixture,
  type HomeDirectoryEnv,
} from '../../../../test-helpers/home-directory-fixture';
import type { ProfileConfig } from '../../../../utils/provider-config';

// =============================================================================
// TYPES
// =============================================================================

/**
 * Mock NAPI response configuration
 */
export interface NapiMockConfig {
  /** Test connection results by provider/profile */
  connectionResults: Map<string, { success: boolean; error?: string }>;
  /** Local models by baseUrl (for modelsListLocalOpenai) */
  localServerModels: Map<string, string[] | Error>;
}

/**
 * Callback tracking for assertions
 */
export interface CallbackTracker {
  onClose: {
    calls: number;
    mock: ReturnType<typeof vi.fn>;
  };
  onSwitchToModels: {
    calls: number;
    mock: ReturnType<typeof vi.fn>;
  };
}

/**
 * ProviderSettingsScreen fixture (composes HomeDirectoryFixture)
 */
export interface ProviderSettingsScreenFixture {
  /** Test environment (HOME, config paths) - from HomeDirectoryFixture */
  env: HomeDirectoryEnv;

  /** NAPI mock configuration */
  napiConfig: NapiMockConfig;

  /** Callback tracker for assertions */
  callbacks: CallbackTracker;

  // ---- Delegated from HomeDirectoryFixture ----

  /**
   * Create a provider profile in the config file
   */
  createProfile: (
    providerId: string,
    profileName: string,
    config: ProfileConfig
  ) => Promise<void>;

  /**
   * Create credentials for a provider
   */
  createCredential: (providerId: string, apiKey: string) => Promise<void>;

  // ---- NAPI Mock Configuration ----

  /**
   * Configure connection test result
   */
  setConnectionResult: (
    providerId: string,
    profileName: string | undefined,
    success: boolean,
    error?: string
  ) => void;

  /**
   * Set local server models (or error)
   */
  setLocalServerModels: (baseUrl: string, models: string[] | Error) => void;

  // ---- NAPI Mocks ----

  /** Mock for testProviderConnection */
  testProviderConnectionMock: ReturnType<typeof vi.fn>;

  /** Mock for modelsListLocalOpenai */
  modelsListLocalOpenaiMock: ReturnType<typeof vi.fn>;

  // ---- Lifecycle ----

  /** Reset fixture state between tests */
  reset: () => Promise<void>;

  /** Reset callbacks only */
  resetCallbacks: () => void;

  /** Clean up temp directories and restore HOME */
  cleanup: () => Promise<void>;

  /** Wait for providers to load */
  waitForProvidersLoaded: (timeout?: number) => Promise<void>;
}

// =============================================================================
// CALLBACK TRACKER FACTORY
// =============================================================================

/**
 * Creates a fresh callback tracker with mocks
 */
function createCallbackTracker(): CallbackTracker {
  const tracker: CallbackTracker = {
    onClose: {
      calls: 0,
      mock: vi.fn(),
    },
    onSwitchToModels: {
      calls: 0,
      mock: vi.fn(),
    },
  };

  // Connect mocks to call counters
  tracker.onClose.mock.mockImplementation(() => {
    tracker.onClose.calls++;
  });
  tracker.onSwitchToModels.mock.mockImplementation(() => {
    tracker.onSwitchToModels.calls++;
  });

  return tracker;
}

// =============================================================================
// FIXTURE FACTORY
// =============================================================================

/**
 * Creates a ProviderSettingsScreen fixture for integration testing.
 *
 * This fixture:
 * - Composes HomeDirectoryFixture for real file system operations
 * - Provides NAPI mock functions for network boundary ONLY
 * - Uses REAL useProviderSettingsState hook
 * - Uses REAL ProviderSettingsScreen component
 * - Cleans up on teardown
 *
 * @example
 * ```typescript
 * describe('ProviderSettingsScreen Integration', () => {
 *   let fixture: ProviderSettingsScreenFixture;
 *
 *   beforeEach(async () => {
 *     fixture = await createProviderSettingsScreenFixture('my-test');
 *   });
 *
 *   afterEach(async () => {
 *     await fixture.cleanup();
 *   });
 *
 *   it('should use real hook with real config files', async () => {
 *     await fixture.createCredential('anthropic', 'test-key');
 *     // render and test...
 *   });
 * });
 * ```
 */
export async function createProviderSettingsScreenFixture(
  testName: string
): Promise<ProviderSettingsScreenFixture> {
  // ========================================
  // Compose HomeDirectoryFixture
  // ========================================

  const homeFixture = await createHomeDirectoryFixture({
    testName,
    dirPrefix: 'fspec-provider-screen',
  });

  // ========================================
  // NAPI Mock Configuration
  // ========================================

  const napiConfig: NapiMockConfig = {
    connectionResults: new Map(),
    localServerModels: new Map(),
  };

  // Create mock functions
  const testProviderConnectionMock = vi.fn();
  const modelsListLocalOpenaiMock = vi.fn();

  // Configure mock implementations
  const updateMockImplementations = () => {
    testProviderConnectionMock.mockImplementation(
      async (providerId: string) => {
        const key = providerId;
        const result = napiConfig.connectionResults.get(key);
        if (result) {
          return result;
        }
        // Default: success if credentials exist
        return { success: true };
      }
    );

    modelsListLocalOpenaiMock.mockImplementation(async (baseUrl: string) => {
      const result = napiConfig.localServerModels.get(baseUrl);
      if (result instanceof Error) {
        throw result;
      }
      return result || [];
    });
  };

  updateMockImplementations();

  // ========================================
  // Callback Tracker
  // ========================================

  let callbacks = createCallbackTracker();

  // ========================================
  // NAPI Configuration Helpers
  // ========================================

  const setConnectionResult = (
    providerId: string,
    profileName: string | undefined,
    success: boolean,
    error?: string
  ): void => {
    const key = profileName ? `${providerId}:${profileName}` : providerId;
    napiConfig.connectionResults.set(key, { success, error });
    updateMockImplementations();
  };

  const setLocalServerModels = (
    baseUrl: string,
    models: string[] | Error
  ): void => {
    napiConfig.localServerModels.set(baseUrl, models);
    updateMockImplementations();
  };

  // ========================================
  // Lifecycle
  // ========================================

  const resetCallbacks = (): void => {
    callbacks = createCallbackTracker();
  };

  const reset = async (): Promise<void> => {
    // Reset HOME directory fixture
    await homeFixture.reset();

    // Reset NAPI config
    napiConfig.connectionResults.clear();
    napiConfig.localServerModels.clear();

    // Clear mocks
    testProviderConnectionMock.mockClear();
    modelsListLocalOpenaiMock.mockClear();

    updateMockImplementations();

    // Reset callbacks
    resetCallbacks();
  };

  const cleanup = async (): Promise<void> => {
    await homeFixture.cleanup();
  };

  const waitForProvidersLoaded = async (_timeout = 2000): Promise<void> => {
    // Wait for the hook's useEffect to complete initial load
    await new Promise(resolve => setTimeout(resolve, 100));
  };

  return {
    // Delegate from HomeDirectoryFixture
    env: homeFixture.env,
    createProfile: homeFixture.createProfile,
    createCredential: homeFixture.createCredential,

    // ProviderSettingsScreen-specific
    napiConfig,
    callbacks,
    setConnectionResult,
    setLocalServerModels,
    testProviderConnectionMock,
    modelsListLocalOpenaiMock,
    reset,
    resetCallbacks,
    cleanup,
    waitForProvidersLoaded,
  };
}
