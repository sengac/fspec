/**
 * Feature: spec/features/custom-provider-vision-support.feature
 *
 * PROV-096: customProviderSectionBuilder must forward `supportsVision` from
 * the widened `JsProviderModelInfo` NAPI shape into `NapiModelInfo.hasVision`
 * so that the SessionHeader [V] badge renders for custom / Rhai-scripted
 * provider models that declare `supports_vision: true` in their JSON config.
 *
 * In the red phase the current `buildCustomModelInfo` hardcodes
 * `hasVision: false`, so these assertions fail — that is the whole point.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { NapiModelInfo } from '@sengac/codelet-napi';

// ===========================================================================
// NAPI BOUNDARY MOCK — widened shape including supportsVision (PROV-096).
// ===========================================================================

interface MockJsProviderModelInfo {
  id: string;
  contextWindow: number;
  maxOutput: number;
  supportsTools: boolean;
  supportsStreaming: boolean;
  supportsThinking: boolean;
  supportsVision?: boolean;
}

interface MockJsProviderInfo {
  name: string;
  displayName?: string;
  available: boolean;
  isCustom: boolean;
  facade?: string;
  baseUrl?: string;
  apiKeyEnvVar?: string;
  models: MockJsProviderModelInfo[];
  apiStyle?: string;
}

const napiMocks = vi.hoisted(() => ({
  listProviders: vi.fn<[], Promise<MockJsProviderInfo[]>>(),
}));

vi.mock('@sengac/codelet-napi', () => ({
  listProviders: () => napiMocks.listProviders(),
}));

vi.mock('../../../utils/logger', () => ({
  logger: { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() },
}));

// Import AFTER mocks so the module picks up the mocked NAPI binding.
import { loadCustomProviderSections } from '../customProviderSectionBuilder';

// ===========================================================================
// Test fixtures
// ===========================================================================

function opus47Entry(
  overrides: Partial<MockJsProviderModelInfo> = {}
): MockJsProviderModelInfo {
  return {
    id: 'opus-4.7',
    contextWindow: 200_000,
    maxOutput: 64_000,
    supportsTools: true,
    supportsStreaming: true,
    supportsThinking: true,
    supportsVision: false,
    ...overrides,
  };
}

function claudeRhaiProvider(
  modelOverrides: Partial<MockJsProviderModelInfo> = {}
): MockJsProviderInfo {
  return {
    name: 'claude-rhai',
    displayName: 'Claude (Rhai)',
    available: true,
    isCustom: true,
    facade: undefined,
    baseUrl: 'https://api.anthropic.com',
    apiKeyEnvVar: 'ANTHROPIC_API_KEY',
    models: [opus47Entry(modelOverrides)],
    apiStyle: 'anthropic_messages',
  };
}

// ===========================================================================
// Tests — map 1:1 to Gherkin scenarios in custom-provider-vision-support.feature
// ===========================================================================

describe('Feature: Custom provider vision support propagates through builder to SessionHeader', () => {
  beforeEach(() => {
    napiMocks.listProviders.mockReset();
    // @step Given listProviders() is mocked to return a "claude-rhai" custom provider entry
    // (See each scenario for the specific model payload under test.)
  });

  describe('Scenario: Custom provider JSON with supports_vision true propagates to NapiModelInfo.hasVision', () => {
    it('hasVision is true when NAPI entry carries supportsVision true', async () => {
      // @step Given a custom provider JSON config with a model definition
      // @step And the model definition sets "supports_vision" to true
      napiMocks.listProviders.mockResolvedValue([
        claudeRhaiProvider({ supportsVision: true }),
      ]);

      // @step When the TUI's custom provider section builder loads the model
      const sections = await loadCustomProviderSections();
      const model: NapiModelInfo = sections[0].models[0];

      // @step Then the resulting NapiModelInfo has hasVision set to true
      expect(model.hasVision).toBe(true);
    });
  });

  describe('Scenario: Custom provider model without supports_vision defaults hasVision to false', () => {
    it('hasVision is false when NAPI entry omits supportsVision', async () => {
      // @step Given a custom provider JSON config with a model definition
      // @step And the model definition omits the "supports_vision" field
      napiMocks.listProviders.mockResolvedValue([
        claudeRhaiProvider({ supportsVision: undefined }),
      ]);

      // @step When the TUI's custom provider section builder loads the model
      const sections = await loadCustomProviderSections();
      const model: NapiModelInfo = sections[0].models[0];

      // @step Then the resulting NapiModelInfo has hasVision set to false
      expect(model.hasVision).toBe(false);
    });
  });

  describe('Scenario: Legacy config without supports_vision loads cleanly without regression', () => {
    it('hasVision stays false when NAPI entry carries supportsVision false', async () => {
      // @step Given an existing custom provider JSON config that has no "supports_vision" field anywhere
      napiMocks.listProviders.mockResolvedValue([
        claudeRhaiProvider({ supportsVision: false }),
      ]);

      // @step When the provider system deserializes and loads the config
      const sections = await loadCustomProviderSections();
      const model: NapiModelInfo = sections[0].models[0];

      // @step Then all models deserialize successfully with supports_vision defaulting to false
      expect(model.hasVision).toBe(false);
      // And the SessionHeader does not show the "[V]" badge for any of those models
      // (covered by the above assertion — hasVision=false prevents [V]).
    });
  });

  describe('Scenario: SessionHeader renders [V] badge for vision-enabled custom model', () => {
    it('hasVision is true so downstream SessionHeader would render [V]', async () => {
      // @step Given a custom provider "claude-rhai" is registered with model "opus-4.7"
      // @step And the model "opus-4.7" declares "supports_vision" as true
      napiMocks.listProviders.mockResolvedValue([
        claudeRhaiProvider({
          id: 'opus-4.7',
          contextWindow: 200_000,
          supportsVision: true,
          supportsThinking: true, // so [R] also shows
        }),
      ]);

      // @step And the developer selects the "opus-4.7" model in the /model selector
      // (implied — model is present in the returned section)

      // @step When the AgentView renders the SessionHeader for the active session
      const sections = await loadCustomProviderSections();
      const model: NapiModelInfo = sections[0].models[0];

      // @step Then the SessionHeader shows the blue "[V]" badge alongside "[R]"
      expect(model.hasVision).toBe(true);
      expect(model.reasoning).toBe(true); // [R] badge prerequisite
    });
  });
});
