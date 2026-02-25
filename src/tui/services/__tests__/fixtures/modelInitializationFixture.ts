/**
 * Model Initialization Service Test Fixture
 *
 * PROV-008: Provides composable test setup for model initialization service.
 *
 * This fixture composes with HomeDirectoryFixture and provides:
 * - NAPI mock configuration for network boundary
 * - Helper functions for common test scenarios
 *
 * SOLID: Single Responsibility - Service initialization test setup only
 * DRY: Reusable across service tests and hook tests that use the service
 * COMPOSABLE: Extends HomeDirectoryFixture, can be used by component fixtures
 */

import { vi } from 'vitest';
import type { NapiProviderModels, NapiModelInfo } from '@sengac/codelet-napi';
import {
  createHomeDirectoryFixture,
  type HomeDirectoryFixture,
} from '../../../../test-helpers/home-directory-fixture';
import { createDefaultCloudProviders } from '../../../../test-helpers/napi-model-fixtures';
import type { ProfileConfig } from '../../../../utils/provider-config';

// =============================================================================
// TYPES
// =============================================================================

/**
 * NAPI mock configuration for model initialization
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
 * Model initialization fixture interface
 */
export interface ModelInitializationFixture {
  /** Home directory fixture (config files, credentials) */
  homeFixture: HomeDirectoryFixture;

  /** NAPI mock configuration */
  napiConfig: NapiMockConfig;

  /** NAPI mock functions */
  mocks: {
    modelsListAll: ReturnType<typeof vi.fn>;
    modelsListLocalOpenai: ReturnType<typeof vi.fn>;
    modelsRefreshCache: ReturnType<typeof vi.fn>;
  };

  /** Configure NAPI mock responses */
  configureNapi: (config: Partial<NapiMockConfig>) => void;

  /** Set local server models (or error for unreachable) */
  setLocalServerModels: (baseUrl: string, models: string[] | Error) => void;

  /** Create a provider profile */
  createProfile: (
    providerId: string,
    profileName: string,
    config: ProfileConfig
  ) => Promise<void>;

  /** Create credentials for a provider */
  createCredential: (providerId: string, apiKey: string) => Promise<void>;

  /** Reset fixture state between tests */
  reset: () => Promise<void>;

  /** Clean up temp directories and restore HOME */
  cleanup: () => Promise<void>;
}

// =============================================================================
// NAPI CONFIG FACTORY
// =============================================================================

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
 * Creates a model initialization fixture for integration testing.
 *
 * This fixture:
 * - Composes HomeDirectoryFixture for real config file operations
 * - Provides NAPI mock functions that can be wired to vitest mocks
 * - Uses real file system for credentials/profiles
 * - Cleans up on teardown
 *
 * @example
 * ```typescript
 * // In test file:
 * const napiMock = vi.hoisted(() => ({
 *   modelsListAll: vi.fn(),
 *   modelsListLocalOpenai: vi.fn(),
 *   modelsRefreshCache: vi.fn(),
 * }));
 *
 * vi.mock('@sengac/codelet-napi', () => ({
 *   modelsListAll: () => napiMock.modelsListAll(),
 *   modelsListLocalOpenai: (url: string) => napiMock.modelsListLocalOpenai(url),
 *   modelsRefreshCache: () => napiMock.modelsRefreshCache(),
 * }));
 *
 * let fixture: ModelInitializationFixture;
 *
 * beforeEach(async () => {
 *   fixture = await createModelInitializationFixture('my-test');
 *   // Wire fixture mocks to vitest mocks
 *   napiMock.modelsListAll.mockImplementation(() => fixture.mocks.modelsListAll());
 *   napiMock.modelsListLocalOpenai.mockImplementation((url) => fixture.mocks.modelsListLocalOpenai(url));
 *   napiMock.modelsRefreshCache.mockImplementation(() => fixture.mocks.modelsRefreshCache());
 * });
 *
 * afterEach(async () => {
 *   await fixture.cleanup();
 * });
 * ```
 */
export async function createModelInitializationFixture(
  testName: string
): Promise<ModelInitializationFixture> {
  // Compose HomeDirectoryFixture
  const homeFixture = await createHomeDirectoryFixture({
    testName,
    dirPrefix: 'fspec-model-init',
  });

  // NAPI mock configuration
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

  // NAPI configuration helpers
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

  // Lifecycle
  const reset = async (): Promise<void> => {
    await homeFixture.reset();

    napiConfig.cloudProviders = createDefaultCloudProviders();
    napiConfig.localServerModels.clear();
    napiConfig.refreshSuccess = true;

    modelsListAllMock.mockClear();
    modelsListLocalOpenaiMock.mockClear();
    modelsRefreshCacheMock.mockClear();

    updateMockImplementations();
  };

  const cleanup = async (): Promise<void> => {
    await homeFixture.cleanup();
  };

  return {
    homeFixture,
    napiConfig,
    mocks: {
      modelsListAll: modelsListAllMock,
      modelsListLocalOpenai: modelsListLocalOpenaiMock,
      modelsRefreshCache: modelsRefreshCacheMock,
    },
    configureNapi,
    setLocalServerModels,
    createProfile: homeFixture.createProfile,
    createCredential: homeFixture.createCredential,
    reset,
    cleanup,
  };
}

// =============================================================================
// TEST SCENARIO HELPERS
// =============================================================================

/**
 * Sets up fixture with standard cloud provider credentials
 */
export async function setupCloudCredentials(
  fixture: ModelInitializationFixture
): Promise<void> {
  await fixture.createCredential('anthropic', 'sk-ant-test-key-12345');
  await fixture.createCredential('openai', 'sk-test-key-67890');

  fixture.configureNapi({
    cloudProviders: createDefaultCloudProviders(),
  });
}

/**
 * Sets up fixture with a local profile
 */
export async function setupLocalProfile(
  fixture: ModelInitializationFixture,
  profileName: string,
  baseUrl: string = 'http://localhost:8000',
  models: string[] = ['llama3', 'codellama']
): Promise<void> {
  await fixture.createProfile('openai', profileName, {
    baseUrl,
    apiKey: 'local-key',
    contextWindow: 128000,
    maxOutputTokens: 16384,
  });

  fixture.setLocalServerModels(baseUrl, models);
}

/**
 * Sets up fixture with an unreachable local server
 */
export async function setupUnreachableServer(
  fixture: ModelInitializationFixture,
  profileName: string,
  baseUrl: string = 'http://unreachable:8888'
): Promise<void> {
  await fixture.createProfile('openai', profileName, {
    baseUrl,
    apiKey: 'key',
  });

  fixture.setLocalServerModels(baseUrl, new Error('Connection refused'));
}
