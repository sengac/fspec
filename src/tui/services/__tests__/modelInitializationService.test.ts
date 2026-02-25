/**
 * Feature: spec/features/screen-component-integration.feature
 *
 * TUI-075: Model Initialization Service Integration Tests
 *
 * Tests for the model initialization service that:
 * 1. Loads models from NAPI
 * 2. Loads profiles for local servers
 * 3. Restores persisted model selection
 * 4. Sets default model when no persisted model exists
 *
 * Test Strategy:
 * - Use REAL file system operations via test fixtures
 * - Mock ONLY the NAPI network boundary (modelsListAll, modelsListLocalOpenai)
 * - Verify store state changes
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdir, writeFile } from 'fs/promises';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../../test-helpers/universal-test-setup';
import { useModelStore } from '../../store/modelStore';

// =============================================================================
// NAPI MOCKS - Only mock the network boundary
// =============================================================================

const napiMocks = vi.hoisted(() => ({
  modelsListAll: vi.fn(),
  modelsListLocalOpenai: vi.fn(),
}));

vi.mock('@sengac/codelet-napi', async importOriginal => {
  const original =
    await importOriginal<typeof import('@sengac/codelet-napi')>();
  return {
    ...original,
    modelsListAll: () => napiMocks.modelsListAll(),
    modelsListLocalOpenai: (baseUrl: string) =>
      napiMocks.modelsListLocalOpenai(baseUrl),
  };
});

// Logger mock - silence output
vi.mock('../../../utils/logger', () => ({
  logger: { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() },
}));

// Import service AFTER mocks are set up
import { initializeModels } from '../modelInitializationService';

// =============================================================================
// TEST DATA FIXTURES
// =============================================================================

function createAnthropicProvider() {
  return {
    providerId: 'anthropic',
    providerName: 'Anthropic',
    models: [
      {
        id: 'claude-sonnet-4-20250514',
        name: 'Claude Sonnet 4',
        reasoning: true,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 200000,
        maxOutput: 16000,
        hasVision: true,
      },
      {
        id: 'claude-opus-4-20250514',
        name: 'Claude Opus 4',
        reasoning: true,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 200000,
        maxOutput: 32000,
        hasVision: true,
      },
    ],
  };
}

function createOpenAIProvider() {
  return {
    providerId: 'openai',
    providerName: 'OpenAI',
    models: [
      {
        id: 'gpt-4o',
        name: 'GPT-4o',
        reasoning: false,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 128000,
        maxOutput: 16384,
        hasVision: true,
      },
    ],
  };
}

function createGoogleProvider() {
  return {
    providerId: 'google',
    providerName: 'Google',
    models: [
      {
        id: 'gemini-2.0-flash',
        name: 'Gemini 2.0 Flash',
        reasoning: false,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 1000000,
        maxOutput: 8192,
        hasVision: true,
      },
    ],
  };
}

// =============================================================================
// TESTS
// =============================================================================

describe('Feature: Model Initialization Service', () => {
  let setup: TestDirectorySetup;
  let originalHome: string | undefined;
  let originalCwd: string;
  let originalEnvVars: Record<string, string | undefined>;

  // Env vars that might provide credentials
  const credentialEnvVars = [
    'ANTHROPIC_API_KEY',
    'CLAUDE_CODE_OAUTH_TOKEN',
    'OPENAI_API_KEY',
    'GOOGLE_API_KEY',
    'GEMINI_API_KEY',
  ];

  beforeEach(async () => {
    // Reset Zustand store
    useModelStore.getState().reset();

    // Reset mocks
    napiMocks.modelsListAll.mockReset();
    napiMocks.modelsListLocalOpenai.mockReset();

    // Save and clear credential env vars for clean testing
    originalEnvVars = {};
    for (const envVar of credentialEnvVars) {
      originalEnvVars[envVar] = process.env[envVar];
      delete process.env[envVar];
    }

    // Setup test directory
    setup = await setupTestDirectory('model-init-service');
    originalHome = process.env.HOME;
    originalCwd = process.cwd();

    // Override HOME
    process.env.HOME = setup.testDir;

    // Change to test directory so .env file lookup doesn't find real credentials
    process.chdir(setup.testDir);

    // Create .fspec directory structure
    await mkdir(join(setup.testDir, '.fspec', 'credentials'), {
      recursive: true,
    });
  });

  afterEach(async () => {
    // Restore env vars
    for (const envVar of credentialEnvVars) {
      if (originalEnvVars[envVar] !== undefined) {
        process.env[envVar] = originalEnvVars[envVar];
      } else {
        delete process.env[envVar];
      }
    }

    // Restore HOME and cwd
    process.env.HOME = originalHome;
    process.chdir(originalCwd);
    await setup.cleanup();
  });

  describe('Scenario: Initialize models with cloud providers and credentials', () => {
    it('should load models from NAPI and select first available as default', async () => {
      // @step Given I have credentials for anthropic
      const credentialsContent = {
        version: 1,
        providers: {
          anthropic: {
            apiKey: 'sk-ant-test-key',
            lastUpdated: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'credentials', 'credentials.json'),
        JSON.stringify(credentialsContent, null, 2),
        { mode: 0o600 }
      );

      // @step And NAPI returns anthropic models
      napiMocks.modelsListAll.mockResolvedValue([createAnthropicProvider()]);

      // @step When I call initializeModels
      const result = await initializeModels();

      // @step Then models should be loaded
      expect(result.sections.length).toBe(1);
      expect(result.sections[0].providerId).toBe('anthropic');
      expect(result.sections[0].models.length).toBe(2);

      // @step And first model should be selected as default
      expect(result.currentModel).not.toBeNull();
      expect(result.currentModel?.providerId).toBe('anthropic');
      expect(result.currentModel?.modelId).toBe('claude-sonnet-4');
      expect(result.currentModel?.displayName).toBe('Claude Sonnet 4');

      // @step And currentProvider should be set
      expect(result.currentProvider).toBe('claude');

      // @step And persistedModelRestored should be false
      expect(result.persistedModelRestored).toBe(false);

      // @step And store should be updated
      const store = useModelStore.getState();
      expect(store.modelsInitialized).toBe(true);
      expect(store.providerSections.length).toBe(1);
      expect(store.currentModel?.modelId).toBe('claude-sonnet-4');
    });
  });

  describe('Scenario: Restore persisted model selection from config', () => {
    it('should restore persisted model if available', async () => {
      // @step Given I have credentials for anthropic
      const credentialsContent = {
        version: 1,
        providers: {
          anthropic: {
            apiKey: 'sk-ant-test-key',
            lastUpdated: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'credentials', 'credentials.json'),
        JSON.stringify(credentialsContent, null, 2),
        { mode: 0o600 }
      );

      // @step And I have a persisted model selection for claude-opus-4
      const configContent = {
        tui: {
          lastUsedModel: 'anthropic/claude-opus-4',
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'fspec-config.json'),
        JSON.stringify(configContent, null, 2)
      );

      // @step And NAPI returns anthropic models
      napiMocks.modelsListAll.mockResolvedValue([createAnthropicProvider()]);

      // @step When I call initializeModels
      const result = await initializeModels();

      // @step Then the persisted model should be selected
      expect(result.currentModel?.modelId).toBe('claude-opus-4');
      expect(result.currentModel?.displayName).toBe('Claude Opus 4');

      // @step And persistedModelRestored should be true
      expect(result.persistedModelRestored).toBe(true);
    });
  });

  describe('Scenario: Fall back to default when persisted model is unavailable', () => {
    it('should select first available when persisted model has no credentials', async () => {
      // @step Given I have credentials for openai but NOT anthropic
      const credentialsContent = {
        version: 1,
        providers: {
          openai: {
            apiKey: 'sk-openai-test-key',
            lastUpdated: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'credentials', 'credentials.json'),
        JSON.stringify(credentialsContent, null, 2),
        { mode: 0o600 }
      );

      // @step And I have a persisted model selection for anthropic
      const configContent = {
        tui: {
          lastUsedModel: 'anthropic/claude-sonnet-4',
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'fspec-config.json'),
        JSON.stringify(configContent, null, 2)
      );

      // @step And NAPI returns both providers
      napiMocks.modelsListAll.mockResolvedValue([
        createAnthropicProvider(),
        createOpenAIProvider(),
      ]);

      // @step When I call initializeModels
      const result = await initializeModels();

      // @step Then only openai should be in sections (anthropic has no credentials)
      expect(result.sections.length).toBe(1);
      expect(result.sections[0].providerId).toBe('openai');

      // @step And openai should be selected (only provider with credentials)
      expect(result.currentModel?.providerId).toBe('openai');
      expect(result.currentModel?.modelId).toBe('gpt-4o');

      // @step And persistedModelRestored should be false
      expect(result.persistedModelRestored).toBe(false);
    });
  });

  describe('Scenario: Multiple providers with credentials', () => {
    it('should include all providers with credentials in sections', async () => {
      // @step Given I have credentials for anthropic, openai, and gemini
      const credentialsContent = {
        version: 1,
        providers: {
          anthropic: {
            apiKey: 'sk-ant-test-key',
            lastUpdated: new Date().toISOString(),
          },
          openai: {
            apiKey: 'sk-openai-test-key',
            lastUpdated: new Date().toISOString(),
          },
          gemini: {
            apiKey: 'gemini-test-key',
            lastUpdated: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'credentials', 'credentials.json'),
        JSON.stringify(credentialsContent, null, 2),
        { mode: 0o600 }
      );

      // @step And NAPI returns all three providers
      napiMocks.modelsListAll.mockResolvedValue([
        createAnthropicProvider(),
        createOpenAIProvider(),
        createGoogleProvider(),
      ]);

      // @step When I call initializeModels
      const result = await initializeModels();

      // @step Then all three providers should be in sections
      expect(result.sections.length).toBe(3);
      const providerIds = result.sections.map(s => s.providerId);
      expect(providerIds).toContain('anthropic');
      expect(providerIds).toContain('openai');
      expect(providerIds).toContain('google');

      // @step And availableProviders should have internal names
      expect(result.availableProviders).toContain('claude');
      expect(result.availableProviders).toContain('openai');
      expect(result.availableProviders).toContain('gemini');
    });
  });

  describe('Scenario: Skip initialization if already initialized', () => {
    it('should return cached data if models already initialized', async () => {
      // @step Given models have already been initialized
      const store = useModelStore.getState();
      store.setProviderSections([
        {
          providerId: 'anthropic',
          providerName: 'Anthropic',
          internalName: 'claude',
          models: [],
          hasCredentials: true,
        },
      ]);
      store.setCurrentModel({
        providerId: 'anthropic',
        modelId: 'claude-sonnet-4',
        apiModelId: 'claude-sonnet-4-20250514',
        displayName: 'Claude Sonnet 4',
        reasoning: true,
        hasVision: true,
        contextWindow: 200000,
        maxOutput: 16000,
      });
      store.setModelsInitialized(true);

      // @step When I call initializeModels again
      const result = await initializeModels();

      // @step Then NAPI should NOT be called
      expect(napiMocks.modelsListAll).not.toHaveBeenCalled();

      // @step And cached data should be returned
      expect(result.currentModel?.modelId).toBe('claude-sonnet-4');
      expect(result.persistedModelRestored).toBe(false);
    });
  });

  describe('Scenario: Handle NAPI failure gracefully', () => {
    it('should return empty sections when NAPI fails', async () => {
      // @step Given NAPI fails to load models
      napiMocks.modelsListAll.mockRejectedValue(new Error('Network error'));

      // @step When I call initializeModels
      const result = await initializeModels();

      // @step Then sections should be empty
      expect(result.sections.length).toBe(0);
      expect(result.currentModel).toBeNull();

      // @step And store should still be marked as initialized
      const store = useModelStore.getState();
      expect(store.modelsInitialized).toBe(true);
    });
  });

  // =============================================================================
  // BUG-097: Profile-based model restoration tests
  // Feature: spec/features/profile-based-model-selection-not-restored-on-session-startup.feature
  // =============================================================================

  describe('Scenario: Restore persisted profile-based model on new session', () => {
    it('should restore profile model with correct providerId, profileName, and modelId', async () => {
      // @step Given ~/.fspec/fspec-config.json contains "tui.lastUsedModel": "openai:work-vllm/Qwen/Qwen3-80B"
      const configContent = {
        tui: {
          lastUsedModel: 'openai:work-vllm/Qwen/Qwen3-80B',
        },
        providers: {
          openai: {
            profiles: {
              'work-vllm': {
                baseUrl: 'http://localhost:8888',
                apiKey: 'test-key',
              },
            },
          },
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'fspec-config.json'),
        JSON.stringify(configContent, null, 2)
      );

      // @step And I have a profile "work-vllm" configured for "openai" provider
      // (done in config above)

      // @step And the profile's local server is reachable
      napiMocks.modelsListLocalOpenai.mockResolvedValue([
        'Qwen/Qwen3-80B',
        'Qwen/Qwen3-32B',
      ]);

      // @step And NAPI returns cloud providers (which we don't have credentials for)
      napiMocks.modelsListAll.mockResolvedValue([createOpenAIProvider()]);

      // @step When I call initializeModels()
      const result = await initializeModels();

      // @step Then the restored model should have providerId="openai"
      expect(result.currentModel?.providerId).toBe('openai');

      // @step And the restored model should have profileName="work-vllm"
      expect(result.currentModel?.profileName).toBe('work-vllm');

      // @step And the restored model should have modelId containing "Qwen"
      expect(result.currentModel?.modelId).toContain('Qwen');

      // @step And persistedModelRestored should be true
      expect(result.persistedModelRestored).toBe(true);
    });
  });

  describe('Scenario: Restore persisted cloud model on new session', () => {
    it('should restore cloud model with providerId, null profileName, and modelId', async () => {
      // @step Given ~/.fspec/fspec-config.json contains "tui.lastUsedModel": "anthropic/claude-sonnet-4"
      const configContent = {
        tui: {
          lastUsedModel: 'anthropic/claude-sonnet-4',
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'fspec-config.json'),
        JSON.stringify(configContent, null, 2)
      );

      // @step And I have credentials for anthropic
      const credentialsContent = {
        version: 1,
        providers: {
          anthropic: {
            apiKey: 'sk-ant-test-key',
            lastUpdated: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'credentials', 'credentials.json'),
        JSON.stringify(credentialsContent, null, 2),
        { mode: 0o600 }
      );

      // @step And NAPI returns anthropic models
      napiMocks.modelsListAll.mockResolvedValue([createAnthropicProvider()]);

      // @step When I call initializeModels()
      const result = await initializeModels();

      // @step Then the restored model should have providerId="anthropic"
      expect(result.currentModel?.providerId).toBe('anthropic');

      // @step And the restored model should have profileName=null (undefined)
      expect(result.currentModel?.profileName).toBeUndefined();

      // @step And the restored model should have modelId="claude-sonnet-4"
      expect(result.currentModel?.modelId).toBe('claude-sonnet-4');

      // @step And persistedModelRestored should be true
      expect(result.persistedModelRestored).toBe(true);
    });
  });
});
