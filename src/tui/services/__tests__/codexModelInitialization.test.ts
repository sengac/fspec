/**
 * Feature: spec/features/codex-model-selector-integration.feature
 *
 * PROV-018: Codex Model Selector Integration Tests
 *
 * Tests that Codex models (from models.dev's OpenAI provider) appear
 * in the model selector when OAuth tokens exist, using providerId='codex'
 * for correct session routing to the Rust CodexProvider.
 *
 * Test Strategy:
 * - Mock NAPI network boundary (modelsListAll) and codexOauthGetTokens
 * - Use REAL file system operations via test fixtures
 * - Verify provider sections contain synthetic Codex section
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
// NAPI MOCKS
// =============================================================================

const napiMocks = vi.hoisted(() => ({
  modelsListAll: vi.fn(),
  modelsListLocalOpenai: vi.fn(),
  codexOauthGetTokens: vi.fn(),
}));

vi.mock('@sengac/codelet-napi', async importOriginal => {
  const original =
    await importOriginal<typeof import('@sengac/codelet-napi')>();
  return {
    ...original,
    modelsListAll: () => napiMocks.modelsListAll(),
    modelsListLocalOpenai: (baseUrl: string) =>
      napiMocks.modelsListLocalOpenai(baseUrl),
    codexOauthGetTokens: () => napiMocks.codexOauthGetTokens(),
  };
});

vi.mock('../../../utils/logger', () => ({
  logger: { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() },
}));

import { initializeModels } from '../modelInitializationService';
import { buildModelString } from '../../utils/model-selection';

// =============================================================================
// TEST DATA FIXTURES
// =============================================================================

function createOpenAIProviderWithCodexModels() {
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
      {
        id: 'gpt-5.3-codex',
        name: 'GPT-5.3 Codex',
        reasoning: false,
        toolCall: true,
        attachment: false,
        temperature: true,
        contextWindow: 400000,
        maxOutput: 128000,
        hasVision: false,
      },
      {
        id: 'gpt-5.2-codex',
        name: 'GPT-5.2 Codex',
        reasoning: false,
        toolCall: true,
        attachment: false,
        temperature: true,
        contextWindow: 400000,
        maxOutput: 128000,
        hasVision: false,
      },
      {
        id: 'codex-mini-latest',
        name: 'Codex Mini',
        reasoning: false,
        toolCall: true,
        attachment: false,
        temperature: true,
        contextWindow: 200000,
        maxOutput: 100000,
        hasVision: false,
      },
    ],
  };
}

/**
 * OpenAI provider where ALL models are codex models (no non-codex models).
 * Tests edge case: empty OpenAI section should be removed after extraction.
 */
function createOpenAIProviderAllCodexModels() {
  return {
    providerId: 'openai',
    providerName: 'OpenAI',
    models: [
      {
        id: 'gpt-5.3-codex',
        name: 'GPT-5.3 Codex',
        reasoning: false,
        toolCall: true,
        attachment: false,
        temperature: true,
        contextWindow: 400000,
        maxOutput: 128000,
        hasVision: false,
      },
      {
        id: 'codex-mini-latest',
        name: 'Codex Mini',
        reasoning: false,
        toolCall: true,
        attachment: false,
        temperature: true,
        contextWindow: 200000,
        maxOutput: 100000,
        hasVision: false,
      },
    ],
  };
}

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
    ],
  };
}

// =============================================================================
// TESTS
// =============================================================================

describe('Feature: Codex Model Selector Integration', () => {
  let setup: TestDirectorySetup;
  let originalHome: string | undefined;
  let originalCwd: string;
  let originalEnvVars: Record<string, string | undefined>;

  const credentialEnvVars = [
    'ANTHROPIC_API_KEY',
    'OPENAI_API_KEY',
    'CODEX_API_KEY',
    'GOOGLE_API_KEY',
    'GEMINI_API_KEY',
  ];

  beforeEach(async () => {
    useModelStore.getState().reset();
    napiMocks.modelsListAll.mockReset();
    napiMocks.modelsListLocalOpenai.mockReset();
    napiMocks.codexOauthGetTokens.mockReset();

    originalEnvVars = {};
    for (const envVar of credentialEnvVars) {
      originalEnvVars[envVar] = process.env[envVar];
      delete process.env[envVar];
    }

    setup = await setupTestDirectory('codex-model-init');
    originalHome = process.env.HOME;
    originalCwd = process.cwd();
    process.env.HOME = setup.testDir;
    process.chdir(setup.testDir);

    await mkdir(join(setup.testDir, '.fspec', 'credentials'), {
      recursive: true,
    });
  });

  afterEach(async () => {
    for (const envVar of credentialEnvVars) {
      if (originalEnvVars[envVar] !== undefined) {
        process.env[envVar] = originalEnvVars[envVar];
      } else {
        delete process.env[envVar];
      }
    }
    process.env.HOME = originalHome;
    process.chdir(originalCwd);
    await setup.cleanup();
  });

  describe('Scenario: Codex models appear in model selector when OAuth tokens exist', () => {
    it('should extract codex models from OpenAI into a separate Codex section', async () => {
      // @step Given I have authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue({
        accessToken: 'test-access-token',
        refreshToken: 'test-refresh-token',
        expiresAt: Date.now() + 3600000,
      });

      // @step And models.dev returns OpenAI provider with codex models
      napiMocks.modelsListAll.mockResolvedValue([
        createOpenAIProviderWithCodexModels(),
      ]);

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then I should see a Codex (ChatGPT) section with codex models
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeDefined();
      expect(codexSection!.models.length).toBe(3);
      expect(codexSection!.providerName).toBe('Codex (ChatGPT)');
      expect(codexSection!.hasCredentials).toBe(true);

      // Verify all models contain 'codex' in their IDs
      for (const model of codexSection!.models) {
        expect(model.id.toLowerCase()).toContain('codex');
      }

      // @step And the Codex section should use providerId codex
      expect(codexSection!.providerId).toBe('codex');
    });
  });

  describe('Scenario: No Codex section when OAuth tokens absent', () => {
    it('should not create a Codex section when no OAuth tokens exist', async () => {
      // @step Given I have not authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue(null);

      // @step And I have no OpenAI API key configured
      // (no credentials set up — env vars cleared in beforeEach)

      // @step When models are loaded for the model selector
      napiMocks.modelsListAll.mockResolvedValue([
        createOpenAIProviderWithCodexModels(),
      ]);
      const result = await initializeModels();

      // @step Then I should not see any Codex or OpenAI section in the model selector
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      const openaiSection = result.sections.find(
        s => s.providerId === 'openai'
      );
      expect(codexSection).toBeUndefined();
      expect(openaiSection).toBeUndefined();
    });
  });

  describe('Scenario: Both OpenAI API key and Codex OAuth show separate sections', () => {
    it('should show both OpenAI and Codex sections when both credentials exist', async () => {
      // @step Given I have authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue({
        accessToken: 'test-access-token',
        refreshToken: 'test-refresh-token',
        expiresAt: Date.now() + 3600000,
      });

      // @step And I have an OpenAI API key configured
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

      // @step And models.dev returns OpenAI provider with codex and non-codex models
      napiMocks.modelsListAll.mockResolvedValue([
        createOpenAIProviderWithCodexModels(),
      ]);

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then I should see an OpenAI section with non-codex models
      const openaiSection = result.sections.find(
        s => s.providerId === 'openai'
      );
      expect(openaiSection).toBeDefined();
      expect(openaiSection!.models.length).toBe(1);
      expect(openaiSection!.models[0].id).toBe('gpt-4o');

      // @step And I should see a separate Codex (ChatGPT) section with codex models
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeDefined();
      expect(codexSection!.models.length).toBe(3);
      expect(codexSection!.providerName).toBe('Codex (ChatGPT)');
    });

    it('should remove empty OpenAI section when all models are codex', async () => {
      // @step Given I have authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue({
        accessToken: 'test-access-token',
        refreshToken: 'test-refresh-token',
        expiresAt: Date.now() + 3600000,
      });

      // @step And I have an OpenAI API key configured
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

      // @step And models.dev returns OpenAI provider with ONLY codex models
      napiMocks.modelsListAll.mockResolvedValue([
        createOpenAIProviderAllCodexModels(),
      ]);

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then the OpenAI section should not appear (all models extracted)
      const openaiSection = result.sections.find(
        s => s.providerId === 'openai'
      );
      expect(openaiSection).toBeUndefined();

      // @step And the Codex section should contain all the models
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeDefined();
      expect(codexSection!.models.length).toBe(2);
    });
  });

  describe('Scenario: Selecting a Codex model creates session with codex provider', () => {
    it('should build model path with codex provider prefix', async () => {
      // @step Given I have authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue({
        accessToken: 'test-access-token',
        refreshToken: 'test-refresh-token',
        expiresAt: Date.now() + 3600000,
      });

      // @step And models.dev returns OpenAI provider with codex models
      napiMocks.modelsListAll.mockResolvedValue([
        createOpenAIProviderWithCodexModels(),
      ]);

      const result = await initializeModels();
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeDefined();

      // @step When I select a codex model from the Codex section
      const selectedModel = codexSection!.models[0];
      const modelPath = buildModelString(
        { providerId: codexSection!.providerId },
        selectedModel.id
      );

      // @step Then the model path should use codex as the provider prefix
      expect(modelPath).toBe('codex/gpt-5.3-codex');
      expect(modelPath.startsWith('codex/')).toBe(true);
      expect(modelPath.startsWith('openai/')).toBe(false);
    });
  });

  describe('Scenario: Persisted Codex model restored on startup', () => {
    it('should restore persisted codex model when OAuth tokens exist', async () => {
      // @step Given I have authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue({
        accessToken: 'test-access-token',
        refreshToken: 'test-refresh-token',
        expiresAt: Date.now() + 3600000,
      });

      // @step And my last used model was codex/gpt-5.3-codex
      const configContent = {
        tui: {
          lastUsedModel: 'codex/gpt-5.3-codex',
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'fspec-config.json'),
        JSON.stringify(configContent, null, 2)
      );

      // @step And models.dev returns OpenAI provider with codex models
      napiMocks.modelsListAll.mockResolvedValue([
        createOpenAIProviderWithCodexModels(),
      ]);

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then the persisted codex model should be restored as the current model
      expect(result.currentModel).not.toBeNull();
      expect(result.currentModel!.modelId).toBe('gpt-5.3-codex');
      expect(result.persistedModelRestored).toBe(true);

      // @step And the model providerId should be codex
      expect(result.currentModel!.providerId).toBe('codex');
    });
  });
});
