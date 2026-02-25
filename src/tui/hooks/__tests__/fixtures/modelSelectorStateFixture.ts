/**
 * Model Selector State Integration Test Fixture
 *
 * TUI-072: Full integration test fixture for useModelSelectorState hook.
 *
 * This fixture COMPOSES the HomeDirectoryFixture and adds:
 * - NAPI network boundary mocking only (models.dev, local servers)
 * - Test scenario helpers for common setups
 *
 * SOLID: Single Responsibility - Model selector hook test setup only
 * DRY: Composes HomeDirectoryFixture instead of duplicating HOME directory logic
 * COMPOSABLE: Extends base fixture, can be used by component fixtures
 */

import { vi } from 'vitest';

import {
  createHomeDirectoryFixture,
  type HomeDirectoryEnv,
} from '../../../../test-helpers/home-directory-fixture';
import type { NapiProviderModels } from '@sengac/codelet-napi';
import type { ProfileConfig } from '../../../../utils/provider-config';

// Re-export from provider-type-fixtures for UI-level fixtures
import {
  createAnthropicSection,
  createOpenAiSection,
  createLocalProfileSection,
  createTestProviderSection,
  createClaudeModel,
  createGptModel,
  createLocalModel,
} from '../../../../test-helpers/provider-type-fixtures';

export {
  createAnthropicSection,
  createOpenAiSection,
  createLocalProfileSection,
  createTestProviderSection,
  createClaudeModel,
  createGptModel,
  createLocalModel,
};

// Import from centralized NAPI model fixtures
import { createDefaultCloudProviders } from '../../../../test-helpers/napi-model-fixtures';

// Re-export for fixture consumers
export { createDefaultCloudProviders } from '../../../../test-helpers/napi-model-fixtures';

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
 * Model selector state fixture (composes HomeDirectoryFixture)
 */
export interface ModelSelectorStateFixture {
  /** Test environment (HOME, config paths) - from HomeDirectoryFixture */
  env: HomeDirectoryEnv;

  /** NAPI mock configuration */
  napiConfig: NapiMockConfig;

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

  /** Clean up temp directories and restore HOME */
  cleanup: () => Promise<void>;
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
// FIXTURE FACTORY
// =============================================================================

/**
 * Creates a model selector state fixture for integration testing.
 *
 * This fixture:
 * - Composes HomeDirectoryFixture for real file system operations
 * - Provides NAPI mock functions for network boundary
 * - Uses real file system operations
 * - Cleans up on teardown
 *
 * @example
 * ```typescript
 * describe('useModelSelectorState Integration', () => {
 *   let fixture: ModelSelectorStateFixture;
 *
 *   beforeEach(async () => {
 *     fixture = await createModelSelectorStateFixture('my-test');
 *   });
 *
 *   afterEach(async () => {
 *     await fixture.cleanup();
 *   });
 *
 *   it('should load cloud models', async () => {
 *     // Uses real file system, mocks only NAPI network calls
 *     fixture.configureNapi({
 *       cloudProviders: createDefaultCloudProviders(),
 *     });
 *
 *     // Test hook with real provider-config module
 *     render(<TestComponent />);
 *     // ...
 *   });
 * });
 * ```
 */
export async function createModelSelectorStateFixture(
  testName: string
): Promise<ModelSelectorStateFixture> {
  // ========================================
  // Compose HomeDirectoryFixture
  // ========================================

  const homeFixture = await createHomeDirectoryFixture({
    testName,
    dirPrefix: 'fspec-model-selector',
  });

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

  const reset = async (): Promise<void> => {
    // Reset HOME directory fixture
    await homeFixture.reset();

    // Reset NAPI config
    napiConfig.cloudProviders = createDefaultCloudProviders();
    napiConfig.localServerModels.clear();
    napiConfig.refreshSuccess = true;

    // Clear mocks
    modelsListAllMock.mockClear();
    modelsListLocalOpenaiMock.mockClear();
    modelsRefreshCacheMock.mockClear();

    updateMockImplementations();
  };

  const cleanup = async (): Promise<void> => {
    await homeFixture.cleanup();
  };

  return {
    // Delegate from HomeDirectoryFixture
    env: homeFixture.env,
    createProfile: homeFixture.createProfile,
    createCredential: homeFixture.createCredential,

    // Model selector specific
    napiConfig,
    configureNapi,
    setLocalServerModels,
    modelsListAllMock,
    modelsListLocalOpenaiMock,
    modelsRefreshCacheMock,
    reset,
    cleanup,
  };
}

// =============================================================================
// TEST SCENARIO HELPERS
// =============================================================================

/**
 * Sets up a fixture with standard cloud providers and credentials
 */
export async function setupWithCloudCredentials(
  fixture: ModelSelectorStateFixture
): Promise<void> {
  // Create credentials for cloud providers
  await fixture.createCredential('anthropic', 'sk-ant-test-key-12345');
  await fixture.createCredential('openai', 'sk-test-key-67890');

  // Configure NAPI to return cloud providers
  fixture.configureNapi({
    cloudProviders: createDefaultCloudProviders(),
  });
}

/**
 * Sets up a fixture with a local profile
 */
export async function setupWithLocalProfile(
  fixture: ModelSelectorStateFixture,
  profileName: string,
  baseUrl: string = 'http://localhost:8000',
  models: string[] = ['llama3', 'codellama']
): Promise<void> {
  // Create profile
  await fixture.createProfile('openai', profileName, {
    baseUrl,
    apiKey: 'local-key',
    contextWindow: 128000,
    maxOutputTokens: 16384,
  });

  // Set local server models
  fixture.setLocalServerModels(baseUrl, models);
}

/**
 * Sets up a fixture with an unreachable local server
 */
export async function setupWithUnreachableServer(
  fixture: ModelSelectorStateFixture,
  profileName: string,
  baseUrl: string = 'http://unreachable:8888'
): Promise<void> {
  // Create profile
  await fixture.createProfile('openai', profileName, {
    baseUrl,
    apiKey: 'key',
  });

  // Set server as unreachable
  fixture.setLocalServerModels(baseUrl, new Error('Connection refused'));
}

/**
 * Creates a complete multi-provider scenario
 */
export async function setupFullScenario(
  fixture: ModelSelectorStateFixture
): Promise<void> {
  // Cloud credentials
  await fixture.createCredential('anthropic', 'sk-ant-test-12345');
  await fixture.createCredential('openai', 'sk-test-67890');

  // Local profiles
  await setupWithLocalProfile(fixture, 'work-vllm', 'http://work:8888', [
    'Qwen/Qwen3-80B',
    'mistral-7b',
  ]);
  await setupWithLocalProfile(
    fixture,
    'home-ollama',
    'http://localhost:11434',
    ['llama3', 'codellama']
  );

  // Unreachable server
  await setupWithUnreachableServer(fixture, 'dead-server', 'http://dead:9999');

  // Cloud providers
  fixture.configureNapi({
    cloudProviders: createDefaultCloudProviders(),
  });
}
