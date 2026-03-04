/**
 * Home Directory Test Fixture - Base Fixture for Provider Config Tests
 *
 * This fixture provides the foundational layer for testing components
 * that depend on ~/.fspec configuration files and credentials.
 *
 * SOLID: Single Responsibility - Only handles HOME directory and config file management
 * DRY: Reusable base that eliminates duplication across screen fixtures
 * COMPOSABLE: Designed to be composed with component-specific fixtures
 *
 * Other fixtures should COMPOSE with this fixture rather than duplicating its logic.
 */

import { mkdir, writeFile, readFile, rm } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';

import type { ProfileConfig } from '../utils/provider-config';

// =============================================================================
// TYPES
// =============================================================================

/**
 * Home directory environment state
 */
export interface HomeDirectoryEnv {
  /** Temporary HOME directory */
  homeDir: string;
  /** Path to ~/.fspec */
  fspecDir: string;
  /** Path to ~/.fspec/fspec-config.json */
  configFile: string;
  /** Path to ~/.fspec/credentials directory */
  credentialsDir: string;
  /** Path to ~/.fspec/credentials/credentials.json */
  credentialsFile: string;
  /** Original HOME value (for restoration) */
  originalHome: string | undefined;
}

/**
 * Configuration for creating a home directory fixture
 */
export interface HomeDirectoryFixtureOptions {
  /** Test name for unique directory naming */
  testName: string;
  /** Prefix for temp directory (default: 'fspec-home') */
  dirPrefix?: string;
}

/**
 * Home directory fixture interface
 */
export interface HomeDirectoryFixture {
  /** Environment state (paths) */
  env: HomeDirectoryEnv;

  // ---- Profile Operations ----

  /**
   * Create a provider profile in the config file
   */
  createProfile: (
    providerId: string,
    profileName: string,
    config: ProfileConfig
  ) => Promise<void>;

  /**
   * Get all profiles for a provider
   */
  getProfiles: (providerId: string) => Promise<Record<string, ProfileConfig>>;

  /**
   * Get a specific profile
   */
  getProfile: (
    providerId: string,
    profileName: string
  ) => Promise<ProfileConfig | undefined>;

  /**
   * Delete a profile
   */
  deleteProfile: (providerId: string, profileName: string) => Promise<void>;

  // ---- Credential Operations ----

  /**
   * Create credentials for a provider
   */
  createCredential: (providerId: string, apiKey: string) => Promise<void>;

  /**
   * Get credentials for a provider
   */
  getCredential: (providerId: string) => Promise<string | undefined>;

  /**
   * Delete credentials for a provider
   */
  deleteCredential: (providerId: string) => Promise<void>;

  // ---- Raw Config Access ----

  /**
   * Read raw config file
   */
  readConfig: () => Promise<Record<string, unknown>>;

  /**
   * Write raw config file
   */
  writeConfig: (config: Record<string, unknown>) => Promise<void>;

  /**
   * Read raw credentials file
   */
  readCredentials: () => Promise<{
    version: number;
    providers: Record<string, unknown>;
  }>;

  /**
   * Write raw credentials file
   */
  writeCredentials: (credentials: {
    version: number;
    providers: Record<string, unknown>;
  }) => Promise<void>;

  // ---- Lifecycle ----

  /**
   * Reset config and credentials to empty state
   */
  reset: () => Promise<void>;

  /**
   * Clean up temp directories and restore HOME
   */
  cleanup: () => Promise<void>;
}

// =============================================================================
// FIXTURE FACTORY
// =============================================================================

/**
 * Creates a home directory fixture for integration testing.
 *
 * This fixture:
 * - Creates a temporary HOME directory
 * - Sets up ~/.fspec with config and credentials
 * - Overrides process.env.HOME
 * - Provides helpers for profile and credential CRUD
 * - Cleans up on teardown
 *
 * @example
 * ```typescript
 * describe('My Integration Tests', () => {
 *   let homeFixture: HomeDirectoryFixture;
 *
 *   beforeEach(async () => {
 *     homeFixture = await createHomeDirectoryFixture({ testName: 'my-test' });
 *   });
 *
 *   afterEach(async () => {
 *     await homeFixture.cleanup();
 *   });
 *
 *   it('should work with real config files', async () => {
 *     await homeFixture.createCredential('anthropic', 'test-key');
 *     // Test with real hook that reads from ~/.fspec
 *   });
 * });
 * ```
 */
export async function createHomeDirectoryFixture(
  options: HomeDirectoryFixtureOptions
): Promise<HomeDirectoryFixture> {
  const { testName, dirPrefix = 'fspec-home' } = options;

  // Generate unique directory name
  const uniqueId = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  const homeDir = join(tmpdir(), `${dirPrefix}-${testName}-${uniqueId}`);
  const fspecDir = join(homeDir, '.fspec');
  const configFile = join(fspecDir, 'fspec-config.json');
  const credentialsDir = join(fspecDir, 'credentials');
  const credentialsFile = join(credentialsDir, 'credentials.json');

  // Create directory structure
  await mkdir(fspecDir, { recursive: true });
  await mkdir(credentialsDir, { recursive: true });

  // Initialize empty config
  await writeFile(configFile, JSON.stringify({ providers: {} }, null, 2));

  // Initialize empty credentials
  await writeFile(
    credentialsFile,
    JSON.stringify({ version: 1, providers: {} }, null, 2)
  );

  // Override HOME environment variable
  const originalHome = process.env.HOME;
  process.env.HOME = homeDir;

  const env: HomeDirectoryEnv = {
    homeDir,
    fspecDir,
    configFile,
    credentialsDir,
    credentialsFile,
    originalHome,
  };

  // ========================================
  // Config File Operations
  // ========================================

  const readConfig = async (): Promise<Record<string, unknown>> => {
    if (!existsSync(configFile)) {
      return { providers: {} };
    }
    const content = await readFile(configFile, 'utf-8');
    return JSON.parse(content) as Record<string, unknown>;
  };

  const writeConfig = async (
    config: Record<string, unknown>
  ): Promise<void> => {
    await writeFile(configFile, JSON.stringify(config, null, 2));
  };

  const readCredentials = async (): Promise<{
    version: number;
    providers: Record<string, unknown>;
  }> => {
    if (!existsSync(credentialsFile)) {
      return { version: 1, providers: {} };
    }
    const content = await readFile(credentialsFile, 'utf-8');
    return JSON.parse(content) as {
      version: number;
      providers: Record<string, unknown>;
    };
  };

  const writeCredentials = async (credentials: {
    version: number;
    providers: Record<string, unknown>;
  }): Promise<void> => {
    await writeFile(credentialsFile, JSON.stringify(credentials, null, 2));
  };

  // ========================================
  // Profile Operations
  // ========================================

  const createProfile = async (
    providerId: string,
    profileName: string,
    config: ProfileConfig
  ): Promise<void> => {
    const currentConfig = await readConfig();

    if (!currentConfig.providers) {
      currentConfig.providers = {};
    }

    const providers = currentConfig.providers as Record<
      string,
      Record<string, unknown>
    >;

    if (!providers[providerId]) {
      providers[providerId] = {};
    }

    if (!providers[providerId].profiles) {
      providers[providerId].profiles = {};
    }

    const profiles = providers[providerId].profiles as Record<
      string,
      ProfileConfig
    >;
    profiles[profileName] = config;

    await writeConfig(currentConfig);
  };

  const getProfiles = async (
    providerId: string
  ): Promise<Record<string, ProfileConfig>> => {
    const config = await readConfig();
    const providers = config.providers as
      | Record<string, Record<string, unknown>>
      | undefined;

    if (!providers?.[providerId]?.profiles) {
      return {};
    }

    return providers[providerId].profiles as Record<string, ProfileConfig>;
  };

  const getProfile = async (
    providerId: string,
    profileName: string
  ): Promise<ProfileConfig | undefined> => {
    const profiles = await getProfiles(providerId);
    return profiles[profileName];
  };

  const deleteProfile = async (
    providerId: string,
    profileName: string
  ): Promise<void> => {
    const config = await readConfig();
    const providers = config.providers as
      | Record<string, Record<string, unknown>>
      | undefined;

    if (!providers?.[providerId]?.profiles) {
      return;
    }

    const profiles = providers[providerId].profiles as Record<
      string,
      ProfileConfig
    >;
    delete profiles[profileName];

    await writeConfig(config);
  };

  // ========================================
  // Credential Operations
  // ========================================

  const createCredential = async (
    providerId: string,
    apiKey: string
  ): Promise<void> => {
    const credentials = await readCredentials();

    credentials.providers[providerId] = {
      apiKey,
      lastUpdated: new Date().toISOString(),
    };

    await writeCredentials(credentials);
  };

  const getCredential = async (
    providerId: string
  ): Promise<string | undefined> => {
    const credentials = await readCredentials();
    const providerCreds = credentials.providers[providerId] as
      | { apiKey: string }
      | undefined;
    return providerCreds?.apiKey;
  };

  const deleteCredential = async (providerId: string): Promise<void> => {
    const credentials = await readCredentials();
    delete credentials.providers[providerId];
    await writeCredentials(credentials);
  };

  // ========================================
  // Lifecycle
  // ========================================

  const reset = async (): Promise<void> => {
    await writeConfig({ providers: {} });
    await writeCredentials({ version: 1, providers: {} });
  };

  const cleanup = async (): Promise<void> => {
    // Restore original HOME
    if (originalHome !== undefined) {
      process.env.HOME = originalHome;
    } else {
      delete process.env.HOME;
    }

    // Remove temp directory
    if (existsSync(homeDir)) {
      await rm(homeDir, { recursive: true, force: true });
    }
  };

  return {
    env,
    createProfile,
    getProfiles,
    getProfile,
    deleteProfile,
    createCredential,
    getCredential,
    deleteCredential,
    readConfig,
    writeConfig,
    readCredentials,
    writeCredentials,
    reset,
    cleanup,
  };
}

// =============================================================================
// COMPOSABLE HELPERS
// =============================================================================

/**
 * Sets up standard cloud provider credentials for testing.
 * Uses 'codex' instead of 'openai' because OpenAI cloud models
 * require Codex credentials (OAuth or CODEX_API_KEY).
 */
export async function setupStandardCredentials(
  fixture: HomeDirectoryFixture
): Promise<void> {
  await fixture.createCredential('anthropic', 'sk-ant-test-key-12345');
  await fixture.createCredential('codex', 'sk-codex-test-key-67890');
}

/**
 * Sets up standard local profiles for testing
 */
export async function setupStandardLocalProfiles(
  fixture: HomeDirectoryFixture
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
 * Sets up a complete test scenario with credentials and profiles
 */
export async function setupFullProviderEnvironment(
  fixture: HomeDirectoryFixture
): Promise<void> {
  await setupStandardCredentials(fixture);
  await setupStandardLocalProfiles(fixture);
}
