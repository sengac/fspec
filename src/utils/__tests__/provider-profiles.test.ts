/**
 * Feature: spec/features/provider-configuration-display-and-profiles.feature
 *
 * Tests for provider profile configuration and TUI display.
 * These tests cover profile management including:
 * - Profile config CRUD (create/read/update/delete)
 * - Profile config structure validation
 * - Model selector profile sections
 * - Session creation with profile config
 * - Profile settings flow to Rust
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { join } from 'path';
import { readFile, mkdir, writeFile } from 'fs/promises';

import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';

// These imports will fail until implemented - that's expected (red phase)
import {
  loadProviderConfig,
  saveProviderConfig,
  getProviderRegistryEntry,
  type ProfileConfig,
  loadProviderProfiles,
  saveProfile,
  deleteProfile,
  getProfile,
} from '../provider-config';

describe('Feature: Provider Configuration Persistence and TUI Display', () => {
  let setup: TestDirectorySetup;
  let originalHome: string | undefined;

  beforeEach(async () => {
    setup = await setupTestDirectory('provider-profiles');
    originalHome = process.env.HOME;
    process.env.HOME = setup.testDir;

    // Create .fspec directory
    await mkdir(join(setup.testDir, '.fspec'), { recursive: true });
  });

  afterEach(async () => {
    process.env.HOME = originalHome;
    vi.clearAllMocks();
    await setup.cleanup();
  });

  // ============================================
  // PROFILE CONFIG STRUCTURE
  // ============================================

  describe('Scenario: Profile config structure', () => {
    it('should store profile config with correct structure', async () => {
      // @step Given I create a profile for "openai" provider
      const profileConfig: ProfileConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'local-key',
        contextWindow: 32768,
        maxOutputTokens: 8192,
      };
      await saveProfile('openai', 'work-vllm', profileConfig);

      // @step Then the config file structure should be:
      const configPath = join(setup.testDir, '.fspec', 'fspec-config.json');
      const configContent = await readFile(configPath, 'utf-8');
      const config = JSON.parse(configContent);

      // | path                                           | type   | description                    |
      // | providers.openai.profiles                      | object | Map of profile name to config  |
      expect(config.providers).toBeDefined();
      expect(config.providers.openai).toBeDefined();
      expect(config.providers.openai.profiles).toBeDefined();
      expect(typeof config.providers.openai.profiles).toBe('object');

      // | providers.openai.profiles.*.baseUrl            | string | API endpoint URL               |
      expect(config.providers.openai.profiles['work-vllm'].baseUrl).toBe(
        'http://work:8888'
      );
      expect(typeof config.providers.openai.profiles['work-vllm'].baseUrl).toBe(
        'string'
      );

      // | providers.openai.profiles.*.apiKey             | string | API key for this profile       |
      expect(config.providers.openai.profiles['work-vllm'].apiKey).toBe(
        'local-key'
      );
      expect(typeof config.providers.openai.profiles['work-vllm'].apiKey).toBe(
        'string'
      );

      // | providers.openai.profiles.*.contextWindow      | number | Context window size (optional) |
      expect(config.providers.openai.profiles['work-vllm'].contextWindow).toBe(
        32768
      );
      expect(
        typeof config.providers.openai.profiles['work-vllm'].contextWindow
      ).toBe('number');

      // | providers.openai.profiles.*.maxOutputTokens    | number | Max output tokens (optional)   |
      expect(
        config.providers.openai.profiles['work-vllm'].maxOutputTokens
      ).toBe(8192);
      expect(
        typeof config.providers.openai.profiles['work-vllm'].maxOutputTokens
      ).toBe('number');
    });
  });

  // ============================================
  // PROFILE CRUD OPERATIONS
  // ============================================

  describe('Scenario: View list of profiles for a provider', () => {
    it('should return all profiles for a provider', async () => {
      // @step Given I have profiles "work-vllm" and "home-ollama" configured for "openai" provider
      const workProfile: ProfileConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'work-key',
      };
      const homeProfile: ProfileConfig = {
        baseUrl: 'http://localhost:11434',
        apiKey: 'local-key',
      };
      await saveProfile('openai', 'work-vllm', workProfile);
      await saveProfile('openai', 'home-ollama', homeProfile);

      // @step When I run the "/provider" command
      // @step And I select the "openai" provider
      const profiles = await loadProviderProfiles('openai');

      // @step Then I should see profile "work-vllm" with its settings
      expect(profiles['work-vllm']).toBeDefined();
      expect(profiles['work-vllm'].baseUrl).toBe('http://work:8888');
      expect(profiles['work-vllm'].apiKey).toBe('work-key');

      // @step And I should see profile "home-ollama" with its settings
      expect(profiles['home-ollama']).toBeDefined();
      expect(profiles['home-ollama'].baseUrl).toBe('http://localhost:11434');
      expect(profiles['home-ollama'].apiKey).toBe('local-key');
    });
  });

  describe('Scenario: Create a new profile', () => {
    it('should persist profile to config file', async () => {
      // @step Given I am viewing the "openai" provider in /provider screen

      // @step When I create a new profile named "dev-server"
      // @step And I set the profile configuration:
      // | setting         | value                 |
      // | baseUrl         | http://dev:8888       |
      // | apiKey          | dev-api-key           |
      // | contextWindow   | 16384                 |
      // | maxOutputTokens | 4096                  |
      const profileConfig: ProfileConfig = {
        baseUrl: 'http://dev:8888',
        apiKey: 'dev-api-key',
        contextWindow: 16384,
        maxOutputTokens: 4096,
      };

      // @step And I save the profile
      await saveProfile('openai', 'dev-server', profileConfig);

      // @step Then the config file should contain the profile under "providers.openai.profiles.dev-server"
      const configPath = join(setup.testDir, '.fspec', 'fspec-config.json');
      const configContent = await readFile(configPath, 'utf-8');
      const config = JSON.parse(configContent);

      expect(config.providers.openai.profiles['dev-server']).toEqual({
        baseUrl: 'http://dev:8888',
        apiKey: 'dev-api-key',
        contextWindow: 16384,
        maxOutputTokens: 4096,
      });

      // @step And the profile should appear in /model selector as "openai: dev-server"
      // Note: This step is verified through TUI integration - the presence of the profile
      // in the config file is what makes it appear in the model selector.
      // Full TUI verification is in AgentView-profile-sections.test.tsx
      expect(config.providers.openai.profiles['dev-server']).toBeDefined();
    });
  });

  describe('Scenario: Edit an existing profile', () => {
    it('should update profile settings in config file', async () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider with baseUrl "http://work:8888"
      const originalProfile: ProfileConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'local-key',
      };
      await saveProfile('openai', 'work-vllm', originalProfile);

      // @step When I run the "/provider" command
      // @step And I select the "openai" provider
      // @step And I edit the "work-vllm" profile
      // @step And I change the baseUrl to "http://work:9000"
      const updatedProfile: ProfileConfig = {
        baseUrl: 'http://work:9000',
        apiKey: 'local-key',
      };

      // @step And I save the changes
      await saveProfile('openai', 'work-vllm', updatedProfile);

      // @step Then the config file should have "providers.openai.profiles.work-vllm.baseUrl" set to "http://work:9000"
      const configPath = join(setup.testDir, '.fspec', 'fspec-config.json');
      const configContent = await readFile(configPath, 'utf-8');
      const config = JSON.parse(configContent);

      expect(config.providers.openai.profiles['work-vllm'].baseUrl).toBe(
        'http://work:9000'
      );
    });
  });

  describe('Scenario: Delete a profile', () => {
    it('should remove profile from config file', async () => {
      // @step Given I have profiles "work-vllm" and "home-ollama" configured for "openai" provider
      const workProfile: ProfileConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'work-key',
      };
      const homeProfile: ProfileConfig = {
        baseUrl: 'http://localhost:11434',
        apiKey: 'local-key',
      };
      await saveProfile('openai', 'work-vllm', workProfile);
      await saveProfile('openai', 'home-ollama', homeProfile);

      // @step When I run the "/provider" command
      // @step And I select the "openai" provider
      // @step And I delete the "home-ollama" profile
      await deleteProfile('openai', 'home-ollama');

      // @step Then the config file should not contain "providers.openai.profiles.home-ollama"
      const configPath = join(setup.testDir, '.fspec', 'fspec-config.json');
      const configContent = await readFile(configPath, 'utf-8');
      const config = JSON.parse(configContent);

      expect(config.providers.openai.profiles['home-ollama']).toBeUndefined();

      // @step And the profile should no longer appear in /model selector
      // Note: This step is verified through TUI integration - the absence of the profile
      // in the config file means it won't appear in the model selector.
      // Full TUI verification is in AgentView-profile-sections.test.tsx

      // work-vllm should still exist
      expect(config.providers.openai.profiles['work-vllm']).toBeDefined();
    });
  });

  describe('Scenario: Get a specific profile', () => {
    it('should return profile config by name', async () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider
      const profileConfig: ProfileConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'local-key',
        contextWindow: 32768,
        maxOutputTokens: 8192,
      };
      await saveProfile('openai', 'work-vllm', profileConfig);

      // @step When I get the profile "work-vllm"
      const profile = await getProfile('openai', 'work-vllm');

      // @step Then I should receive the profile configuration
      expect(profile).toEqual({
        baseUrl: 'http://work:8888',
        apiKey: 'local-key',
        contextWindow: 32768,
        maxOutputTokens: 8192,
      });
    });

    it('should return undefined for non-existent profile', async () => {
      // @step Given I have no profiles configured for "openai" provider

      // @step When I get the profile "non-existent"
      const profile = await getProfile('openai', 'non-existent');

      // @step Then I should receive undefined
      expect(profile).toBeUndefined();
    });
  });

  // ============================================
  // PROFILE CONFIG OPTIONAL FIELDS
  // ============================================

  describe('Scenario: Profile with optional fields omitted', () => {
    it('should store profile with only required fields', async () => {
      // @step Given I create a minimal profile for "openai" provider
      const profileConfig: ProfileConfig = {
        baseUrl: 'http://localhost:11434',
        apiKey: 'local-key',
      };
      await saveProfile('openai', 'minimal-profile', profileConfig);

      // @step Then the profile should be saved without optional fields
      const profile = await getProfile('openai', 'minimal-profile');
      expect(profile).toEqual({
        baseUrl: 'http://localhost:11434',
        apiKey: 'local-key',
      });
      expect(profile?.contextWindow).toBeUndefined();
      expect(profile?.maxOutputTokens).toBeUndefined();
    });
  });

  // ============================================
  // MULTIPLE PROVIDERS WITH PROFILES
  // ============================================

  describe('Scenario: Profiles for multiple providers', () => {
    it('should store profiles independently per provider', async () => {
      // @step Given I have profiles for "openai" and "anthropic" providers
      const openaiProfile: ProfileConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'openai-key',
      };
      const anthropicProfile: ProfileConfig = {
        baseUrl: 'http://anthropic-proxy:8888',
        apiKey: 'anthropic-key',
      };

      await saveProfile('openai', 'work-vllm', openaiProfile);
      await saveProfile('anthropic', 'proxy-server', anthropicProfile);

      // @step Then each provider should have its own profiles
      const openaiProfiles = await loadProviderProfiles('openai');
      const anthropicProfiles = await loadProviderProfiles('anthropic');

      expect(openaiProfiles['work-vllm']).toBeDefined();
      expect(openaiProfiles['work-vllm'].baseUrl).toBe('http://work:8888');

      expect(anthropicProfiles['proxy-server']).toBeDefined();
      expect(anthropicProfiles['proxy-server'].baseUrl).toBe(
        'http://anthropic-proxy:8888'
      );
    });
  });

  // ============================================
  // EMPTY PROFILES
  // ============================================

  describe('Scenario: No profiles configured', () => {
    it('should return empty object when no profiles exist', async () => {
      // @step Given no profiles are configured for "openai" provider

      // @step When I load profiles for "openai"
      const profiles = await loadProviderProfiles('openai');

      // @step Then I should receive an empty object
      expect(profiles).toEqual({});
    });
  });

  // ============================================
  // MODEL SELECTOR PROFILE SECTIONS
  // ============================================

  describe('Scenario: Profiles appear as separate sections in model selector', () => {
    it('should generate profile section names correctly', async () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider:
      // | setting         | value              |
      // | baseUrl         | http://work:8888   |
      // | apiKey          | local-key          |
      // | contextWindow   | 32768              |
      // | maxOutputTokens | 8192               |
      const workProfile: ProfileConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'local-key',
        contextWindow: 32768,
        maxOutputTokens: 8192,
      };
      await saveProfile('openai', 'work-vllm', workProfile);

      // @step And I have a profile "home-ollama" configured for "openai" provider:
      // | setting         | value                  |
      // | baseUrl         | http://localhost:11434 |
      // | apiKey          | local-key              |
      const homeProfile: ProfileConfig = {
        baseUrl: 'http://localhost:11434',
        apiKey: 'local-key',
      };
      await saveProfile('openai', 'home-ollama', homeProfile);

      // @step When I run the "/model" command
      const profiles = await loadProviderProfiles('openai');

      // @step Then I should see a section "openai: work-vllm"
      const profileSectionNames = Object.keys(profiles).map(
        name => `openai: ${name}`
      );
      expect(profileSectionNames).toContain('openai: work-vllm');

      // @step And I should see a section "openai: home-ollama"
      expect(profileSectionNames).toContain('openai: home-ollama');

      // @step And these sections should appear alongside cloud providers like "anthropic"
      // Note: Cloud provider sections are verified separately in TUI tests
      expect(profileSectionNames.length).toBe(2);
    });
  });

  describe('Scenario: Profile section fetches models from local server', () => {
    it('should provide baseUrl for model fetching', async () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider:
      // | setting | value            |
      // | baseUrl | http://work:8888 |
      // | apiKey  | local-key        |
      const profile: ProfileConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'local-key',
      };
      await saveProfile('openai', 'work-vllm', profile);

      // @step And the local server at "http://work:8888" has models:
      // | model_id      |
      // | Qwen/Qwen3-80B |
      // | mistral-7b    |
      // Note: Actual server interaction is tested in NAPI integration tests

      // @step When I run the "/model" command
      // @step And I expand the "openai: work-vllm" section
      const loadedProfile = await getProfile('openai', 'work-vllm');

      // @step Then I should see "Qwen/Qwen3-80B" in the model list
      // @step And I should see "mistral-7b" in the model list
      // @step And these models should be fetched via modelsListLocalOpenai("http://work:8888")
      // Note: The baseUrl is what enables fetching from the local server
      expect(loadedProfile?.baseUrl).toBe('http://work:8888');
    });
  });

  describe('Scenario: Selecting model from profile creates session with profile config', () => {
    it('should provide all profile settings for session creation', async () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider:
      // | setting         | value              |
      // | baseUrl         | http://work:8888   |
      // | apiKey          | local-key          |
      // | contextWindow   | 32768              |
      // | maxOutputTokens | 8192               |
      const profile: ProfileConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'local-key',
        contextWindow: 32768,
        maxOutputTokens: 8192,
      };
      await saveProfile('openai', 'work-vllm', profile);

      // @step When I run the "/model" command
      // @step And I select "Qwen/Qwen3-80B" from the "openai: work-vllm" section
      const loadedProfile = await getProfile('openai', 'work-vllm');

      // @step Then sessionService should set environment variable "OPENAI_BASE_URL" to "http://work:8888"
      expect(loadedProfile?.baseUrl).toBe('http://work:8888');

      // @step And sessionService should set environment variable "OPENAI_API_KEY" to "local-key"
      expect(loadedProfile?.apiKey).toBe('local-key');

      // @step And sessionService should set environment variable "OPENAI_CONTEXT_WINDOW" to "32768"
      expect(loadedProfile?.contextWindow).toBe(32768);

      // @step And sessionService should set environment variable "OPENAI_MAX_OUTPUT_TOKENS" to "8192"
      expect(loadedProfile?.maxOutputTokens).toBe(8192);

      // @step And a session should be created with model "openai/Qwen/Qwen3-80B"
      // Note: Actual session creation is tested in sessionService integration tests
    });
  });

  describe('Scenario: Cloud provider section uses models.dev when no profile', () => {
    it('should return empty profiles for provider without configuration', async () => {
      // @step Given I have ANTHROPIC_API_KEY configured
      // Note: API key is managed separately via credentials system

      // @step And I have no profiles for "anthropic" provider
      const profiles = await loadProviderProfiles('anthropic');

      // @step When I run the "/model" command
      // @step And I expand the "anthropic" section
      // @step Then models should be fetched from models.dev
      // Note: When no profiles exist, TUI falls back to models.dev
      expect(profiles).toEqual({});

      // @step And I should see "claude-sonnet-4" in the model list
      // Note: models.dev integration is tested in NAPI tests
    });
  });

  describe('Scenario: Profile settings flow through to Rust provider', () => {
    it('should provide complete profile settings for Rust provider', async () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider:
      // | setting         | value              |
      // | baseUrl         | http://work:8888   |
      // | apiKey          | my-local-key       |
      // | contextWindow   | 32768              |
      // | maxOutputTokens | 8192               |
      const profile: ProfileConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'my-local-key',
        contextWindow: 32768,
        maxOutputTokens: 8192,
      };
      await saveProfile('openai', 'work-vllm', profile);

      // @step When I select a model from the "openai: work-vllm" section
      const loadedProfile = await getProfile('openai', 'work-vllm');

      // @step Then the Rust OpenAI provider should receive:
      // | env_var                  | value            |
      // | OPENAI_BASE_URL          | http://work:8888 |
      // | OPENAI_API_KEY           | my-local-key     |
      // | OPENAI_CONTEXT_WINDOW    | 32768            |
      // | OPENAI_MAX_OUTPUT_TOKENS | 8192             |
      // Note: Environment variable setting is handled by sessionService
      expect(loadedProfile).toEqual({
        baseUrl: 'http://work:8888',
        apiKey: 'my-local-key',
        contextWindow: 32768,
        maxOutputTokens: 8192,
      });
    });
  });

  describe('Scenario: Handle unreachable local server gracefully', () => {
    it('should store profile even when server is unreachable', async () => {
      // @step Given I have a profile "offline-server" configured for "openai" provider:
      // | setting | value                     |
      // | baseUrl | http://unreachable:8888   |
      // | apiKey  | local-key                 |
      const profile: ProfileConfig = {
        baseUrl: 'http://unreachable:8888',
        apiKey: 'local-key',
      };
      await saveProfile('openai', 'offline-server', profile);

      // @step When I run the "/model" command
      const loadedProfile = await getProfile('openai', 'offline-server');

      // @step Then the "openai: offline-server" section should show "(unreachable)"
      // Note: Server reachability check is done by TUI when fetching models
      // Profile itself is still stored and valid
      expect(loadedProfile).toBeDefined();
      expect(loadedProfile?.baseUrl).toBe('http://unreachable:8888');

      // @step And I should still be able to use other providers
      // Note: Each profile is independent - one failing doesn't affect others
    });
  });
});
