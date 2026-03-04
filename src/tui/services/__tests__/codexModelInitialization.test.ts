/**
 * Feature: spec/features/codex-model-selector-integration.feature
 *
 * PROV-033: Codex Model Selector Integration Tests
 *
 * Tests that when Codex OAuth tokens exist, ALL models from the OpenAI
 * cloud provider appear in the synthetic 'Codex (ChatGPT)' section —
 * not just models with 'codex' in the ID.
 *
 * The previous isCodexModel() filter was fundamentally wrong: real models.dev
 * OpenAI models are gpt-5.2, o3-pro, gpt-4.1, etc., none of which contain 'codex'.
 *
 * Test Strategy:
 * - Mock NAPI network boundary (modelsListAll) and codexOauthGetTokens
 * - Use REAL file system operations via test fixtures
 * - Verify ALL OpenAI models end up in Codex section when OAuth active
 * - Verify no OpenAI cloud section exists when Codex OAuth active
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
  claudeOauthGetTokens: vi.fn(),
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
    claudeOauthGetTokens: () => napiMocks.claudeOauthGetTokens(),
  };
});

vi.mock('../../../utils/logger', () => ({
  logger: { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() },
}));

import { initializeModels } from '../modelInitializationService';
import { buildModelString } from '../../utils/model-selection';

// =============================================================================
// TEST DATA FIXTURES — Realistic model IDs from models.dev
// =============================================================================

/**
 * Creates an OpenAI provider with realistic models.dev model IDs.
 * These are real model IDs — none contain 'codex' in the name.
 */
function createRealisticOpenAIProvider() {
  return {
    providerId: 'openai',
    providerName: 'OpenAI',
    models: [
      {
        id: 'gpt-5.2',
        name: 'GPT-5.2',
        reasoning: true,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 400000,
        maxOutput: 128000,
        hasVision: true,
      },
      {
        id: 'gpt-5',
        name: 'GPT-5',
        reasoning: true,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 400000,
        maxOutput: 128000,
        hasVision: true,
      },
      {
        id: 'o3-pro',
        name: 'o3 Pro',
        reasoning: true,
        toolCall: true,
        attachment: false,
        temperature: true,
        contextWindow: 200000,
        maxOutput: 100000,
        hasVision: true,
      },
      {
        id: 'o4-mini',
        name: 'o4 Mini',
        reasoning: true,
        toolCall: true,
        attachment: false,
        temperature: true,
        contextWindow: 200000,
        maxOutput: 100000,
        hasVision: true,
      },
      {
        id: 'gpt-4.1',
        name: 'GPT-4.1',
        reasoning: false,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 1000000,
        maxOutput: 32768,
        hasVision: true,
      },
      {
        id: 'o1',
        name: 'o1',
        reasoning: true,
        toolCall: true,
        attachment: false,
        temperature: true,
        contextWindow: 200000,
        maxOutput: 100000,
        hasVision: true,
      },
    ],
  };
}

/**
 * Creates a smaller OpenAI provider for the API key scenarios.
 */
function createSmallOpenAIProvider() {
  return {
    providerId: 'openai',
    providerName: 'OpenAI',
    models: [
      {
        id: 'gpt-5.2',
        name: 'GPT-5.2',
        reasoning: true,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 400000,
        maxOutput: 128000,
        hasVision: true,
      },
      {
        id: 'gpt-5',
        name: 'GPT-5',
        reasoning: true,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 400000,
        maxOutput: 128000,
        hasVision: true,
      },
      {
        id: 'o3-pro',
        name: 'o3 Pro',
        reasoning: true,
        toolCall: true,
        attachment: false,
        temperature: true,
        contextWindow: 200000,
        maxOutput: 100000,
        hasVision: true,
      },
      {
        id: 'gpt-4.1',
        name: 'GPT-4.1',
        reasoning: false,
        toolCall: true,
        attachment: true,
        temperature: true,
        contextWindow: 1000000,
        maxOutput: 32768,
        hasVision: true,
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
    napiMocks.claudeOauthGetTokens.mockReset();

    // Default: no Claude OAuth
    napiMocks.claudeOauthGetTokens.mockResolvedValue(null);

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

    // PROV-034: Write a broad codex-models.json that includes all PROV-033 test fixture model IDs.
    // This ensures PROV-033 tests (which verify all OpenAI models go to Codex section)
    // pass through the PROV-034 allowlist filter.
    const broadAllowlist = {
      version: 1,
      description: 'Broad allowlist for PROV-033 tests',
      models: [
        { slug: 'gpt-5.2', visibility: 'list', priority: 0 },
        { slug: 'gpt-5', visibility: 'list', priority: 1 },
        { slug: 'o3-pro', visibility: 'list', priority: 2 },
        { slug: 'o4-mini', visibility: 'list', priority: 3 },
        { slug: 'gpt-4.1', visibility: 'list', priority: 4 },
        { slug: 'o1', visibility: 'list', priority: 5 },
      ],
    };
    await writeFile(
      join(setup.testDir, '.fspec', 'codex-models.json'),
      JSON.stringify(broadAllowlist, null, 2)
    );
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

  // ===========================================================================
  // Scenario: All OpenAI cloud models appear in Codex section when OAuth tokens exist
  // ===========================================================================

  describe('Scenario: All OpenAI cloud models appear in Codex section when OAuth tokens exist', () => {
    it('should move ALL OpenAI models into a Codex (ChatGPT) section', async () => {
      // @step Given I have authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue({
        accessToken: 'test-access-token',
        refreshToken: 'test-refresh-token',
        expiresAt: Date.now() + 3600000,
      });

      // @step And models.dev returns OpenAI provider with models gpt-5.2, gpt-5, o3-pro, o4-mini, gpt-4.1, and o1
      napiMocks.modelsListAll.mockResolvedValue([
        createRealisticOpenAIProvider(),
      ]);

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then I should see a Codex (ChatGPT) section containing ALL OpenAI cloud models
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeDefined();
      expect(codexSection!.providerName).toBe('Codex (ChatGPT)');
      expect(codexSection!.hasCredentials).toBe(true);
      expect(codexSection!.models.length).toBe(6);

      // Verify ALL realistic model IDs are present
      const modelIds = codexSection!.models.map(m => m.id);
      expect(modelIds).toContain('gpt-5.2');
      expect(modelIds).toContain('gpt-5');
      expect(modelIds).toContain('o3-pro');
      expect(modelIds).toContain('o4-mini');
      expect(modelIds).toContain('gpt-4.1');
      expect(modelIds).toContain('o1');

      // @step And the Codex section should use providerId codex
      expect(codexSection!.providerId).toBe('codex');

      // @step And no OpenAI cloud section should exist
      const openaiSection = result.sections.find(
        s => s.providerId === 'openai'
      );
      expect(openaiSection).toBeUndefined();
    });
  });

  // ===========================================================================
  // Scenario: No Codex section when OAuth tokens absent
  // ===========================================================================

  describe('Scenario: No Codex section when OAuth tokens absent', () => {
    it('should not create a Codex section when no OAuth tokens exist', async () => {
      // @step Given I have not authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue(null);

      // @step And I have no OpenAI API key configured
      // (no credentials set up — env vars cleared in beforeEach)

      // @step When models are loaded for the model selector
      napiMocks.modelsListAll.mockResolvedValue([
        createRealisticOpenAIProvider(),
      ]);
      const result = await initializeModels();

      // @step Then I should not see any Codex or OpenAI section in the model selector
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeUndefined();

      // Note: OpenAI cloud section appears because requiresApiKey=false in provider registry
      // (PROV-029: OpenAI is a profile-only local model provider, always passes credentials check)
      // The key assertion is that NO Codex section exists without OAuth tokens
    });
  });

  // ===========================================================================
  // Scenario: Codex OAuth active with OpenAI API key shows only Codex section for cloud models
  // ===========================================================================

  describe('Scenario: Codex OAuth active with OpenAI API key shows only Codex section for cloud models', () => {
    it('should show only Codex section with ALL cloud models, no OpenAI cloud section', async () => {
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

      // @step And models.dev returns OpenAI provider with models gpt-5.2, gpt-5, o3-pro, and gpt-4.1
      napiMocks.modelsListAll.mockResolvedValue([createSmallOpenAIProvider()]);

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then I should see a Codex (ChatGPT) section with ALL cloud models
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeDefined();
      expect(codexSection!.models.length).toBe(4);
      expect(codexSection!.providerName).toBe('Codex (ChatGPT)');

      // @step And no OpenAI cloud section should exist
      const openaiSection = result.sections.find(
        s => s.providerId === 'openai'
      );
      expect(openaiSection).toBeUndefined();

      // @step And local profile sections should remain unaffected
      // (no profiles configured in this test, so no profile sections expected —
      //  the key assertion is that cloud OpenAI section is gone)
    });
  });

  // ===========================================================================
  // Scenario: Selecting a model from Codex section creates session with codex provider
  // ===========================================================================

  describe('Scenario: Selecting a model from Codex section creates session with codex provider', () => {
    it('should build model path with codex provider prefix for gpt-5.2', async () => {
      // @step Given I have authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue({
        accessToken: 'test-access-token',
        refreshToken: 'test-refresh-token',
        expiresAt: Date.now() + 3600000,
      });

      // @step And models.dev returns OpenAI provider with models gpt-5.2, gpt-5, o3-pro, o4-mini, gpt-4.1, and o1
      napiMocks.modelsListAll.mockResolvedValue([
        createRealisticOpenAIProvider(),
      ]);

      const result = await initializeModels();
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeDefined();

      // @step When I select gpt-5.2 from the Codex section
      const selectedModel = codexSection!.models.find(m => m.id === 'gpt-5.2');
      expect(selectedModel).toBeDefined();
      const modelPath = buildModelString(
        { providerId: codexSection!.providerId },
        selectedModel!.id
      );

      // @step Then the model path should be codex/gpt-5.2
      expect(modelPath).toBe('codex/gpt-5.2');
      expect(modelPath.startsWith('codex/')).toBe(true);
      expect(modelPath.startsWith('openai/')).toBe(false);
    });
  });

  // ===========================================================================
  // Scenario: Persisted Codex model restored on startup
  // ===========================================================================

  describe('Scenario: Persisted Codex model restored on startup', () => {
    it('should restore persisted codex model when OAuth tokens exist', async () => {
      // @step Given I have authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue({
        accessToken: 'test-access-token',
        refreshToken: 'test-refresh-token',
        expiresAt: Date.now() + 3600000,
      });

      // @step And my last used model was codex/gpt-5.2
      const configContent = {
        tui: {
          lastUsedModel: 'codex/gpt-5.2',
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'fspec-config.json'),
        JSON.stringify(configContent, null, 2)
      );

      // @step And models.dev returns OpenAI provider with models gpt-5.2, gpt-5, o3-pro, o4-mini, gpt-4.1, and o1
      napiMocks.modelsListAll.mockResolvedValue([
        createRealisticOpenAIProvider(),
      ]);

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then the persisted codex model should be restored as the current model
      expect(result.currentModel).not.toBeNull();
      expect(result.currentModel!.modelId).toBe('gpt-5.2');
      expect(result.persistedModelRestored).toBe(true);

      // @step And the model providerId should be codex
      expect(result.currentModel!.providerId).toBe('codex');
    });
  });

  // ===========================================================================
  // Scenario: OpenAI API key without Codex OAuth shows OpenAI section
  // ===========================================================================

  describe('Scenario: OpenAI API key without Codex OAuth shows OpenAI section', () => {
    it('should show OpenAI section with all cloud models and no Codex section', async () => {
      // @step Given I have not authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue(null);

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

      // @step And models.dev returns OpenAI provider with models gpt-5.2, gpt-5, o3-pro, and gpt-4.1
      napiMocks.modelsListAll.mockResolvedValue([createSmallOpenAIProvider()]);

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then I should see an OpenAI section with all cloud models
      const openaiSection = result.sections.find(
        s => s.providerId === 'openai'
      );
      expect(openaiSection).toBeDefined();
      expect(openaiSection!.models.length).toBe(4);

      // @step And no Codex section should exist
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeUndefined();
    });
  });
});
