/**
 * Feature: spec/features/model-selection-service.feature
 *
 * PROV-008: Model Selection Service and Profile Environment Tests
 *
 * Tests for model selection architecture refactoring:
 * 1. Deprecated handler removal validation
 * 2. Profile environment configuration
 * 3. Model selection service orchestration
 *
 * Test Strategy:
 * - Unit tests with mocked NAPI and config
 * - Build validation for deprecated code removal
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { readFileSync, existsSync } from 'fs';
import { glob } from 'tinyglobby';
import { join } from 'path';
import type { ProfileConfig } from '../../../utils/provider-config';
import type { ModelSelection } from '../../types/provider';

// =============================================================================
// MOCKS - Must be defined before imports
// =============================================================================

const napiMocks = vi.hoisted(() => ({
  sessionSetModel: vi.fn(),
  sessionSetModelProfile: vi.fn(),
}));

const configMocks = vi.hoisted(() => ({
  loadConfig: vi.fn(),
  writeConfig: vi.fn(),
}));

const envServiceMock = vi.hoisted(() => ({
  configureProfileEnvironment: vi.fn(),
}));

vi.mock('@sengac/codelet-napi', async importOriginal => {
  const original =
    await importOriginal<typeof import('@sengac/codelet-napi')>();
  return {
    ...original,
    sessionSetModel: napiMocks.sessionSetModel,
    sessionSetModelProfile: napiMocks.sessionSetModelProfile,
  };
});

vi.mock('../../../utils/config', () => ({
  loadConfig: configMocks.loadConfig,
  writeConfig: configMocks.writeConfig,
  getFspecUserDir: vi.fn().mockReturnValue('/tmp/.fspec'),
}));

vi.mock('../profileEnvironmentService', () => ({
  configureProfileEnvironment: envServiceMock.configureProfileEnvironment,
}));

vi.mock('../../../utils/logger', () => ({
  logger: { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() },
}));

// Import services AFTER mocks
import { selectModel } from '../modelSelectionService';

// =============================================================================
// TEST DATA FIXTURES
// =============================================================================

function createCloudModelSelection(): ModelSelection {
  return {
    providerId: 'anthropic',
    modelId: 'claude-sonnet-4',
    apiModelId: 'claude-sonnet-4-20250514',
    displayName: 'Claude Sonnet 4',
    reasoning: true,
    hasVision: true,
    contextWindow: 200000,
    maxOutput: 16000,
  };
}

function createProfileModelSelection(): ModelSelection {
  return {
    providerId: 'openai',
    modelId: 'Qwen3-80B',
    apiModelId: 'Qwen/Qwen3-80B',
    displayName: 'Qwen3 80B',
    reasoning: false,
    hasVision: false,
    contextWindow: 128000,
    maxOutput: 16384,
    profileName: 'work-vllm',
    profileConfig: {
      baseUrl: 'http://192.168.0.50:8888',
      apiKey: 'test-api-key',
      contextWindow: 128000,
      maxOutputTokens: 16384,
    },
  };
}

// =============================================================================
// SCENARIO: Delete deprecated handler without breaking build
// =============================================================================

describe('Feature: Model Selection Service', () => {
  describe('Scenario: Delete deprecated handler without breaking build', () => {
    it('should have no handleSelectModel callback in AgentView', async () => {
      // @step Given the AgentView component contains handleSelectModel callback
      const agentViewPath = join(
        process.cwd(),
        'src/tui/components/AgentView.tsx'
      );
      expect(existsSync(agentViewPath)).toBe(true);

      // @step When the deprecated handleSelectModel callback is removed
      const agentViewContent = readFileSync(agentViewPath, 'utf-8');

      // @step Then the TypeScript project compiles without errors
      const hasDeprecatedHandler =
        /const handleSelectModel\s*=\s*useCallback/.test(agentViewContent);
      expect(hasDeprecatedHandler).toBe(false);

      // @step And no code references handleSelectModel
      const srcFiles = await glob(['src/**/*.ts', 'src/**/*.tsx'], {
        cwd: process.cwd(),
        ignore: ['**/*.test.ts', '**/*.test.tsx', '**/node_modules/**'],
      });

      const references: string[] = [];
      for (const file of srcFiles) {
        const content = readFileSync(join(process.cwd(), file), 'utf-8');
        const matches = content.match(/handleSelectModel/g);
        if (matches && matches.length > 0) {
          references.push(`${file}: ${matches.length} references`);
        }
      }
      expect(references).toEqual([]);
    });
  });

  // ===========================================================================
  // SCENARIO: Configure environment variables for profile-based model
  // ===========================================================================

  describe('Scenario: Configure environment variables for profile-based model', () => {
    const originalEnv = {
      OPENAI_BASE_URL: process.env.OPENAI_BASE_URL,
      OPENAI_API_KEY: process.env.OPENAI_API_KEY,
      OPENAI_CONTEXT_WINDOW: process.env.OPENAI_CONTEXT_WINDOW,
      OPENAI_MAX_OUTPUT_TOKENS: process.env.OPENAI_MAX_OUTPUT_TOKENS,
    };

    beforeEach(() => {
      delete process.env.OPENAI_BASE_URL;
      delete process.env.OPENAI_API_KEY;
      delete process.env.OPENAI_CONTEXT_WINDOW;
      delete process.env.OPENAI_MAX_OUTPUT_TOKENS;
    });

    afterEach(() => {
      // Restore original environment
      Object.entries(originalEnv).forEach(([key, value]) => {
        if (value !== undefined) {
          process.env[key] = value;
        } else {
          delete process.env[key];
        }
      });
    });

    it('should set OPENAI_BASE_URL and OPENAI_API_KEY from profile config', async () => {
      // Import directly from the source module, bypassing the mock
      // This test validates the REAL implementation
      const profileEnvModule = await vi.importActual<
        typeof import('../profileEnvironmentService')
      >('../profileEnvironmentService');
      const realConfigureEnv = profileEnvModule.configureProfileEnvironment;

      // @step Given a profile config with baseUrl "http://192.168.0.50:8888" and apiKey "test-api-key"
      const profileConfig: ProfileConfig = {
        baseUrl: 'http://192.168.0.50:8888',
        apiKey: 'test-api-key',
      };

      // @step When configureProfileEnvironment is called with the profile config
      realConfigureEnv(profileConfig);

      // @step Then OPENAI_BASE_URL should be set to "http://192.168.0.50:8888"
      expect(process.env.OPENAI_BASE_URL).toBe('http://192.168.0.50:8888');

      // @step And OPENAI_API_KEY should be set to "test-api-key"
      expect(process.env.OPENAI_API_KEY).toBe('test-api-key');
    });
  });

  // ===========================================================================
  // SCENARIO: Persist model selection to config file
  // ===========================================================================

  describe('Scenario: Persist model selection to config file', () => {
    beforeEach(() => {
      vi.clearAllMocks();
      napiMocks.sessionSetModel.mockResolvedValue(undefined);
      napiMocks.sessionSetModelProfile.mockResolvedValue(undefined);
      configMocks.loadConfig.mockResolvedValue({});
      configMocks.writeConfig.mockResolvedValue(undefined);
    });

    it('should call writeConfig with lastUsedModel', async () => {
      // @step Given a model selection for provider "anthropic" model "claude-sonnet-4"
      const selection = createCloudModelSelection();

      // @step When selectModel completes successfully
      const result = await selectModel({
        sessionId: null,
        selection,
      });

      // @step Then the result should indicate success
      expect(result.success).toBe(true);

      // @step And writeConfig should be called with lastUsedModel "anthropic/claude-sonnet-4"
      expect(configMocks.writeConfig).toHaveBeenCalledWith('user', {
        tui: {
          lastUsedModel: 'anthropic/claude-sonnet-4',
        },
      });
    });
  });

  // ===========================================================================
  // SCENARIO: Select cloud provider model with active session
  // ===========================================================================

  describe('Scenario: Select cloud provider model with active session', () => {
    beforeEach(() => {
      vi.clearAllMocks();
      napiMocks.sessionSetModel.mockResolvedValue(undefined);
      napiMocks.sessionSetModelProfile.mockResolvedValue(undefined);
      configMocks.loadConfig.mockResolvedValue({});
      configMocks.writeConfig.mockResolvedValue(undefined);
    });

    it('should call sessionSetModel and persist to config', async () => {
      // @step Given an active session with id "session-123"
      const sessionId = 'session-123';
      const onRefreshRustState = vi.fn();
      const onSetCurrentModel = vi.fn();
      const onSetCurrentProvider = vi.fn();

      // @step And a cloud model selection for provider "anthropic" model "claude-sonnet-4"
      const selection = createCloudModelSelection();

      // @step When selectModel is called with the session and selection
      const result = await selectModel({
        sessionId,
        selection,
        onRefreshRustState,
        onSetCurrentModel,
        onSetCurrentProvider,
      });

      // @step Then the result should indicate success
      expect(result.success).toBe(true);

      // @step And sessionSetModel should be called with the provider and model
      expect(napiMocks.sessionSetModel).toHaveBeenCalledWith(
        'session-123',
        'anthropic',
        'claude-sonnet-4',
        200000,
        16000,
        null,
        null
      );

      // @step And the model store should be updated
      expect(onRefreshRustState).toHaveBeenCalledWith('session-123');

      // @step And the Zustand store should be updated (BUG-097: always update on success)
      expect(onSetCurrentModel).toHaveBeenCalledWith(selection);
      expect(onSetCurrentProvider).toHaveBeenCalledWith('claude');

      // @step And the selection should be persisted to config
      expect(configMocks.writeConfig).toHaveBeenCalled();
    });
  });

  // ===========================================================================
  // SCENARIO: Select profile-based model with active session
  // ===========================================================================

  describe('Scenario: Select profile-based model with active session', () => {
    beforeEach(() => {
      vi.clearAllMocks();
      napiMocks.sessionSetModel.mockResolvedValue(undefined);
      napiMocks.sessionSetModelProfile.mockResolvedValue(undefined);
      configMocks.loadConfig.mockResolvedValue({});
      configMocks.writeConfig.mockResolvedValue(undefined);
    });

    it('should configure environment and call sessionSetModelProfile', async () => {
      // @step Given an active session with id "session-123"
      const sessionId = 'session-123';
      const onRefreshRustState = vi.fn();
      const onSetCurrentModel = vi.fn();
      const onSetCurrentProvider = vi.fn();

      // @step And a model selection with profileConfig containing baseUrl and apiKey
      const selection = createProfileModelSelection();

      // @step When selectModel is called with the session and selection
      const result = await selectModel({
        sessionId,
        selection,
        onRefreshRustState,
        onSetCurrentModel,
        onSetCurrentProvider,
      });

      // @step Then the result should indicate success
      expect(result.success).toBe(true);

      // @step And configureProfileEnvironment should be called with the profileConfig
      expect(envServiceMock.configureProfileEnvironment).toHaveBeenCalledWith(
        selection.profileConfig
      );

      // @step And sessionSetModelProfile should be called instead of sessionSetModel
      expect(napiMocks.sessionSetModelProfile).toHaveBeenCalledWith(
        'session-123',
        'openai',
        'Qwen3-80B',
        128000,
        16384,
        null,
        null,
        null
      );
      expect(napiMocks.sessionSetModel).not.toHaveBeenCalled();

      // @step And the Zustand store should be updated (BUG-097: always update on success)
      expect(onSetCurrentModel).toHaveBeenCalledWith(selection);
      expect(onSetCurrentProvider).toHaveBeenCalledWith('openai');
    });
  });

  // ===========================================================================
  // SCENARIO: Select model without active session
  // ===========================================================================

  describe('Scenario: Select model without active session', () => {
    beforeEach(() => {
      vi.clearAllMocks();
      napiMocks.sessionSetModel.mockResolvedValue(undefined);
      napiMocks.sessionSetModelProfile.mockResolvedValue(undefined);
      configMocks.loadConfig.mockResolvedValue({});
      configMocks.writeConfig.mockResolvedValue(undefined);
    });

    it('should update store and persist config without calling NAPI session methods', async () => {
      // @step Given no active session exists
      const sessionId = null;
      const onSetCurrentModel = vi.fn();
      const onSetCurrentProvider = vi.fn();

      // @step And a model selection for provider "openai" model "gpt-4o"
      const selection: ModelSelection = {
        providerId: 'openai',
        modelId: 'gpt-4o',
        apiModelId: 'gpt-4o',
        displayName: 'GPT-4o',
        reasoning: false,
        hasVision: true,
        contextWindow: 128000,
        maxOutput: 16384,
      };

      // @step When selectModel is called with null session and the selection
      const result = await selectModel({
        sessionId,
        selection,
        onSetCurrentModel,
        onSetCurrentProvider,
      });

      // @step Then the result should indicate success
      expect(result.success).toBe(true);

      // @step And neither sessionSetModel nor sessionSetModelProfile should be called
      expect(napiMocks.sessionSetModel).not.toHaveBeenCalled();
      expect(napiMocks.sessionSetModelProfile).not.toHaveBeenCalled();

      // @step And the model store should be updated for later session sync
      expect(onSetCurrentModel).toHaveBeenCalledWith(selection);
      expect(onSetCurrentProvider).toHaveBeenCalledWith('openai');

      // @step And the selection should be persisted to config
      expect(configMocks.writeConfig).toHaveBeenCalledWith('user', {
        tui: {
          lastUsedModel: 'openai/gpt-4o',
        },
      });
    });
  });

  // ===========================================================================
  // SCENARIO: Cloud model selection failure handling
  // ===========================================================================

  describe('Scenario: Cloud model selection fails', () => {
    beforeEach(() => {
      vi.clearAllMocks();
      configMocks.loadConfig.mockResolvedValue({});
      configMocks.writeConfig.mockResolvedValue(undefined);
    });

    it('should not persist when sessionSetModel fails', async () => {
      // @step Given an active session
      const sessionId = 'session-123';
      const onSetCurrentModel = vi.fn();
      const onSetCurrentProvider = vi.fn();

      // @step And sessionSetModel will fail
      napiMocks.sessionSetModel.mockRejectedValue(
        new Error('Failed to select model')
      );

      // @step When selectModel is called with a cloud model
      const selection = createCloudModelSelection();
      const result = await selectModel({
        sessionId,
        selection,
        onSetCurrentModel,
        onSetCurrentProvider,
      });

      // @step Then the result should indicate failure
      expect(result.success).toBe(false);
      expect(result.error).toBe('Failed to select model');

      // @step And writeConfig should NOT be called
      expect(configMocks.writeConfig).not.toHaveBeenCalled();

      // @step And the Zustand store should NOT be updated (BUG-097: only update on success)
      expect(onSetCurrentModel).not.toHaveBeenCalled();
      expect(onSetCurrentProvider).not.toHaveBeenCalled();
    });
  });

  // ===========================================================================
  // MODEL-005: modelSelectionService passes contextWindow and maxOutput to NAPI
  // Feature: spec/features/per-model-context-window-and-max-output-configuration.feature
  // ===========================================================================

  describe('Scenario: modelSelectionService passes contextWindow and maxOutput to sessionSetModel', () => {
    beforeEach(() => {
      vi.clearAllMocks();
      napiMocks.sessionSetModel.mockResolvedValue(undefined);
      napiMocks.sessionSetModelProfile.mockResolvedValue(undefined);
      configMocks.loadConfig.mockResolvedValue({});
      configMocks.writeConfig.mockResolvedValue(undefined);
    });

    it('should pass contextWindow and maxOutput to sessionSetModel', async () => {
      // @step Given a ModelSelection with providerId="openai" and modelId="o3" and contextWindow=200000 and maxOutput=100000
      const selection: ModelSelection = {
        providerId: 'openai',
        modelId: 'o3',
        apiModelId: 'o3',
        displayName: 'o3',
        reasoning: true,
        hasVision: false,
        contextWindow: 200000,
        maxOutput: 100000,
      };

      // @step And an active session exists
      const sessionId = 'session-456';

      // @step When selectModel is called
      const result = await selectModel({
        sessionId,
        selection,
      });

      expect(result.success).toBe(true);

      // @step Then sessionSetModel is called with context_window=200000 and max_output_tokens=100000
      expect(napiMocks.sessionSetModel).toHaveBeenCalledWith(
        'session-456',
        'openai',
        'o3',
        200000,
        100000,
        null,
        null
      );
    });
  });

  describe('Scenario: modelSelectionService passes contextWindow and maxOutput to sessionSetModelProfile', () => {
    beforeEach(() => {
      vi.clearAllMocks();
      napiMocks.sessionSetModel.mockResolvedValue(undefined);
      napiMocks.sessionSetModelProfile.mockResolvedValue(undefined);
      configMocks.loadConfig.mockResolvedValue({});
      configMocks.writeConfig.mockResolvedValue(undefined);
    });

    it('should pass contextWindow and maxOutput to sessionSetModelProfile', async () => {
      // @step Given a ModelSelection with profileConfig and contextWindow=32000 and maxOutput=4096
      const selection: ModelSelection = {
        providerId: 'openai',
        modelId: 'local-model',
        apiModelId: 'local-model',
        displayName: 'Local Model',
        reasoning: false,
        hasVision: false,
        contextWindow: 32000,
        maxOutput: 4096,
        profileName: 'local-vllm',
        profileConfig: {
          baseUrl: 'http://localhost:8080',
          apiKey: 'test-key',
          contextWindow: 32000,
          maxOutputTokens: 4096,
        },
      };

      // @step And an active session exists
      const sessionId = 'session-789';

      // @step When selectModel is called
      const result = await selectModel({
        sessionId,
        selection,
      });

      expect(result.success).toBe(true);

      // @step Then sessionSetModelProfile is called with context_window=32000 and max_output_tokens=4096
      expect(napiMocks.sessionSetModelProfile).toHaveBeenCalledWith(
        'session-789',
        'openai',
        'local-model',
        32000,
        4096,
        null,
        null,
        null
      );
    });
  });
});
