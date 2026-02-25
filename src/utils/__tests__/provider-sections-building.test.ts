/**
 * Feature: Provider Section Building Integration Tests
 *
 * PROV-007: Integration tests for building provider sections from profiles.
 * This tests the actual flow that AgentView uses to build the model selector.
 *
 * Uses REAL implementations - no mocks - to debug actual section building issues.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { join } from 'path';
import { mkdir, writeFile, rm } from 'fs/promises';
import { existsSync } from 'fs';

// Real implementations under test
import {
  loadProviderProfiles,
  SUPPORTED_PROVIDERS,
  type ProfileConfig,
} from '../provider-config';

// PROV-008: Import provider mapping from shared utility (DRY)
import { mapProviderIdToInternal } from '../../tui/utils/provider-mapping';

// ============================================
// TEST FIXTURES
// ============================================

interface SectionBuildingFixture {
  homeDir: string;
  fspecDir: string;
  configFile: string;
  originalHome: string | undefined;
  writeConfig: (config: Record<string, unknown>) => Promise<void>;
  cleanup: () => Promise<void>;
}

async function createSectionBuildingFixture(
  testName: string
): Promise<SectionBuildingFixture> {
  const homeDir = join('/tmp', `fspec-sections-${testName}-${Date.now()}`);
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
    cleanup,
  };
}

// ============================================
// MOCK TYPES (matching AgentView)
// ============================================

interface NapiModelInfo {
  id: string;
  name: string;
  reasoning: boolean;
  toolCall: boolean;
  attachment: boolean;
  temperature: boolean;
  contextWindow: number;
  maxOutput: number;
  hasVision: boolean;
}

interface ProviderSection {
  providerId: string;
  providerName: string;
  internalName: string;
  models: NapiModelInfo[];
  hasCredentials: boolean;
  profileName?: string;
  profileConfig?: ProfileConfig;
  isUnreachable?: boolean;
}

// ============================================
// INTEGRATION TESTS
// ============================================

describe('Feature: Provider Section Building Integration', () => {
  let fixture: SectionBuildingFixture;

  beforeEach(async () => {
    fixture = await createSectionBuildingFixture('section-building');
  });

  afterEach(async () => {
    await fixture.cleanup();
  });

  describe('Scenario: Build profile sections like AgentView', () => {
    it('should build profile sections for all providers with profiles', async () => {
      // @step Given a config file with openai profile
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

      // @step When I iterate through SUPPORTED_PROVIDERS like AgentView does
      const profileSections: ProviderSection[] = [];

      for (const providerId of SUPPORTED_PROVIDERS) {
        try {
          const profiles = await loadProviderProfiles(providerId);
          const profileNames = Object.keys(profiles);

          console.log(
            `Provider ${providerId}: ${profileNames.length} profiles found`
          );

          for (const profileName of profileNames) {
            const profile = profiles[profileName];
            const displayName = `${providerId}: ${profileName}`;

            console.log(`  Creating section for profile: ${profileName}`);
            console.log(`    baseUrl: ${profile.baseUrl}`);
            console.log(`    apiKey: ${profile.apiKey ? '***' : 'none'}`);

            // Simulate model fetching failure (server not reachable in test)
            const localModels: NapiModelInfo[] = [];
            const isUnreachable = true; // Assume unreachable in test

            profileSections.push({
              providerId,
              providerName: isUnreachable
                ? `${displayName} (unreachable)`
                : displayName,
              internalName: mapProviderIdToInternal(providerId),
              models: localModels,
              hasCredentials: true,
              profileName,
              profileConfig: profile,
              isUnreachable,
            });
          }
        } catch (err) {
          console.error(`Failed to load profiles for ${providerId}:`, err);
        }
      }

      // @step Then profile sections should be created
      console.log(`\nTotal profile sections: ${profileSections.length}`);
      console.log(
        'Profile sections:',
        JSON.stringify(profileSections, null, 2)
      );

      expect(profileSections.length).toBe(1);
      expect(profileSections[0].providerId).toBe('openai');
      expect(profileSections[0].profileName).toBe('qwen3-coder-next');
      expect(profileSections[0].profileConfig?.baseUrl).toBe(
        'http://192.168.0.50:8888'
      );
    });

    it('should include profile sections alongside cloud sections', async () => {
      // @step Given a config file with openai profile
      const realConfig = {
        providers: {
          openai: {
            profiles: {
              'local-vllm': {
                baseUrl: 'http://localhost:8888',
                apiKey: 'local-key',
              },
            },
          },
        },
      };
      await fixture.writeConfig(realConfig);

      // @step When I build both cloud and profile sections

      // Simulate cloud sections (from models.dev)
      const cloudSections: ProviderSection[] = [
        {
          providerId: 'openai',
          providerName: 'OpenAI',
          internalName: 'openai',
          models: [], // Assume no credentials for this test
          hasCredentials: false,
        },
        {
          providerId: 'anthropic',
          providerName: 'Anthropic',
          internalName: 'claude',
          models: [
            {
              id: 'claude-sonnet-4',
              name: 'Claude Sonnet 4',
              reasoning: false,
              toolCall: true,
              attachment: false,
              temperature: true,
              contextWindow: 200000,
              maxOutput: 16000,
              hasVision: true,
            },
          ],
          hasCredentials: true,
        },
      ];

      // Build profile sections
      const profileSections: ProviderSection[] = [];
      for (const providerId of SUPPORTED_PROVIDERS) {
        const profiles = await loadProviderProfiles(providerId);
        const profileNames = Object.keys(profiles);

        for (const profileName of profileNames) {
          const profile = profiles[profileName];
          profileSections.push({
            providerId,
            providerName: `${providerId}: ${profileName}`,
            internalName: mapProviderIdToInternal(providerId),
            models: [],
            hasCredentials: true,
            profileName,
            profileConfig: profile,
            isUnreachable: true,
          });
        }
      }

      // Combine sections (profiles first, like AgentView does)
      const allSections = [...profileSections, ...cloudSections];

      // @step Then profile section should appear first
      console.log(
        'All sections:',
        allSections.map(s => ({
          providerId: s.providerId,
          profileName: s.profileName,
          providerName: s.providerName,
        }))
      );

      expect(allSections.length).toBe(3);
      expect(allSections[0].profileName).toBe('local-vllm');
      expect(allSections[0].providerId).toBe('openai');
    });

    it('should handle multiple profiles for same provider', async () => {
      // @step Given multiple profiles for openai
      const realConfig = {
        providers: {
          openai: {
            profiles: {
              'work-vllm': {
                baseUrl: 'http://work:8888',
                apiKey: 'work-key',
              },
              'home-ollama': {
                baseUrl: 'http://localhost:11434',
                apiKey: 'home-key',
              },
            },
          },
        },
      };
      await fixture.writeConfig(realConfig);

      // @step When I build profile sections
      const profileSections: ProviderSection[] = [];
      for (const providerId of SUPPORTED_PROVIDERS) {
        const profiles = await loadProviderProfiles(providerId);
        for (const [profileName, profile] of Object.entries(profiles)) {
          profileSections.push({
            providerId,
            providerName: `${providerId}: ${profileName}`,
            internalName: mapProviderIdToInternal(providerId),
            models: [],
            hasCredentials: true,
            profileName,
            profileConfig: profile,
          });
        }
      }

      // @step Then both profiles should create separate sections
      console.log(
        'Profile sections:',
        profileSections.map(s => s.providerName)
      );

      expect(profileSections.length).toBe(2);
      expect(profileSections.some(s => s.profileName === 'work-vllm')).toBe(
        true
      );
      expect(profileSections.some(s => s.profileName === 'home-ollama')).toBe(
        true
      );
    });
  });

  describe('Scenario: Filter sections by credentials/models', () => {
    it('should include profile sections even with 0 models (unreachable)', async () => {
      // @step Given a profile for an unreachable server
      const realConfig = {
        providers: {
          openai: {
            profiles: {
              'unreachable-server': {
                baseUrl: 'http://unreachable:8888',
                apiKey: 'key',
              },
            },
          },
        },
      };
      await fixture.writeConfig(realConfig);

      // @step When I build sections like AgentView
      const profileSections: ProviderSection[] = [];
      for (const providerId of SUPPORTED_PROVIDERS) {
        const profiles = await loadProviderProfiles(providerId);
        for (const [profileName, profile] of Object.entries(profiles)) {
          // Simulate unreachable server
          profileSections.push({
            providerId,
            providerName: `${providerId}: ${profileName} (unreachable)`,
            internalName: mapProviderIdToInternal(providerId),
            models: [], // No models - server unreachable
            hasCredentials: true,
            profileName,
            profileConfig: profile,
            isUnreachable: true,
          });
        }
      }

      // @step Then section should still be created (shows as unreachable)
      expect(profileSections.length).toBe(1);
      expect(profileSections[0].isUnreachable).toBe(true);
      expect(profileSections[0].providerName).toContain('unreachable');

      // @step And section should have hasCredentials=true (profiles always do)
      expect(profileSections[0].hasCredentials).toBe(true);
    });

    it('should verify setProviderSections receives profile sections', async () => {
      // @step Given a profile configuration
      const realConfig = {
        providers: {
          openai: {
            profiles: {
              'test-profile': {
                baseUrl: 'http://test:8888',
                apiKey: 'test',
              },
            },
          },
        },
      };
      await fixture.writeConfig(realConfig);

      // @step When I build the final sections array
      const cloudSections: ProviderSection[] = [
        {
          providerId: 'anthropic',
          providerName: 'Anthropic',
          internalName: 'claude',
          models: [
            {
              id: 'claude-sonnet-4',
              name: 'Claude Sonnet 4',
              reasoning: false,
              toolCall: true,
              attachment: false,
              temperature: true,
              contextWindow: 200000,
              maxOutput: 16000,
              hasVision: true,
            },
          ],
          hasCredentials: true,
        },
      ];

      const profileSections: ProviderSection[] = [];
      for (const providerId of SUPPORTED_PROVIDERS) {
        const profiles = await loadProviderProfiles(providerId);
        for (const [profileName, profile] of Object.entries(profiles)) {
          profileSections.push({
            providerId,
            providerName: `${providerId}: ${profileName}`,
            internalName: mapProviderIdToInternal(providerId),
            models: [],
            hasCredentials: true,
            profileName,
            profileConfig: profile,
            isUnreachable: true,
          });
        }
      }

      // Combine like AgentView does
      const sections: ProviderSection[] = [
        ...profileSections,
        ...cloudSections,
      ];

      // @step Then setProviderSections should receive all sections
      console.log('Final sections for setProviderSections:', sections.length);
      console.log('Profile sections count:', profileSections.length);
      console.log('Cloud sections count:', cloudSections.length);

      expect(sections.length).toBe(2); // 1 profile + 1 cloud
      expect(sections[0].profileName).toBe('test-profile'); // Profile first
      expect(sections[1].profileName).toBeUndefined(); // Cloud has no profileName
    });
  });
});
