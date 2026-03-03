/**
 * Feature: spec/features/claude-oauth-model-routing.feature
 *
 * PROV-026: Claude OAuth Model Routing Tests
 *
 * Tests that Claude models appear in the model selector when OAuth tokens
 * exist in claude_auth.json, setting hasCredentials=true for the Anthropic
 * section even without an ANTHROPIC_API_KEY.
 *
 * Also tests that persisted Anthropic models are restored correctly,
 * and that OAuth takes precedence in credential resolution.
 *
 * Test Strategy:
 * - Mock NAPI network boundary (modelsListAll) and claudeOauthGetTokens
 * - Use REAL file system operations via test fixtures
 * - Verify Anthropic section hasCredentials overridden by OAuth tokens
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
        maxOutput: 16000,
        hasVision: true,
      },
    ],
  };
}

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
    ],
  };
}

// =============================================================================
// TESTS
// =============================================================================

describe('Feature: Anthropic Provider Routing with Subscription Auth', () => {
  let setup: TestDirectorySetup;
  let originalHome: string | undefined;
  let originalCwd: string;
  let originalEnvVars: Record<string, string | undefined>;

  const credentialEnvVars = [
    'ANTHROPIC_API_KEY',
    'CLAUDE_CODE_OAUTH_TOKEN',
    'OPENAI_API_KEY',
    'CODEX_API_KEY',
    'GOOGLE_GENERATIVE_AI_API_KEY',
    'GEMINI_API_KEY',
  ];

  beforeEach(async () => {
    useModelStore.getState().reset();
    napiMocks.modelsListAll.mockReset();
    napiMocks.modelsListLocalOpenai.mockReset();
    napiMocks.codexOauthGetTokens.mockReset();
    napiMocks.claudeOauthGetTokens.mockReset();

    // Default: no Codex OAuth tokens
    napiMocks.codexOauthGetTokens.mockReturnValue(null);

    originalEnvVars = {};
    for (const envVar of credentialEnvVars) {
      originalEnvVars[envVar] = process.env[envVar];
      delete process.env[envVar];
    }

    setup = await setupTestDirectory('claude-oauth-model-routing');
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

  describe('Scenario: Anthropic models appear in model selector when OAuth tokens exist', () => {
    it('should show Anthropic section with hasCredentials=true when OAuth tokens exist and no API key', async () => {
      // @step Given I have authenticated with Claude via OAuth
      napiMocks.claudeOauthGetTokens.mockResolvedValue({
        accessToken: 'sk-ant-oat-test-access-token',
        refreshToken: 'sk-ant-ort-test-refresh-token',
        expires: Date.now() + 3600000,
      });

      // @step And claude_auth.json exists with valid access and refresh tokens
      // (implied by claudeOauthGetTokens returning tokens)

      // @step And models.dev returns the anthropic provider with Claude models
      napiMocks.modelsListAll.mockResolvedValue([createAnthropicProvider()]);

      // @step And I have no ANTHROPIC_API_KEY environment variable set
      // (env vars cleared in beforeEach)

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then I should see the Anthropic section with Claude models
      const anthropicSection = result.sections.find(
        s => s.providerId === 'anthropic'
      );
      expect(anthropicSection).toBeDefined();
      expect(anthropicSection!.models.length).toBe(2);

      // @step And the Anthropic section should have hasCredentials true
      expect(anthropicSection!.hasCredentials).toBe(true);
    });
  });

  describe('Scenario: No Anthropic section when no OAuth tokens and no API key', () => {
    it('should not show Anthropic section without any Claude credentials', async () => {
      // @step Given I have not authenticated with Claude via OAuth
      napiMocks.claudeOauthGetTokens.mockResolvedValue(null);

      // @step And I have no ANTHROPIC_API_KEY environment variable set
      // (env vars cleared in beforeEach)

      // @step When models are loaded for the model selector
      napiMocks.modelsListAll.mockResolvedValue([createAnthropicProvider()]);
      const result = await initializeModels();

      // @step Then I should not see the Anthropic section in the model selector
      const anthropicSection = result.sections.find(
        s => s.providerId === 'anthropic'
      );
      expect(anthropicSection).toBeUndefined();
    });
  });

  describe('Scenario: Both API key and OAuth tokens show Anthropic section', () => {
    it('should show Anthropic section when both API key and OAuth tokens exist', async () => {
      // @step Given I have authenticated with Claude via OAuth
      napiMocks.claudeOauthGetTokens.mockResolvedValue({
        accessToken: 'sk-ant-oat-test-access-token',
        refreshToken: 'sk-ant-ort-test-refresh-token',
        expires: Date.now() + 3600000,
      });

      // @step And I have an ANTHROPIC_API_KEY environment variable set
      const credentialsContent = {
        version: 1,
        providers: {
          anthropic: {
            apiKey: 'sk-ant-api03-test-key',
            lastUpdated: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'credentials', 'credentials.json'),
        JSON.stringify(credentialsContent, null, 2),
        { mode: 0o600 }
      );

      // @step And models.dev returns the anthropic provider with Claude models
      napiMocks.modelsListAll.mockResolvedValue([createAnthropicProvider()]);

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then I should see the Anthropic section with Claude models
      const anthropicSection = result.sections.find(
        s => s.providerId === 'anthropic'
      );
      expect(anthropicSection).toBeDefined();
      expect(anthropicSection!.models.length).toBe(2);

      // @step And the Anthropic section should have hasCredentials true
      expect(anthropicSection!.hasCredentials).toBe(true);
    });
  });

  describe('Scenario: API key only shows Anthropic section without OAuth', () => {
    it('should show Anthropic section with API key even without OAuth tokens', async () => {
      // @step Given I have not authenticated with Claude via OAuth
      napiMocks.claudeOauthGetTokens.mockResolvedValue(null);

      // @step And I have an ANTHROPIC_API_KEY environment variable set
      const credentialsContent = {
        version: 1,
        providers: {
          anthropic: {
            apiKey: 'sk-ant-api03-test-key',
            lastUpdated: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'credentials', 'credentials.json'),
        JSON.stringify(credentialsContent, null, 2),
        { mode: 0o600 }
      );

      // @step And models.dev returns the anthropic provider with Claude models
      napiMocks.modelsListAll.mockResolvedValue([createAnthropicProvider()]);

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then I should see the Anthropic section with Claude models
      const anthropicSection = result.sections.find(
        s => s.providerId === 'anthropic'
      );
      expect(anthropicSection).toBeDefined();
      expect(anthropicSection!.models.length).toBe(2);
      expect(anthropicSection!.hasCredentials).toBe(true);
    });
  });

  describe('Scenario: Persisted Anthropic model restored on startup with OAuth tokens', () => {
    it('should restore persisted anthropic model when OAuth tokens exist', async () => {
      // @step Given I have authenticated with Claude via OAuth
      napiMocks.claudeOauthGetTokens.mockResolvedValue({
        accessToken: 'sk-ant-oat-test-access-token',
        refreshToken: 'sk-ant-ort-test-refresh-token',
        expires: Date.now() + 3600000,
      });

      // @step And my last used model was anthropic/claude-sonnet-4-20250514
      const configContent = {
        tui: {
          lastUsedModel: 'anthropic/claude-sonnet-4-20250514',
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'fspec-config.json'),
        JSON.stringify(configContent, null, 2)
      );

      // @step And models.dev returns the anthropic provider with Claude models
      napiMocks.modelsListAll.mockResolvedValue([createAnthropicProvider()]);

      // @step When models are loaded for the model selector
      const result = await initializeModels();

      // @step Then the persisted model should be restored as the current model
      expect(result.currentModel).not.toBeNull();
      expect(result.persistedModelRestored).toBe(true);

      // @step And the model providerId should be anthropic
      expect(result.currentModel!.providerId).toBe('anthropic');
      expect(result.currentModel!.modelId).toBe('claude-sonnet-4');
    });
  });

  describe('Scenario: Non-OAuth providers unaffected by Claude OAuth changes', () => {
    it('should not show OpenAI section when no OpenAI credentials exist', async () => {
      // @step Given I have authenticated with Claude via OAuth
      napiMocks.claudeOauthGetTokens.mockResolvedValue({
        accessToken: 'sk-ant-oat-test-access-token',
        refreshToken: 'sk-ant-ort-test-refresh-token',
        expires: Date.now() + 3600000,
      });

      // @step And I have no OpenAI API key configured
      // (env vars cleared in beforeEach)

      // @step When models are loaded for the model selector
      napiMocks.modelsListAll.mockResolvedValue([
        createAnthropicProvider(),
        createOpenAIProviderWithCodexModels(),
      ]);
      const result = await initializeModels();

      // @step Then I should not see any OpenAI section
      const openaiSection = result.sections.find(
        s => s.providerId === 'openai'
      );
      expect(openaiSection).toBeUndefined();

      // @step And Codex OAuth behavior should be unchanged
      const codexSection = result.sections.find(s => s.providerId === 'codex');
      expect(codexSection).toBeUndefined(); // no Codex OAuth tokens

      // Anthropic section should be present from Claude OAuth
      const anthropicSection = result.sections.find(
        s => s.providerId === 'anthropic'
      );
      expect(anthropicSection).toBeDefined();
      expect(anthropicSection!.hasCredentials).toBe(true);
    });
  });
});
