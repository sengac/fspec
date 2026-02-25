/**
 * ModelSelectorScreen Integration Test Fixture
 *
 * TUI-073: Full integration test fixture for ModelSelectorScreen component.
 *
 * This fixture COMPOSES the HomeDirectoryFixture and adds:
 * - NAPI network boundary mocking only (models.dev, local servers)
 * - Callback tracking for model selection and screen navigation
 * - Proper ink-testing-library integration
 *
 * SOLID: Single Responsibility - ModelSelectorScreen test setup only
 * DRY: Composes HomeDirectoryFixture instead of duplicating HOME directory logic
 * COMPOSABLE: Can be extended for specific test scenarios
 */

import { vi } from 'vitest';

import {
  createHomeDirectoryFixture,
  type HomeDirectoryEnv,
} from '../../../../test-helpers/home-directory-fixture';
import type { NapiProviderModels } from '@sengac/codelet-napi';
import type { ProfileConfig } from '../../../../utils/provider-config';
import type { ModelSelection } from '../../../types/provider';
import { useModelStore } from '../../../store/modelStore';

// Import from single source of truth for NAPI model data
import {
  createNapiModelInfo,
  createAnthropicNapiModels,
  createOpenAiNapiModels,
  createDefaultCloudProviders,
} from '../../../../test-helpers/napi-model-fixtures';

// Re-export NAPI builders for fixture consumers
export {
  createNapiModelInfo,
  createAnthropicNapiModels,
  createOpenAiNapiModels,
  createDefaultCloudProviders,
} from '../../../../test-helpers/napi-model-fixtures';

// Re-export from provider-type-fixtures for convenience (UI-level fixtures)
export {
  createAnthropicSection,
  createOpenAiSection,
  createLocalProfileSection,
  createTestProviderSection,
  createClaudeModel,
  createGptModel,
  createLocalModel,
  createTestModelSelection,
  createClaudeSelection,
  createMultiProviderScenario,
} from '../../../../test-helpers/provider-type-fixtures';

// =============================================================================
// TYPES
// =============================================================================

/**
 * Mock NAPI response configuration
 */
export interface NapiMockConfig {
  /** Cloud providers response from modelsListAll */
  cloudProviders: NapiProviderModels[];
  /** Local models by baseUrl (for modelsListLocalOpenai) */
  localServerModels: Map<string, string[] | Error>;
  /** Whether modelsRefreshCache should succeed */
  refreshSuccess: boolean;
}

/**
 * Callback tracking for assertions
 */
export interface CallbackTracker {
  onSelectModel: {
    calls: ModelSelection[];
    mock: ReturnType<typeof vi.fn>;
  };
  onClose: {
    calls: number;
    mock: ReturnType<typeof vi.fn>;
  };
  onSwitchToSettings: {
    calls: number;
    mock: ReturnType<typeof vi.fn>;
  };
}

/**
 * ModelSelectorScreen fixture (composes HomeDirectoryFixture)
 */
export interface ModelSelectorScreenFixture {
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

  // ---- NAPI Configuration ----

  /**
   * Configure NAPI mock responses
   */
  configureNapi: (config: Partial<NapiMockConfig>) => void;

  /**
   * Set local server models (or error)
   */
  setLocalServerModels: (baseUrl: string, models: string[] | Error) => void;

  // ---- NAPI Mocks ----

  /** Mock for modelsListAll */
  modelsListAllMock: ReturnType<typeof vi.fn>;

  /** Mock for modelsListLocalOpenai */
  modelsListLocalOpenaiMock: ReturnType<typeof vi.fn>;

  /** Mock for modelsRefreshCache */
  modelsRefreshCacheMock: ReturnType<typeof vi.fn>;

  // ---- Lifecycle ----

  /** Reset fixture state between tests */
  reset: () => Promise<void>;

  /** Reset callbacks only */
  resetCallbacks: () => void;

  /** Clean up temp directories and restore HOME */
  cleanup: () => Promise<void>;

  /** Wait for models to load */
  waitForModelsLoaded: (timeout?: number) => Promise<void>;
}

// =============================================================================
// NAPI CONFIG FACTORY
// =============================================================================

/**
 * Creates default NAPI mock configuration
 */
function createDefaultNapiConfig(): NapiMockConfig {
  return {
    cloudProviders: createDefaultCloudProviders(),
    localServerModels: new Map(),
    refreshSuccess: true,
  };
}

// =============================================================================
// CALLBACK TRACKER FACTORY
// =============================================================================

/**
 * Creates a fresh callback tracker with mocks
 */
function createCallbackTracker(): CallbackTracker {
  const tracker: CallbackTracker = {
    onSelectModel: {
      calls: [],
      mock: vi.fn(),
    },
    onClose: {
      calls: 0,
      mock: vi.fn(),
    },
    onSwitchToSettings: {
      calls: 0,
      mock: vi.fn(),
    },
  };

  // Connect mocks to call trackers
  tracker.onSelectModel.mock.mockImplementation((model: ModelSelection) => {
    tracker.onSelectModel.calls.push(model);
  });
  tracker.onClose.mock.mockImplementation(() => {
    tracker.onClose.calls++;
  });
  tracker.onSwitchToSettings.mock.mockImplementation(() => {
    tracker.onSwitchToSettings.calls++;
  });

  return tracker;
}

// =============================================================================
// FIXTURE FACTORY
// =============================================================================

/**
 * Creates a ModelSelectorScreen fixture for integration testing.
 *
 * This fixture:
 * - Composes HomeDirectoryFixture for real file system operations
 * - Provides NAPI mock functions for network boundary ONLY
 * - Uses REAL useModelSelectorState hook
 * - Uses REAL ModelSelectorScreen component
 * - Cleans up on teardown
 *
 * @example
 * ```typescript
 * describe('ModelSelectorScreen Integration', () => {
 *   let fixture: ModelSelectorScreenFixture;
 *
 *   beforeEach(async () => {
 *     fixture = await createModelSelectorScreenFixture('my-test');
 *   });
 *
 *   afterEach(async () => {
 *     await fixture.cleanup();
 *   });
 *
 *   it('should navigate with real hook', async () => {
 *     // Uses real hook, real component, only NAPI mocked
 *     await fixture.createCredential('anthropic', 'test-key');
 *     // render and test...
 *   });
 * });
 * ```
 */
export async function createModelSelectorScreenFixture(
  testName: string
): Promise<ModelSelectorScreenFixture> {
  // ========================================
  // Compose HomeDirectoryFixture
  // ========================================

  const homeFixture = await createHomeDirectoryFixture({
    testName,
    dirPrefix: 'fspec-model-screen',
  });

  // TUI-073: Reset model store to ensure clean state for each test
  useModelStore.getState().reset();

  // ========================================
  // NAPI Mock Configuration
  // ========================================

  const napiConfig = createDefaultNapiConfig();

  // Create mock functions
  const modelsListAllMock = vi.fn();
  const modelsListLocalOpenaiMock = vi.fn();
  const modelsRefreshCacheMock = vi.fn();

  // Configure mock implementations
  const updateMockImplementations = () => {
    modelsListAllMock.mockImplementation(async () => {
      return napiConfig.cloudProviders;
    });

    modelsListLocalOpenaiMock.mockImplementation(async (baseUrl: string) => {
      const result = napiConfig.localServerModels.get(baseUrl);
      if (result instanceof Error) {
        throw result;
      }
      return result || [];
    });

    modelsRefreshCacheMock.mockImplementation(async () => {
      if (!napiConfig.refreshSuccess) {
        throw new Error('Refresh failed');
      }
      return undefined;
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

  const configureNapi = (config: Partial<NapiMockConfig>): void => {
    if (config.cloudProviders !== undefined) {
      napiConfig.cloudProviders = config.cloudProviders;
    }
    if (config.localServerModels !== undefined) {
      napiConfig.localServerModels = config.localServerModels;
    }
    if (config.refreshSuccess !== undefined) {
      napiConfig.refreshSuccess = config.refreshSuccess;
    }
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

    // TUI-073: Reset model store to clear any state from previous tests
    useModelStore.getState().reset();

    // Reset NAPI config
    napiConfig.cloudProviders = createDefaultCloudProviders();
    napiConfig.localServerModels.clear();
    napiConfig.refreshSuccess = true;

    // Clear mocks
    modelsListAllMock.mockClear();
    modelsListLocalOpenaiMock.mockClear();
    modelsRefreshCacheMock.mockClear();

    updateMockImplementations();

    // Reset callbacks
    resetCallbacks();
  };

  const cleanup = async (): Promise<void> => {
    // TUI-073: Reset model store before cleanup to prevent state leakage
    useModelStore.getState().reset();
    await homeFixture.cleanup();
  };

  const waitForModelsLoaded = async (timeout = 2000): Promise<void> => {
    const startTime = Date.now();
    while (Date.now() - startTime < timeout) {
      if (modelsListAllMock.mock.calls.length > 0) {
        // Give a small delay for state to update
        await new Promise(resolve => setTimeout(resolve, 50));
        return;
      }
      await new Promise(resolve => setTimeout(resolve, 10));
    }
    throw new Error(`Timeout waiting for models to load after ${timeout}ms`);
  };

  return {
    // Delegate from HomeDirectoryFixture
    env: homeFixture.env,
    createProfile: homeFixture.createProfile,
    createCredential: homeFixture.createCredential,

    // ModelSelectorScreen-specific
    napiConfig,
    callbacks,
    configureNapi,
    setLocalServerModels,
    modelsListAllMock,
    modelsListLocalOpenaiMock,
    modelsRefreshCacheMock,
    reset,
    resetCallbacks,
    cleanup,
    waitForModelsLoaded,
  };
}
