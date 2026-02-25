/**
 * Feature: Provider Profile Loading Integration Tests
 *
 * PROV-007: Integration tests for profile loading that mirror real configuration.
 * These tests verify the complete flow from config file to model selector sections.
 *
 * Uses REAL implementations - no mocks - to debug actual loading issues.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdir, writeFile, readFile, rm } from 'fs/promises';
import { existsSync } from 'fs';

// Real implementations under test
import {
  loadProviderProfiles,
  loadProviderConfig,
  type ProfileConfig,
  type ProviderConfig,
} from '../provider-config';
import { loadConfig } from '../config';

// ============================================
// TEST FIXTURES
// ============================================

/**
 * Creates a realistic fspec config fixture matching user's actual configuration.
 */
interface RealConfigFixture {
  homeDir: string;
  fspecDir: string;
  configFile: string;
  originalHome: string | undefined;
  writeConfig: (config: Record<string, unknown>) => Promise<void>;
  readConfig: () => Promise<Record<string, unknown>>;
  cleanup: () => Promise<void>;
}

async function createRealConfigFixture(
  testName: string
): Promise<RealConfigFixture> {
  const homeDir = join('/tmp', `fspec-real-config-${testName}-${Date.now()}`);
  const fspecDir = join(homeDir, '.fspec');
  const configFile = join(fspecDir, 'fspec-config.json');

  await mkdir(fspecDir, { recursive: true });

  const originalHome = process.env.HOME;
  process.env.HOME = homeDir;

  const writeConfig = async (
    config: Record<string, unknown>
  ): Promise<void> => {
    await writeFile(configFile, JSON.stringify(config, null, 2));
  };

  const readConfig = async (): Promise<Record<string, unknown>> => {
    if (!existsSync(configFile)) {
      return {};
    }
    const content = await readFile(configFile, 'utf-8');
    return JSON.parse(content) as Record<string, unknown>;
  };

  const cleanup = async (): Promise<void> => {
    if (originalHome !== undefined) {
      process.env.HOME = originalHome;
    } else {
      delete process.env.HOME;
    }
    if (existsSync(homeDir)) {
      await rm(homeDir, { recursive: true, force: true });
    }
  };

  return {
    homeDir,
    fspecDir,
    configFile,
    originalHome,
    writeConfig,
    readConfig,
    cleanup,
  };
}

// ============================================
// INTEGRATION TESTS
// ============================================

describe('Feature: Provider Profile Loading Integration', () => {
  let fixture: RealConfigFixture;

  beforeEach(async () => {
    fixture = await createRealConfigFixture('profile-loading');
  });

  afterEach(async () => {
    await fixture.cleanup();
  });

  describe('Scenario: Load profiles from real config structure', () => {
    it('should load profiles matching user actual config structure', async () => {
      // @step Given a config file matching user's actual structure
      const realConfig = {
        research: {
          perplexity: { apiKey: 'test-key' },
        },
        agent: 'claude',
        tools: {
          test: { command: 'npm run test' },
        },
        tui: {
          lastUsedModel: 'anthropic/claude-opus-4-5',
          defaultThinkingLevel: 3,
        },
        providers: {
          openai: {
            profiles: {
              'qwen3-coder-next': {
                baseUrl: 'http://192.168.0.50:8888',
                apiKey: 'nothing',
              },
            },
          },
        },
      };
      await fixture.writeConfig(realConfig);

      // @step When I call loadConfig()
      const config = await loadConfig(process.cwd());

      // @step Then providers should be loaded
      expect(config).toBeDefined();
      expect(config.providers).toBeDefined();
      console.log('Loaded config:', JSON.stringify(config, null, 2));

      // @step And openai provider should have profiles
      expect(config.providers.openai).toBeDefined();
      expect(config.providers.openai.profiles).toBeDefined();
      console.log(
        'OpenAI provider:',
        JSON.stringify(config.providers.openai, null, 2)
      );
    });

    it('should load provider config for openai', async () => {
      // @step Given a config file with openai profiles
      const realConfig = {
        providers: {
          openai: {
            profiles: {
              'qwen3-coder-next': {
                baseUrl: 'http://192.168.0.50:8888',
                apiKey: 'nothing',
              },
            },
          },
        },
      };
      await fixture.writeConfig(realConfig);

      // @step When I call loadProviderConfig('openai')
      const providerConfig = await loadProviderConfig('openai');

      // @step Then it should return the openai config with profiles
      expect(providerConfig).toBeDefined();
      console.log('Provider config:', JSON.stringify(providerConfig, null, 2));
      expect(providerConfig.profiles).toBeDefined();
      expect(providerConfig.profiles?.['qwen3-coder-next']).toBeDefined();
    });

    it('should load profiles via loadProviderProfiles', async () => {
      // @step Given a config file with openai profiles
      const realConfig = {
        providers: {
          openai: {
            profiles: {
              'qwen3-coder-next': {
                baseUrl: 'http://192.168.0.50:8888',
                apiKey: 'nothing',
              },
            },
          },
        },
      };
      await fixture.writeConfig(realConfig);

      // @step When I call loadProviderProfiles('openai')
      const profiles = await loadProviderProfiles('openai');

      // @step Then it should return the profiles
      console.log('Loaded profiles:', JSON.stringify(profiles, null, 2));
      expect(profiles).toBeDefined();
      expect(Object.keys(profiles).length).toBeGreaterThan(0);
      expect(profiles['qwen3-coder-next']).toBeDefined();
      expect(profiles['qwen3-coder-next'].baseUrl).toBe(
        'http://192.168.0.50:8888'
      );
      expect(profiles['qwen3-coder-next'].apiKey).toBe('nothing');
    });

    it('should return empty profiles for provider without profiles', async () => {
      // @step Given a config file without anthropic profiles
      const realConfig = {
        providers: {
          openai: {
            profiles: {
              'test-profile': {
                baseUrl: 'http://localhost:8888',
                apiKey: 'test',
              },
            },
          },
        },
      };
      await fixture.writeConfig(realConfig);

      // @step When I call loadProviderProfiles('anthropic')
      const profiles = await loadProviderProfiles('anthropic');

      // @step Then it should return empty object
      console.log('Anthropic profiles:', JSON.stringify(profiles, null, 2));
      expect(profiles).toEqual({});
    });

    it('should iterate through all supported providers', async () => {
      // @step Given a config file with openai profiles
      const realConfig = {
        providers: {
          openai: {
            profiles: {
              'local-vllm': {
                baseUrl: 'http://localhost:8888',
                apiKey: 'local',
              },
            },
          },
        },
      };
      await fixture.writeConfig(realConfig);

      // @step When I iterate through SUPPORTED_PROVIDERS like AgentView does
      const { SUPPORTED_PROVIDERS } = await import('../provider-config');

      const profilesByProvider: Record<
        string,
        Record<string, ProfileConfig>
      > = {};
      for (const providerId of SUPPORTED_PROVIDERS) {
        const profiles = await loadProviderProfiles(providerId);
        const profileNames = Object.keys(profiles);
        console.log(`Provider ${providerId}: ${profileNames.length} profiles`);
        if (profileNames.length > 0) {
          profilesByProvider[providerId] = profiles;
        }
      }

      // @step Then openai should have profiles
      expect(profilesByProvider.openai).toBeDefined();
      expect(Object.keys(profilesByProvider.openai).length).toBe(1);
      expect(profilesByProvider.openai['local-vllm']).toBeDefined();
    });
  });

  describe('Scenario: Config file not found', () => {
    it('should return empty profiles when config file does not exist', async () => {
      // @step Given no config file exists (cleanup fixture config)
      const { rm: removeFile } = await import('fs/promises');
      if (existsSync(fixture.configFile)) {
        await removeFile(fixture.configFile);
      }

      // @step When I call loadProviderProfiles('openai')
      const profiles = await loadProviderProfiles('openai');

      // @step Then it should return empty object
      expect(profiles).toEqual({});
    });
  });

  describe('Scenario: Verify config file path resolution', () => {
    it('should use HOME environment variable for config path', async () => {
      // @step Given HOME is set to our test directory
      expect(process.env.HOME).toBe(fixture.homeDir);

      // @step And a config file exists at $HOME/.fspec/fspec-config.json
      const testConfig = {
        providers: {
          openai: {
            profiles: {
              'path-test': {
                baseUrl: 'http://path-test:8888',
                apiKey: 'path-key',
              },
            },
          },
        },
      };
      await fixture.writeConfig(testConfig);

      // @step When I verify the config file exists
      const configPath = join(fixture.homeDir, '.fspec', 'fspec-config.json');
      expect(existsSync(configPath)).toBe(true);

      // @step And I load profiles
      const profiles = await loadProviderProfiles('openai');

      // @step Then profiles should be loaded from that path
      expect(profiles['path-test']).toBeDefined();
      expect(profiles['path-test'].baseUrl).toBe('http://path-test:8888');
    });
  });

  describe('Scenario: Debug actual getFspecUserDir resolution', () => {
    it('should verify getFspecUserDir uses correct HOME', async () => {
      // @step Given HOME is set to our test directory
      const { getFspecUserDir } = await import('../config');

      // @step When I call getFspecUserDir
      const fspecDir = getFspecUserDir();

      // @step Then it should point to $HOME/.fspec
      console.log('getFspecUserDir() returned:', fspecDir);
      console.log('Expected:', join(fixture.homeDir, '.fspec'));
      expect(fspecDir).toBe(join(fixture.homeDir, '.fspec'));
    });
  });
});
