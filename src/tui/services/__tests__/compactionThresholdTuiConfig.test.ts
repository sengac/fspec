/**
 * Feature: spec/features/compaction-threshold-tui-config.feature
 *
 * CTX-008: Tests for TUI Configuration Fields and NAPI Bridge for Compaction Threshold.
 *
 * Validates:
 * - parseCompactionThreshold input parsing logic
 * - Form field constants include compactionThreshold
 * - TypeScript types have compactionThreshold field
 * - modelSelectionService passes compactionThreshold to NAPI
 * - NAPI type declarations include compaction threshold params
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { readFileSync } from 'fs';
import { join } from 'path';
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

// Import AFTER mocks
import { selectModel } from '../modelSelectionService';
import { parseCompactionThreshold } from '../../utils/compactionThresholdParser';
import { PROFILE_FORM_FIELDS } from '../../constants/providerSettings';
import { CUSTOM_MODEL_FORM_FIELDS } from '../../constants/customModelForm';

// =============================================================================
// Input Parsing Tests
// =============================================================================

describe('Feature: TUI Configuration Fields and NAPI Bridge for Compaction Threshold', () => {
  describe('Scenario: Parse percentage compaction threshold input', () => {
    it('should parse 80% as percentage type with value 80', () => {
      // @step Given a compaction threshold input field
      // (parser function accepts raw input string)

      // @step When the user enters "80%"
      const result = parseCompactionThreshold('80%');

      // @step Then the parsed value should be type "percentage" with value 80
      expect(result).toEqual({ type: 'percentage', value: 80 });
    });
  });

  describe('Scenario: Parse token count compaction threshold input', () => {
    it('should parse 200000 as tokens type with value 200000', () => {
      // @step Given a compaction threshold input field
      // (parser function accepts raw input string)

      // @step When the user enters "200000"
      const result = parseCompactionThreshold('200000');

      // @step Then the parsed value should be type "tokens" with value 200000
      expect(result).toEqual({ type: 'tokens', value: 200000 });
    });
  });

  describe('Scenario: Empty compaction threshold uses built-in default', () => {
    it('should return undefined for empty string', () => {
      // @step Given a compaction threshold input field
      // (parser function accepts raw input string)

      // @step When the user enters ""
      const result = parseCompactionThreshold('');

      // @step Then the parsed value should be undefined
      expect(result).toBeUndefined();
    });
  });

  describe('Scenario: Reject invalid percentage values', () => {
    it('should return undefined for 0% and 101%', () => {
      // @step Given a compaction threshold input field
      // (parser function accepts raw input string)

      // @step When the user enters "0%" or "101%"
      const result0 = parseCompactionThreshold('0%');
      const result101 = parseCompactionThreshold('101%');

      // @step Then the parsed value should be undefined
      expect(result0).toBeUndefined();
      expect(result101).toBeUndefined();
    });
  });

  describe('Scenario: Reject token count below minimum threshold', () => {
    it('should return undefined for values below 1000', () => {
      // @step Given a compaction threshold input field
      // (parser function accepts raw input string)

      // @step When the user enters "500"
      const result = parseCompactionThreshold('500');

      // @step Then the parsed value should be undefined because it is below 1000
      expect(result).toBeUndefined();
    });
  });

  // ===========================================================================
  // Form Field Constants Tests
  // ===========================================================================

  describe('Scenario: Provider Settings Panel includes compaction threshold field', () => {
    it('should have compactionThreshold after maxOutputTokens', () => {
      // @step Given the Provider Settings Panel form field list
      const fields = PROFILE_FORM_FIELDS;

      // @step Then "compactionThreshold" should appear after "maxOutputTokens"
      const maxOutputIdx = fields.indexOf('maxOutputTokens');
      const compactionIdx = fields.indexOf('compactionThreshold');
      expect(compactionIdx).toBeGreaterThan(-1);
      expect(compactionIdx).toBe(maxOutputIdx + 1);
    });
  });

  describe('Scenario: Custom Model Form includes compaction threshold field', () => {
    it('should have compactionThreshold between maxOutputTokens and reasoning', () => {
      // @step Given the Custom Model Form field list
      const fields = CUSTOM_MODEL_FORM_FIELDS;

      // @step Then "compactionThreshold" should appear between "maxOutputTokens" and "reasoning"
      const keys = fields.map((f: { key: string }) => f.key);
      const maxOutputIdx = keys.indexOf('maxOutputTokens');
      const compactionIdx = keys.indexOf('compactionThreshold');
      const reasoningIdx = keys.indexOf('reasoning');

      expect(compactionIdx).toBeGreaterThan(maxOutputIdx);
      expect(compactionIdx).toBeLessThan(reasoningIdx);
    });
  });

  // ===========================================================================
  // Type System Tests
  // ===========================================================================

  describe('Scenario: ModelSelection type includes compactionThreshold', () => {
    it('should allow compactionThreshold in ModelSelection', () => {
      // @step Given the ModelSelection interface
      const selection: ModelSelection = {
        providerId: 'openai',
        modelId: 'test',
        apiModelId: 'test',
        displayName: 'Test',
        reasoning: false,
        hasVision: false,
        contextWindow: 128000,
        maxOutput: 16384,
      };

      // @step Then it should have an optional compactionThreshold field of type CompactionThresholdConfig
      const withThreshold: ModelSelection = {
        ...selection,
        compactionThreshold: { type: 'tokens', value: 100000 },
      };
      expect(withThreshold.compactionThreshold).toEqual({
        type: 'tokens',
        value: 100000,
      });
      expect(selection.compactionThreshold).toBeUndefined();
    });
  });

  // ===========================================================================
  // NAPI Bridge Tests
  // ===========================================================================

  describe('NAPI Bridge', () => {
    beforeEach(() => {
      vi.clearAllMocks();
      napiMocks.sessionSetModel.mockResolvedValue(undefined);
      napiMocks.sessionSetModelProfile.mockResolvedValue(undefined);
      configMocks.loadConfig.mockResolvedValue({});
      configMocks.writeConfig.mockResolvedValue(undefined);
    });

    describe('Scenario: Model selection service passes compaction threshold to NAPI for profile models', () => {
      it('should pass compaction threshold params to sessionSetModelProfile', async () => {
        // @step Given a ModelSelection with compactionThreshold type "tokens" and value 100000
        // @step And the model is a profile-based model
        const selection: ModelSelection = {
          providerId: 'openai',
          modelId: 'local-model',
          apiModelId: 'local-model',
          displayName: 'Local Model',
          reasoning: false,
          hasVision: false,
          contextWindow: 128000,
          maxOutput: 16384,
          profileName: 'my-profile',
          profileConfig: {
            baseUrl: 'http://localhost:8888',
            apiKey: 'test-key',
          },
          compactionThreshold: { type: 'tokens', value: 100000 },
        };

        // @step When the model selection service applies the selection
        const result = await selectModel({
          sessionId: 'session-123',
          selection,
        });

        expect(result.success).toBe(true);

        // @step Then sessionSetModelProfile should be called with compactionThresholdType "tokens" and compactionThresholdValue 100000
        expect(napiMocks.sessionSetModelProfile).toHaveBeenCalledWith(
          'session-123',
          'openai',
          'local-model',
          128000,
          16384,
          null,
          'tokens',
          100000,
          'my-profile'
        );
      });
    });

    describe('Scenario: Model selection service passes compaction threshold to NAPI for cloud models', () => {
      it('should pass compaction threshold params to sessionSetModel', async () => {
        // @step Given a ModelSelection with compactionThreshold type "percentage" and value 80
        // @step And the model is a cloud provider model
        const selection: ModelSelection = {
          providerId: 'anthropic',
          modelId: 'claude-sonnet-4',
          apiModelId: 'claude-sonnet-4-20250514',
          displayName: 'Claude Sonnet 4',
          reasoning: true,
          hasVision: true,
          contextWindow: 200000,
          maxOutput: 16000,
          compactionThreshold: { type: 'percentage', value: 80 },
        };

        // @step When the model selection service applies the selection
        const result = await selectModel({
          sessionId: 'session-456',
          selection,
        });

        expect(result.success).toBe(true);

        // @step Then sessionSetModel should be called with compactionThresholdType "percentage" and compactionThresholdValue 80
        expect(napiMocks.sessionSetModel).toHaveBeenCalledWith(
          'session-456',
          'anthropic',
          'claude-sonnet-4',
          200000,
          16000,
          'percentage',
          80
        );
      });
    });

    describe('Scenario: Model selection service omits compaction threshold when not configured', () => {
      it('should pass null for compaction threshold when not set', async () => {
        // @step Given a ModelSelection without compactionThreshold
        const selection: ModelSelection = {
          providerId: 'anthropic',
          modelId: 'claude-sonnet-4',
          apiModelId: 'claude-sonnet-4-20250514',
          displayName: 'Claude Sonnet 4',
          reasoning: true,
          hasVision: true,
          contextWindow: 200000,
          maxOutput: 16000,
        };

        // @step When the model selection service applies the selection
        const result = await selectModel({
          sessionId: 'session-789',
          selection,
        });

        expect(result.success).toBe(true);

        // @step Then the NAPI call should pass null for compaction threshold parameters
        expect(napiMocks.sessionSetModel).toHaveBeenCalledWith(
          'session-789',
          'anthropic',
          'claude-sonnet-4',
          200000,
          16000,
          null,
          null
        );
      });
    });
  }); // end NAPI Bridge

  // ===========================================================================
  // NAPI Type Declaration Tests
  // ===========================================================================

  describe('Scenario: NAPI type declarations include compaction threshold parameters', () => {
    it('should have compaction threshold params in sessionSetModel and sessionSetModelProfile', () => {
      // @step Given the codelet-napi index.d.ts type declarations
      const indexDtsPath = join(process.cwd(), 'codelet/napi/index.d.ts');
      const content = readFileSync(indexDtsPath, 'utf-8');

      // @step Then sessionSetModel should accept optional compactionThresholdType and compactionThresholdValue parameters
      const setModelDecl = content.match(
        /export declare function sessionSetModel\([^)]+\)/
      );
      expect(setModelDecl).not.toBeNull();
      expect(setModelDecl![0]).toContain('compactionThresholdType');
      expect(setModelDecl![0]).toContain('compactionThresholdValue');

      // @step And sessionSetModelProfile should accept optional compactionThresholdType and compactionThresholdValue parameters
      const setModelProfileDecl = content.match(
        /export declare function sessionSetModelProfile\([^)]+\)/
      );
      expect(setModelProfileDecl).not.toBeNull();
      expect(setModelProfileDecl![0]).toContain('compactionThresholdType');
      expect(setModelProfileDecl![0]).toContain('compactionThresholdValue');
    });
  });

  // ===========================================================================
  // Integration Tests
  // ===========================================================================

  describe('Scenario: Profile compaction threshold flows through when model has none', () => {
    beforeEach(() => {
      vi.clearAllMocks();
      napiMocks.sessionSetModel.mockResolvedValue(undefined);
      napiMocks.sessionSetModelProfile.mockResolvedValue(undefined);
      configMocks.loadConfig.mockResolvedValue({});
      configMocks.writeConfig.mockResolvedValue(undefined);
    });

    it('should pass profile-level threshold when model has none', async () => {
      // @step Given a profile with compactionThreshold type "percentage" and value 75
      // @step And a custom model without a compactionThreshold override
      const selection: ModelSelection = {
        providerId: 'openai',
        modelId: 'custom-model',
        apiModelId: 'custom-model',
        displayName: 'Custom Model',
        reasoning: false,
        hasVision: false,
        contextWindow: 128000,
        maxOutput: 16384,
        profileName: 'my-profile',
        profileConfig: {
          baseUrl: 'http://localhost:8888',
          apiKey: 'test-key',
          compactionThreshold: { type: 'percentage', value: 75 },
        },
        compactionThreshold: { type: 'percentage', value: 75 },
      };

      // @step When the user selects the custom model from that profile
      const result = await selectModel({
        sessionId: 'session-abc',
        selection,
      });

      expect(result.success).toBe(true);

      // @step Then the profile-level compaction threshold should be passed to the NAPI call
      expect(napiMocks.sessionSetModelProfile).toHaveBeenCalledWith(
        'session-abc',
        'openai',
        'custom-model',
        128000,
        16384,
        null,
        'percentage',
        75,
        'my-profile'
      );
    });
  });
});
