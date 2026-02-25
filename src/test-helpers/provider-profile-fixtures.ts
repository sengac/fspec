/**
 * Provider Profile Test Fixtures
 *
 * PROV-007: Reusable fixtures for testing provider profile configuration.
 *
 * This fixture COMPOSES the HomeDirectoryFixture and adds:
 * - Mock local server configuration
 * - Standard profile/server setups for common test scenarios
 *
 * SOLID: Single Responsibility - Only handles profile-related test setup
 * DRY: Composes HomeDirectoryFixture instead of duplicating HOME directory logic
 * COMPOSABLE: Works with other fixtures that need provider profiles
 */

import {
  createHomeDirectoryFixture,
  type HomeDirectoryFixture,
  type HomeDirectoryEnv,
} from './home-directory-fixture';
import type { ProfileConfig } from '../utils/provider-config';

// =============================================================================
// TYPES
// =============================================================================

/**
 * Mock local server configuration
 */
export interface MockLocalServer {
  /** Server base URL */
  baseUrl: string;
  /** Models available on this server */
  models: Array<{
    id: string;
    name: string;
    contextWindow?: number;
    maxOutput?: number;
  }>;
  /** If true, server is unreachable */
  unreachable?: boolean;
}

/**
 * Complete provider profile fixture (composes HomeDirectoryFixture)
 */
export interface ProviderProfileFixture {
  /** Environment state (HOME, config paths) - from HomeDirectoryFixture */
  homeDir: string;
  fspecDir: string;
  configFile: string;
  originalHome: string | undefined;

  // ---- Delegated from HomeDirectoryFixture ----

  /** Create a profile for a provider */
  createProfile: (
    providerId: string,
    profileName: string,
    config: ProfileConfig
  ) => Promise<void>;

  /** Get all profiles for a provider */
  getProfiles: (providerId: string) => Promise<Record<string, ProfileConfig>>;

  /** Get a specific profile */
  getProfile: (
    providerId: string,
    profileName: string
  ) => Promise<ProfileConfig | undefined>;

  /** Delete a profile */
  deleteProfile: (providerId: string, profileName: string) => Promise<void>;

  /** Read raw config file */
  readConfig: () => Promise<Record<string, unknown>>;

  // ---- Mock Server Management ----

  /** Register a mock local server (for model fetching tests) */
  registerMockServer: (server: MockLocalServer) => void;

  /** Get registered mock servers */
  getMockServers: () => MockLocalServer[];

  // ---- Lifecycle ----

  /** Cleanup and restore HOME */
  cleanup: () => Promise<void>;
}

// =============================================================================
// FIXTURE FACTORY
// =============================================================================

/**
 * Creates a provider profile fixture for testing.
 *
 * This fixture composes HomeDirectoryFixture and adds mock server management.
 *
 * @example
 * ```typescript
 * describe('Profile Tests', () => {
 *   let fixture: ProviderProfileFixture;
 *
 *   beforeEach(async () => {
 *     fixture = await createProviderProfileFixture('my-test');
 *   });
 *
 *   afterEach(async () => {
 *     await fixture.cleanup();
 *   });
 *
 *   it('should create profile', async () => {
 *     await fixture.createProfile('openai', 'work-vllm', {
 *       baseUrl: 'http://work:8888',
 *       apiKey: 'local-key',
 *     });
 *
 *     const profile = await fixture.getProfile('openai', 'work-vllm');
 *     expect(profile?.baseUrl).toBe('http://work:8888');
 *   });
 * });
 * ```
 */
export async function createProviderProfileFixture(
  testName: string
): Promise<ProviderProfileFixture> {
  // ========================================
  // Compose HomeDirectoryFixture
  // ========================================

  const homeFixture = await createHomeDirectoryFixture({
    testName,
    dirPrefix: 'fspec-profile-test',
  });

  // ========================================
  // Mock Server Registry
  // ========================================

  const mockServers: MockLocalServer[] = [];

  const registerMockServer = (server: MockLocalServer): void => {
    // Remove existing server with same URL
    const idx = mockServers.findIndex(s => s.baseUrl === server.baseUrl);
    if (idx !== -1) {
      mockServers.splice(idx, 1);
    }
    mockServers.push(server);
  };

  const getMockServers = (): MockLocalServer[] => {
    return [...mockServers];
  };

  return {
    // Expose HomeDirectoryEnv fields for backwards compatibility
    homeDir: homeFixture.env.homeDir,
    fspecDir: homeFixture.env.fspecDir,
    configFile: homeFixture.env.configFile,
    originalHome: homeFixture.env.originalHome,

    // Delegate from HomeDirectoryFixture
    createProfile: homeFixture.createProfile,
    getProfiles: homeFixture.getProfiles,
    getProfile: homeFixture.getProfile,
    deleteProfile: homeFixture.deleteProfile,
    readConfig: homeFixture.readConfig,
    cleanup: homeFixture.cleanup,

    // Mock server management
    registerMockServer,
    getMockServers,
  };
}

// =============================================================================
// COMPOSABLE HELPERS
// =============================================================================

/**
 * Creates standard test profiles for common scenarios.
 *
 * @example
 * ```typescript
 * const fixture = await createProviderProfileFixture('test');
 * await createStandardProfiles(fixture);
 * // Now has 'work-vllm' and 'home-ollama' profiles for openai
 * ```
 */
export async function createStandardProfiles(
  fixture: ProviderProfileFixture
): Promise<void> {
  await fixture.createProfile('openai', 'work-vllm', {
    baseUrl: 'http://work:8888',
    apiKey: 'work-api-key',
    contextWindow: 32768,
    maxOutputTokens: 8192,
  });

  await fixture.createProfile('openai', 'home-ollama', {
    baseUrl: 'http://localhost:11434',
    apiKey: 'local-key',
  });
}

/**
 * Registers standard mock servers for testing.
 *
 * @example
 * ```typescript
 * const fixture = await createProviderProfileFixture('test');
 * registerStandardMockServers(fixture);
 * // Now has mock servers for work:8888 and localhost:11434
 * ```
 */
export function registerStandardMockServers(
  fixture: ProviderProfileFixture
): void {
  fixture.registerMockServer({
    baseUrl: 'http://work:8888',
    models: [
      { id: 'Qwen/Qwen3-80B', name: 'Qwen 3 80B', contextWindow: 32768 },
      { id: 'mistral-7b', name: 'Mistral 7B', contextWindow: 8192 },
    ],
  });

  fixture.registerMockServer({
    baseUrl: 'http://localhost:11434',
    models: [
      { id: 'llama3', name: 'Llama 3', contextWindow: 8192 },
      { id: 'codellama', name: 'Code Llama', contextWindow: 16384 },
    ],
  });

  // Unreachable server for error handling tests
  fixture.registerMockServer({
    baseUrl: 'http://unreachable:8888',
    models: [],
    unreachable: true,
  });
}

/**
 * Gets models for a mock server URL.
 * Returns undefined if server is unreachable.
 */
export function getMockServerModels(
  fixture: ProviderProfileFixture,
  baseUrl: string
): Array<{ id: string; name: string }> | undefined {
  const server = fixture.getMockServers().find(s => s.baseUrl === baseUrl);
  if (!server || server.unreachable) {
    return undefined;
  }
  return server.models;
}
