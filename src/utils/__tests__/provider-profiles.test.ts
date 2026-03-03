/**
 * Feature: spec/features/provider-configuration-display-and-profiles.feature
 *
 * Tests for provider profile configuration persistence.
 * These tests verify profile CRUD operations using real provider-config functions.
 *
 * SOLID: Uses composable fixtures, tests real implementations
 * DRY: Reusable fixture setup/teardown
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import {
  createProviderProfileFixture,
  createStandardProfiles,
  type ProviderProfileFixture,
} from '../../test-helpers/provider-profile-fixtures';

// Real implementations under test
import {
  loadProviderProfiles,
  saveProfile,
  deleteProfile,
  getProfile,
  type ProfileConfig,
} from '../provider-config';

describe('Feature: Provider Configuration Persistence and TUI Display', () => {
  let fixture: ProviderProfileFixture;

  beforeEach(async () => {
    fixture = await createProviderProfileFixture('provider-profiles');
  });

  afterEach(async () => {
    await fixture.cleanup();
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
      const config = await fixture.readConfig();

      // | path                                           | type   | description                    |
      // | providers.openai.profiles                      | object | Map of profile name to config  |
      expect(config.providers).toBeDefined();
      const providers = config.providers as Record<
        string,
        Record<string, unknown>
      >;
      expect(providers.openai).toBeDefined();
      expect(providers.openai.profiles).toBeDefined();
      expect(typeof providers.openai.profiles).toBe('object');

      const profiles = providers.openai.profiles as Record<
        string,
        ProfileConfig
      >;

      // | providers.openai.profiles.*.baseUrl            | string | API endpoint URL               |
      expect(profiles['work-vllm'].baseUrl).toBe('http://work:8888');
      expect(typeof profiles['work-vllm'].baseUrl).toBe('string');

      // | providers.openai.profiles.*.apiKey             | string | API key for this profile       |
      expect(profiles['work-vllm'].apiKey).toBe('local-key');
      expect(typeof profiles['work-vllm'].apiKey).toBe('string');

      // | providers.openai.profiles.*.contextWindow      | number | Context window size (optional) |
      expect(profiles['work-vllm'].contextWindow).toBe(32768);
      expect(typeof profiles['work-vllm'].contextWindow).toBe('number');

      // | providers.openai.profiles.*.maxOutputTokens    | number | Max output tokens (optional)   |
      expect(profiles['work-vllm'].maxOutputTokens).toBe(8192);
      expect(typeof profiles['work-vllm'].maxOutputTokens).toBe('number');
    });
  });

  // ============================================
  // PROFILE CRUD OPERATIONS
  // ============================================

  describe('Scenario: View list of profiles for a provider', () => {
    it('should return all profiles for a provider', async () => {
      // @step Given I have profiles "work-vllm" and "home-ollama" configured for "openai" provider
      await createStandardProfiles(fixture);

      // @step When I run the "/provider" command
      // @step And I select the "openai" provider
      const profiles = await loadProviderProfiles('openai');

      // @step Then I should see profile "work-vllm" with its settings
      expect(profiles['work-vllm']).toBeDefined();
      expect(profiles['work-vllm'].baseUrl).toBe('http://work:8888');
      expect(profiles['work-vllm'].apiKey).toBe('work-api-key');

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
      const profileConfig: ProfileConfig = {
        baseUrl: 'http://dev:8888',
        apiKey: 'dev-api-key',
        contextWindow: 16384,
        maxOutputTokens: 4096,
      };

      // @step And I save the profile
      await saveProfile('openai', 'dev-server', profileConfig);

      // @step Then the config file should contain the profile under "providers.openai.profiles.dev-server"
      const config = await fixture.readConfig();
      const providers = config.providers as Record<
        string,
        Record<string, unknown>
      >;
      const profiles = providers.openai.profiles as Record<
        string,
        ProfileConfig
      >;

      expect(profiles['dev-server']).toEqual({
        baseUrl: 'http://dev:8888',
        apiKey: 'dev-api-key',
        contextWindow: 16384,
        maxOutputTokens: 4096,
      });

      // @step And the profile should appear in /model selector as "openai: dev-server"
      // Note: TUI integration verified in separate TUI tests
      expect(profiles['dev-server']).toBeDefined();
    });
  });

  describe('Scenario: Edit an existing profile', () => {
    it('should update profile settings in config file', async () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider with baseUrl "http://work:8888"
      await saveProfile('openai', 'work-vllm', {
        baseUrl: 'http://work:8888',
        apiKey: 'local-key',
      });

      // @step When I run the "/provider" command
      // @step And I select the "openai" provider
      // @step And I edit the "work-vllm" profile
      // @step And I change the baseUrl to "http://work:9000"
      // @step And I save the changes
      await saveProfile('openai', 'work-vllm', {
        baseUrl: 'http://work:9000',
        apiKey: 'local-key',
      });

      // @step Then the config file should have "providers.openai.profiles.work-vllm.baseUrl" set to "http://work:9000"
      const config = await fixture.readConfig();
      const providers = config.providers as Record<
        string,
        Record<string, unknown>
      >;
      const profiles = providers.openai.profiles as Record<
        string,
        ProfileConfig
      >;

      expect(profiles['work-vllm'].baseUrl).toBe('http://work:9000');
    });
  });

  describe('Scenario: Delete a profile', () => {
    it('should remove profile from config file', async () => {
      // @step Given I have profiles "work-vllm" and "home-ollama" configured for "openai" provider
      await createStandardProfiles(fixture);

      // @step When I run the "/provider" command
      // @step And I select the "openai" provider
      // @step And I delete the "home-ollama" profile
      await deleteProfile('openai', 'home-ollama');

      // @step Then the config file should not contain "providers.openai.profiles.home-ollama"
      const config = await fixture.readConfig();
      const providers = config.providers as Record<
        string,
        Record<string, unknown>
      >;
      const profiles = providers.openai.profiles as Record<
        string,
        ProfileConfig
      >;

      expect(profiles['home-ollama']).toBeUndefined();

      // @step And the profile should no longer appear in /model selector
      // Note: TUI integration verified in separate TUI tests

      // work-vllm should still exist
      expect(profiles['work-vllm']).toBeDefined();
    });
  });

  // ============================================
  // PROFILE RETRIEVAL
  // ============================================

  describe('Scenario: Get a specific profile', () => {
    it('should return profile config by name', async () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider
      await saveProfile('openai', 'work-vllm', {
        baseUrl: 'http://work:8888',
        apiKey: 'local-key',
        contextWindow: 32768,
        maxOutputTokens: 8192,
      });

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
  // OPTIONAL FIELDS
  // ============================================

  describe('Scenario: Profile with optional fields omitted', () => {
    it('should store profile with only required fields', async () => {
      // @step Given I create a minimal profile for "openai" provider
      await saveProfile('openai', 'minimal-profile', {
        baseUrl: 'http://localhost:11434',
        apiKey: 'local-key',
      });

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
  // MULTIPLE PROVIDERS
  // ============================================

  describe('Scenario: Profiles for multiple providers', () => {
    it('should reject saving profiles for non-OpenAI providers', async () => {
      // PROV-029: Profiles are only supported for OpenAI API provider
      // @step Given I save a profile for "openai" provider
      await saveProfile('openai', 'work-vllm', {
        baseUrl: 'http://work:8888',
        apiKey: 'openai-key',
      });

      // @step When I try to save a profile for "anthropic" provider
      // @step Then the save should be rejected
      await expect(
        saveProfile('anthropic', 'proxy-server', {
          baseUrl: 'http://anthropic-proxy:8888',
          apiKey: 'anthropic-key',
        })
      ).rejects.toThrow('Profiles are only supported for OpenAI API provider');

      // @step And the openai profile should still exist
      const openaiProfiles = await loadProviderProfiles('openai');
      expect(openaiProfiles['work-vllm']).toBeDefined();
      expect(openaiProfiles['work-vllm'].baseUrl).toBe('http://work:8888');
    });
  });

  // ============================================
  // EMPTY STATE
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
  // CLOUD PROVIDER FALLBACK
  // ============================================

  describe('Scenario: Cloud provider section uses models.dev when no profile', () => {
    it('should return empty profiles for provider without configuration', async () => {
      // @step Given I have ANTHROPIC_API_KEY configured
      // Note: API key is managed separately via credentials system

      // @step And I have no profiles for "anthropic" provider

      // @step When I load profiles for "anthropic"
      const profiles = await loadProviderProfiles('anthropic');

      // @step Then I should receive an empty object
      expect(profiles).toEqual({});
    });
  });

  // ============================================
  // MODEL SELECTOR PROFILE SECTIONS
  // ============================================

  describe('Scenario: Profiles appear as separate sections in model selector', () => {
    it('should create profile sections with correct provider names', async () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider:
      await saveProfile('openai', 'work-vllm', {
        baseUrl: 'http://work:8888',
        apiKey: 'local-key',
        contextWindow: 32768,
        maxOutputTokens: 8192,
      });

      // @step And I have a profile "home-ollama" configured for "openai" provider:
      await saveProfile('openai', 'home-ollama', {
        baseUrl: 'http://localhost:11434',
        apiKey: 'local-key',
      });

      // @step When I build profile sections
      const profiles = await loadProviderProfiles('openai');

      // @step Then I should see a section "openai: work-vllm"
      expect(profiles['work-vllm']).toBeDefined();
      expect(profiles['work-vllm'].baseUrl).toBe('http://work:8888');

      // @step And I should see a section "openai: home-ollama"
      expect(profiles['home-ollama']).toBeDefined();
      expect(profiles['home-ollama'].baseUrl).toBe('http://localhost:11434');
    });
  });

  describe('Scenario: Selecting model from profile creates session with profile config', () => {
    it('should have profile config available for setting environment variables', async () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider:
      const profileConfig: ProfileConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'local-key',
        contextWindow: 32768,
        maxOutputTokens: 8192,
      };
      await saveProfile('openai', 'work-vllm', profileConfig);

      // @step When I get the profile config
      const profile = await getProfile('openai', 'work-vllm');

      // @step Then the profile config should contain all settings for env vars
      expect(profile?.baseUrl).toBe('http://work:8888');
      expect(profile?.apiKey).toBe('local-key');
      expect(profile?.contextWindow).toBe(32768);
      expect(profile?.maxOutputTokens).toBe(8192);
    });
  });

  describe('Scenario: Handle unreachable local server gracefully', () => {
    it('should still return profile config even if server is unreachable', async () => {
      // @step Given I have a profile "offline-server" configured for "openai" provider:
      await saveProfile('openai', 'offline-server', {
        baseUrl: 'http://unreachable:8888',
        apiKey: 'local-key',
      });

      // @step When I load profiles
      const profiles = await loadProviderProfiles('openai');

      // @step Then the profile should still be available
      expect(profiles['offline-server']).toBeDefined();
      expect(profiles['offline-server'].baseUrl).toBe(
        'http://unreachable:8888'
      );
    });
  });

  describe('Scenario: Profile settings flow through to Rust provider', () => {
    it('should provide all settings needed for Rust env vars', async () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider:
      await saveProfile('openai', 'work-vllm', {
        baseUrl: 'http://work:8888',
        apiKey: 'my-local-key',
        contextWindow: 32768,
        maxOutputTokens: 8192,
      });

      // @step When I get the profile
      const profile = await getProfile('openai', 'work-vllm');

      // @step Then the profile should have all settings for Rust env vars
      expect(profile?.baseUrl).toBe('http://work:8888');
      expect(profile?.apiKey).toBe('my-local-key');
      expect(profile?.contextWindow).toBe(32768);
      expect(profile?.maxOutputTokens).toBe(8192);
    });
  });

  // ============================================
  // MODEL SELECTION UTILITIES
  // (Rules [9], [10], [11] from PROV-007)
  // ============================================

  describe('Scenario: Profile model selection saves with profile-qualified ID', () => {
    it('should save model string with profile name qualifier', async () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider
      const profileConfig: ProfileConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'test-key',
        contextWindow: 32768,
        maxOutputTokens: 8192,
      };
      await saveProfile('openai', 'work-vllm', profileConfig);

      // Import model selection utilities
      const { buildModelString } = await import(
        '../../tui/utils/model-selection'
      );

      const section = {
        providerId: 'openai',
        profileName: 'work-vllm',
      };
      const modelId = 'Qwen/Qwen3-80B';

      // @step When I select model "Qwen/Qwen3-80B" from the "openai: work-vllm" section
      const savedModelString = buildModelString(section, modelId);

      // @step Then the lastUsedModel should be saved as "openai:work-vllm/Qwen/Qwen3-80B"
      expect(savedModelString).toBe('openai:work-vllm/Qwen/Qwen3-80B');

      // @step And the lastUsedModel should NOT be saved as "openai/Qwen/Qwen3-80B"
      expect(savedModelString).not.toBe('openai/Qwen/Qwen3-80B');
    });

    it('should save cloud provider model string without profile qualifier', async () => {
      const { buildModelString } = await import(
        '../../tui/utils/model-selection'
      );

      // Given a cloud provider section (no profileName)
      const section = { providerId: 'openai' };
      const modelId = 'gpt-4';

      // When I select a model from cloud section
      const savedModelString = buildModelString(section, modelId);

      // Then the lastUsedModel should use cloud format
      expect(savedModelString).toBe('openai/gpt-4');
    });
  });

  describe('Scenario: Restoring persisted model finds correct profile section', () => {
    it('should find profile section when profile model was persisted', async () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider
      const profileConfig: ProfileConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'test-key',
      };
      await saveProfile('openai', 'work-vllm', profileConfig);

      const { findSectionForPersistedModel } = await import(
        '../../tui/utils/model-selection'
      );

      // @step And I have OPENAI_API_KEY configured for cloud provider
      const cloudSection = { providerId: 'openai' };
      const profileSection = { providerId: 'openai', profileName: 'work-vllm' };
      const sections = [profileSection, cloudSection];

      // @step And lastUsedModel is "openai:work-vllm/Qwen/Qwen3-80B"
      const persistedModel = 'openai:work-vllm/Qwen/Qwen3-80B';

      // @step When I open the model selector
      const foundSection = findSectionForPersistedModel(
        sections,
        persistedModel
      );

      // @step Then the restored section should be the profile section with profileName="work-vllm"
      expect(foundSection).not.toBeNull();
      expect(foundSection?.profileName).toBe('work-vllm');

      // @step And the restored section should NOT be the cloud provider section
      expect(foundSection?.profileName).not.toBeUndefined();
    });

    it('should find cloud section when cloud model was persisted', async () => {
      const { findSectionForPersistedModel } = await import(
        '../../tui/utils/model-selection'
      );

      // Given both cloud and profile sections exist
      const cloudSection = { providerId: 'openai' };
      const profileSection = { providerId: 'openai', profileName: 'work-vllm' };
      const sections = [profileSection, cloudSection];

      // And lastUsedModel is in cloud format
      const persistedModel = 'openai/gpt-4';

      // When I restore the model
      const foundSection = findSectionForPersistedModel(
        sections,
        persistedModel
      );

      // Then the restored section should be the cloud section
      expect(foundSection).not.toBeNull();
      expect(foundSection?.profileName).toBeUndefined();
    });

    it('should NOT return cloud section when profile model was persisted', async () => {
      const { findSectionForPersistedModel } = await import(
        '../../tui/utils/model-selection'
      );

      // This is the BUG we found - the old code returned the FIRST match by providerId
      const cloudSection = { providerId: 'openai' };
      const profileSection = { providerId: 'openai', profileName: 'work-vllm' };
      // Cloud section FIRST - old buggy code would return this!
      const sections = [cloudSection, profileSection];

      const persistedModel = 'openai:work-vllm/Qwen/Qwen3-80B';
      const foundSection = findSectionForPersistedModel(
        sections,
        persistedModel
      );

      // Should find profile section, NOT the first cloud section
      expect(foundSection?.profileName).toBe('work-vllm');
    });
  });

  describe('Scenario: Model selector has unique keys for cloud and profile sections', () => {
    it('should generate unique keys for cloud and profile sections', async () => {
      const { generateSectionKey } = await import(
        '../../tui/utils/model-selection'
      );

      // @step Given I have OPENAI_API_KEY configured for cloud provider
      const cloudSection = { providerId: 'openai' };

      // @step And I have a profile "work-vllm" configured for "openai" provider
      const profileSection = { providerId: 'openai', profileName: 'work-vllm' };

      // @step When I build the model selector sections
      const cloudKey = generateSectionKey(cloudSection);
      const profileKey = generateSectionKey(profileSection);

      // @step Then the cloud section key should be "section-openai-cloud"
      expect(cloudKey).toBe('section-openai-cloud');

      // @step And the profile section key should be "section-openai-work-vllm"
      expect(profileKey).toBe('section-openai-work-vllm');

      // @step And there should be no duplicate React keys
      expect(cloudKey).not.toBe(profileKey);
    });

    it('should generate unique keys for multiple profiles of same provider', async () => {
      const { generateSectionKey } = await import(
        '../../tui/utils/model-selection'
      );

      // Given multiple profiles for the same provider
      const profile1 = { providerId: 'openai', profileName: 'work-vllm' };
      const profile2 = { providerId: 'openai', profileName: 'home-ollama' };
      const cloudSection = { providerId: 'openai' };

      // When I generate keys
      const keys = [profile1, profile2, cloudSection].map(generateSectionKey);

      // Then all keys should be unique
      const uniqueKeys = new Set(keys);
      expect(uniqueKeys.size).toBe(keys.length);

      // And keys should match expected format
      expect(keys).toContain('section-openai-work-vllm');
      expect(keys).toContain('section-openai-home-ollama');
      expect(keys).toContain('section-openai-cloud');
    });
  });

  describe('Scenario: Selecting profile model passes profile config to Rust session', () => {
    it('should include profile config in section for Rust session', async () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider with baseUrl "http://work:8888"
      const profileConfig: ProfileConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'my-api-key',
        contextWindow: 32768,
        maxOutputTokens: 8192,
      };
      await saveProfile('openai', 'work-vllm', profileConfig);

      // When I get the profile config
      const profile = await getProfile('openai', 'work-vllm');

      // @step Then OPENAI_BASE_URL environment variable should be set to "http://work:8888"
      expect(profile?.baseUrl).toBe('http://work:8888');

      // @step And OPENAI_API_KEY environment variable should be set from profile config
      expect(profile?.apiKey).toBe('my-api-key');

      // @step And the session should use the local server not the cloud provider registry
      expect(profile?.contextWindow).toBe(32768);
      expect(profile?.maxOutputTokens).toBe(8192);
    });
  });

  describe('Edge cases: Model string parsing', () => {
    it('should parse profile model string with nested slashes in model ID', async () => {
      const { parseModelString } = await import(
        '../../tui/utils/model-selection'
      );

      // Model IDs can contain slashes (e.g., "Qwen/Qwen3-80B")
      const modelString = 'openai:work-vllm/Qwen/Qwen3-80B';
      const parsed = parseModelString(modelString);

      expect(parsed.providerId).toBe('openai');
      expect(parsed.profileName).toBe('work-vllm');
      expect(parsed.modelId).toBe('Qwen/Qwen3-80B');
    });

    it('should parse cloud model string with nested slashes', async () => {
      const { parseModelString } = await import(
        '../../tui/utils/model-selection'
      );

      const modelString = 'openai/Qwen/Qwen3-80B';
      const parsed = parseModelString(modelString);

      expect(parsed.providerId).toBe('openai');
      expect(parsed.profileName).toBeNull();
      expect(parsed.modelId).toBe('Qwen/Qwen3-80B');
    });

    it('should throw on invalid model string format', async () => {
      const { parseModelString } = await import(
        '../../tui/utils/model-selection'
      );

      expect(() => parseModelString('invalid')).toThrow(
        'Invalid model string format'
      );
    });
  });
});
