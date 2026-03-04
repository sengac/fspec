/**
 * Feature: spec/features/codex-subscription-model-filtering.feature
 *
 * PROV-034: Codex Subscription Model Filtering Tests
 *
 * Tests that when Codex OAuth is active, the model selector filters models.dev
 * catalog against a Codex-supported allowlist loaded from a config file.
 * Only models whose slugs match (or prefix-match) the allowlist appear in the
 * Codex (ChatGPT) section. Models not in the Codex catalog are hidden.
 *
 * Test Strategy:
 * - Mock NAPI network boundary (modelsListAll, codexOauthGetTokens)
 * - Write codex-models.json to test HOME directory
 * - Verify allowlist filtering, prefix matching, and config-driven behavior
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdir, writeFile, readFile } from 'fs/promises';
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

// =============================================================================
// CODEX ALLOWLIST CONFIG
// =============================================================================

/**
 * The 12 Codex-supported model slugs from the openai/codex repository.
 * This is the content that goes into codex-models.json.
 */
function createCodexAllowlistConfig() {
  return {
    version: 1,
    description:
      'Codex-supported model slugs from openai/codex repository (codex-rs/core/models.json)',
    models: [
      { slug: 'gpt-5.3-codex', visibility: 'list', priority: 0 },
      { slug: 'gpt-5.2-codex', visibility: 'list', priority: 3 },
      { slug: 'gpt-5.1-codex-max', visibility: 'list', priority: 4 },
      { slug: 'gpt-5.1-codex', visibility: 'hide', priority: 5 },
      { slug: 'gpt-5.2', visibility: 'list', priority: 6 },
      { slug: 'gpt-5.1', visibility: 'hide', priority: 7 },
      { slug: 'gpt-5-codex', visibility: 'hide', priority: 10 },
      { slug: 'gpt-5', visibility: 'hide', priority: 11 },
      { slug: 'gpt-oss-120b', visibility: 'hide', priority: 11 },
      { slug: 'gpt-oss-20b', visibility: 'hide', priority: 11 },
      { slug: 'gpt-5.1-codex-mini', visibility: 'list', priority: 12 },
      { slug: 'gpt-5-codex-mini', visibility: 'hide', priority: 13 },
    ],
  };
}

// =============================================================================
// TEST DATA FIXTURES — Full models.dev catalog including unsupported models
// =============================================================================

/**
 * Creates a large OpenAI provider simulating models.dev with 19 models,
 * including many NOT available in the Codex subscription.
 */
function createFullModelsDevOpenAIProvider() {
  return {
    providerId: 'openai',
    providerName: 'OpenAI',
    models: [
      // Codex-supported models (should pass filter)
      createModel('gpt-5.2-codex', 'GPT-5.2 Codex'),
      createModel('gpt-5.2', 'GPT-5.2'),
      createModel('gpt-5.1-codex-max', 'GPT-5.1 Codex Max'),
      createModel('gpt-5.3-codex', 'GPT-5.3 Codex'),
      createModel('gpt-5.1-codex-mini', 'GPT-5.1 Codex Mini'),
      createModel('gpt-5', 'GPT-5'),
      // NOT in Codex catalog (should be filtered out)
      createModel('o3-pro', 'o3 Pro'),
      createModel('o4-mini', 'o4 Mini'),
      createModel('gpt-4.1', 'GPT-4.1'),
      createModel('gpt-4.1-mini', 'GPT-4.1 Mini'),
      createModel('gpt-4.1-nano', 'GPT-4.1 Nano'),
      createModel('o1', 'o1'),
      createModel('o1-pro', 'o1 Pro'),
      createModel('o3-mini', 'o3 Mini'),
      createModel('gpt-5-mini', 'GPT-5 Mini'),
      createModel('gpt-5-nano', 'GPT-5 Nano'),
      createModel('gpt-5-pro', 'GPT-5 Pro'),
      createModel('gpt-5.2-pro', 'GPT-5.2 Pro'),
      createModel('gpt-4o-2024-11-20', 'GPT-4o'),
    ],
  };
}

function createModel(id: string, name: string) {
  return {
    id,
    name,
    reasoning: id.startsWith('o'),
    toolCall: true,
    attachment: false,
    temperature: true,
    contextWindow: 200000,
    maxOutput: 100000,
    hasVision: true,
  };
}

// =============================================================================
// TESTS
// =============================================================================

describe('Feature: Codex Subscription Model Filtering', () => {
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
    napiMocks.claudeOauthGetTokens.mockResolvedValue(null);

    originalEnvVars = {};
    for (const envVar of credentialEnvVars) {
      originalEnvVars[envVar] = process.env[envVar];
      delete process.env[envVar];
    }

    setup = await setupTestDirectory('codex-subscription-filter');
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

  // ===========================================================================
  // Scenario: Codex OAuth active filters models to only Codex-supported catalog entries
  // ===========================================================================

  describe('Scenario: Codex OAuth active filters models to only Codex-supported catalog entries', () => {
    it('should filter models.dev catalog to only Codex-supported models', async () => {
      // @step Given I have authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue({
        accessToken: 'test-access-token',
        refreshToken: 'test-refresh-token',
        expiresAt: Date.now() + 3600000,
      });

      // @step And models.dev returns 19 OpenAI models including o3-pro, gpt-4.1, gpt-5-mini, gpt-5.2-codex, gpt-5.2, and gpt-5.1-codex-max
      napiMocks.modelsListAll.mockResolvedValue([
        createFullModelsDevOpenAIProvider(),
      ]);

      // @step And the Codex allowlist contains slugs gpt-5.3-codex, gpt-5.2-codex, gpt-5.1-codex-max, gpt-5.1-codex, gpt-5.2, gpt-5.1, gpt-5-codex, gpt-5, gpt-oss-120b, gpt-oss-20b, gpt-5.1-codex-mini, gpt-5-codex-mini
      await writeFile(
        join(setup.testDir, '.fspec', 'codex-models.json'),
        JSON.stringify(createCodexAllowlistConfig(), null, 2)
      );

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then the Codex (ChatGPT) section should only contain picker-visible models matching the allowlist
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeDefined();
      const modelIds = codexSection!.models.map(m => m.id);

      // These 5 picker-visible models from the fixture match the allowlist
      // (gpt-5 has visibility='hide' so it does NOT appear)
      expect(modelIds).toContain('gpt-5.2-codex');
      expect(modelIds).toContain('gpt-5.2');
      expect(modelIds).toContain('gpt-5.1-codex-max');
      expect(modelIds).toContain('gpt-5.3-codex');
      expect(modelIds).toContain('gpt-5.1-codex-mini');

      // @step And models with visibility hide in the allowlist should not appear in the selector
      // gpt-5 has visibility='hide' — usable but not shown in picker
      expect(modelIds).not.toContain('gpt-5');

      // @step And models like o3-pro, gpt-4.1, gpt-5-mini should not appear
      expect(modelIds).not.toContain('o3-pro');
      expect(modelIds).not.toContain('gpt-4.1');
      expect(modelIds).not.toContain('gpt-5-mini');

      // Also verify other unsupported models are hidden
      expect(modelIds).not.toContain('o4-mini');
      expect(modelIds).not.toContain('gpt-4.1-mini');
      expect(modelIds).not.toContain('gpt-4.1-nano');
      expect(modelIds).not.toContain('o1');
      expect(modelIds).not.toContain('o1-pro');
      expect(modelIds).not.toContain('o3-mini');
      expect(modelIds).not.toContain('gpt-5-nano');
      expect(modelIds).not.toContain('gpt-5-pro');
      expect(modelIds).not.toContain('gpt-5.2-pro');
      expect(modelIds).not.toContain('gpt-4o-2024-11-20');

      // Total: only 5 out of 19 should pass (only visibility='list' models)
      expect(codexSection!.models.length).toBe(5);

      // @step And models should be sorted by allowlist priority
      // Verify priority ordering: gpt-5.3-codex (0) first, gpt-5.1-codex-mini (12) last
      expect(codexSection!.models[0].id).toBe('gpt-5.3-codex');
      expect(codexSection!.models[codexSection!.models.length - 1].id).toBe(
        'gpt-5.1-codex-mini'
      );
    });
  });

  // ===========================================================================
  // Scenario: No Codex OAuth shows full unfiltered models.dev catalog
  // ===========================================================================

  describe('Scenario: No Codex OAuth shows full unfiltered models.dev catalog', () => {
    it('should show all models unfiltered when no Codex OAuth exists', async () => {
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

      // @step And models.dev returns 19 OpenAI models including o3-pro, gpt-4.1, gpt-5-mini, and gpt-5.2
      napiMocks.modelsListAll.mockResolvedValue([
        createFullModelsDevOpenAIProvider(),
      ]);

      // Also write the allowlist — it should NOT be applied without OAuth
      await writeFile(
        join(setup.testDir, '.fspec', 'codex-models.json'),
        JSON.stringify(createCodexAllowlistConfig(), null, 2)
      );

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then I should see an OpenAI section with all 19 models unfiltered
      const openaiSection = result.sections.find(
        s => s.providerId === 'openai'
      );
      expect(openaiSection).toBeDefined();
      expect(openaiSection!.models.length).toBe(19);

      // @step And no Codex allowlist filtering should be applied
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeUndefined();
    });
  });

  // ===========================================================================
  // Scenario: Allowlist is loaded from external config file not hardcoded in source
  // ===========================================================================

  describe('Scenario: Allowlist is loaded from external config file not hardcoded in source', () => {
    it('should read the allowlist from codex-models.json config file', async () => {
      // @step Given I have authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue({
        accessToken: 'test-access-token',
        refreshToken: 'test-refresh-token',
        expiresAt: Date.now() + 3600000,
      });

      // @step And a codex-models.json config file exists with the Codex-supported model slugs
      // Write a CUSTOM allowlist with only 2 models to prove it's config-driven
      const customAllowlist = {
        version: 1,
        description: 'Custom test allowlist',
        models: [
          { slug: 'gpt-5.2', visibility: 'list', priority: 0 },
          { slug: 'gpt-5', visibility: 'list', priority: 1 },
        ],
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'codex-models.json'),
        JSON.stringify(customAllowlist, null, 2)
      );

      napiMocks.modelsListAll.mockResolvedValue([
        createFullModelsDevOpenAIProvider(),
      ]);

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then the allowlist should be read from the config file
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeDefined();

      // @step And the filtering behavior should match the config file contents
      // Only gpt-5.2 and gpt-5 from the custom allowlist should appear
      const modelIds = codexSection!.models.map(m => m.id);
      expect(modelIds).toContain('gpt-5.2');
      expect(modelIds).toContain('gpt-5');
      expect(codexSection!.models.length).toBe(2);

      // Models NOT in the custom allowlist should be filtered out
      expect(modelIds).not.toContain('gpt-5.2-codex');
      expect(modelIds).not.toContain('o3-pro');
    });
  });

  // ===========================================================================
  // Scenario: Adding a new model to the allowlist config makes it appear without code changes
  // ===========================================================================

  describe('Scenario: Adding a new model to the allowlist config makes it appear without code changes', () => {
    it('should pick up new models when config is updated between loads', async () => {
      // @step Given I have authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue({
        accessToken: 'test-access-token',
        refreshToken: 'test-refresh-token',
        expiresAt: Date.now() + 3600000,
      });

      // @step And the Codex allowlist config does not contain gpt-6-codex
      const initialAllowlist = {
        version: 1,
        description: 'Initial allowlist without gpt-6-codex',
        models: [{ slug: 'gpt-5.2', visibility: 'list', priority: 0 }],
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'codex-models.json'),
        JSON.stringify(initialAllowlist, null, 2)
      );

      // @step And models.dev returns a model with slug gpt-6-codex
      const providerWithGpt6 = {
        providerId: 'openai',
        providerName: 'OpenAI',
        models: [
          createModel('gpt-5.2', 'GPT-5.2'),
          createModel('gpt-6-codex', 'GPT-6 Codex'),
        ],
      };
      napiMocks.modelsListAll.mockResolvedValue([providerWithGpt6]);

      // @step When models are loaded for the model selector
      const result1 = await initializeModels();

      // @step Then gpt-6-codex should not appear in the Codex section
      const codexSection1 = result1.sections.find(
        s => s.providerId === 'codex'
      );
      expect(codexSection1).toBeDefined();
      const modelIds1 = codexSection1!.models.map(m => m.id);
      expect(modelIds1).not.toContain('gpt-6-codex');
      expect(modelIds1).toContain('gpt-5.2');

      // @step When the allowlist config is updated to include gpt-6-codex
      const updatedAllowlist = {
        version: 1,
        description: 'Updated allowlist with gpt-6-codex',
        models: [
          { slug: 'gpt-5.2', visibility: 'list', priority: 0 },
          { slug: 'gpt-6-codex', visibility: 'list', priority: 1 },
        ],
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'codex-models.json'),
        JSON.stringify(updatedAllowlist, null, 2)
      );

      // @step And models are reloaded for the model selector
      // Reset store to force re-initialization
      useModelStore.getState().reset();
      const result2 = await initializeModels();

      // @step Then gpt-6-codex should appear in the Codex section
      const codexSection2 = result2.sections.find(
        s => s.providerId === 'codex'
      );
      expect(codexSection2).toBeDefined();
      const modelIds2 = codexSection2!.models.map(m => m.id);
      expect(modelIds2).toContain('gpt-6-codex');
      expect(modelIds2).toContain('gpt-5.2');
    });
  });

  // ===========================================================================
  // Scenario: Slug prefix matching filters dated model variants correctly
  // ===========================================================================

  describe('Scenario: Slug prefix matching filters dated model variants correctly', () => {
    it('should match dated model variants via slug prefix', async () => {
      // @step Given I have authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue({
        accessToken: 'test-access-token',
        refreshToken: 'test-refresh-token',
        expiresAt: Date.now() + 3600000,
      });

      // @step And the Codex allowlist contains slug gpt-5.2-codex
      const allowlist = {
        version: 1,
        description: 'Allowlist for prefix matching test',
        models: [{ slug: 'gpt-5.2-codex', visibility: 'list', priority: 0 }],
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'codex-models.json'),
        JSON.stringify(allowlist, null, 2)
      );

      // @step And models.dev returns a model with slug gpt-5.2-codex-2026-03-01
      const providerWithDated = {
        providerId: 'openai',
        providerName: 'OpenAI',
        models: [
          createModel('gpt-5.2-codex-2026-03-01', 'GPT-5.2 Codex (2026-03-01)'),
          createModel('o3-pro-2026-03-01', 'o3 Pro (2026-03-01)'),
        ],
      };
      napiMocks.modelsListAll.mockResolvedValue([providerWithDated]);

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then gpt-5.2-codex-2026-03-01 should appear in the Codex section because it prefix-matches gpt-5.2-codex
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeDefined();
      const modelIds = codexSection!.models.map(m => m.id);
      expect(modelIds).toContain('gpt-5.2-codex-2026-03-01');

      // o3-pro-2026-03-01 should NOT match because o3-pro is not in the allowlist
      expect(modelIds).not.toContain('o3-pro-2026-03-01');
      expect(codexSection!.models.length).toBe(1);
    });
  });

  // ===========================================================================
  // Scenario: Local model profiles are never filtered by the Codex allowlist
  // ===========================================================================

  describe('Scenario: Local model profiles are never filtered by the Codex allowlist', () => {
    it('should not filter local profile models even when Codex allowlist is active', async () => {
      // @step Given I have authenticated with Codex via OAuth
      napiMocks.codexOauthGetTokens.mockReturnValue({
        accessToken: 'test-access-token',
        refreshToken: 'test-refresh-token',
        expiresAt: Date.now() + 3600000,
      });

      // @step And I have a local OpenAI profile named work-vllm with models Qwen3-80B and Llama-4-Scout
      const configContent = {
        providers: {
          openai: {
            profiles: {
              'work-vllm': {
                baseUrl: 'http://localhost:8888',
                apiKey: 'test-key',
                contextWindow: 128000,
                maxOutputTokens: 16384,
              },
            },
          },
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'fspec-config.json'),
        JSON.stringify(configContent, null, 2)
      );

      // Mock local model fetch to return the profile models
      napiMocks.modelsListLocalOpenai.mockResolvedValue([
        'Qwen3-80B',
        'Llama-4-Scout',
      ]);

      // @step And the Codex allowlist does not contain Qwen3-80B or Llama-4-Scout
      const allowlist = {
        version: 1,
        description: 'Allowlist that does not include local models',
        models: [{ slug: 'gpt-5.2-codex', visibility: 'list', priority: 0 }],
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'codex-models.json'),
        JSON.stringify(allowlist, null, 2)
      );

      // Provide a cloud model too
      const cloudProvider = {
        providerId: 'openai',
        providerName: 'OpenAI',
        models: [
          createModel('gpt-5.2-codex', 'GPT-5.2 Codex'),
          createModel('o3-pro', 'o3 Pro'),
        ],
      };
      napiMocks.modelsListAll.mockResolvedValue([cloudProvider]);

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then the local profile section openai: work-vllm should still display both models
      const profileSection = result.sections.find(
        s => s.profileName === 'work-vllm'
      );
      expect(profileSection).toBeDefined();
      const profileModelIds = profileSection!.models.map(m => m.id);
      expect(profileModelIds).toContain('Qwen3-80B');
      expect(profileModelIds).toContain('Llama-4-Scout');
      expect(profileSection!.models.length).toBe(2);

      // @step And the Codex allowlist filtering should only apply to cloud models from models.dev
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeDefined();
      const codexModelIds = codexSection!.models.map(m => m.id);
      expect(codexModelIds).toContain('gpt-5.2-codex');
      expect(codexModelIds).not.toContain('o3-pro');
      expect(codexSection!.models.length).toBe(1);
    });
  });
});
