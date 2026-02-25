/**
 * NAPI Model Fixtures - Single Source of Truth for NAPI Model Test Data
 *
 * This module provides factory functions for creating NAPI-format model data
 * used in integration tests. All fixtures that need NAPI model data should
 * import from here to maintain DRY/SOLID/COMPOSABLE principles.
 *
 * ARCHITECTURE:
 * - provider-type-fixtures.ts: ProviderSection data (UI-level)
 * - napi-model-fixtures.ts (THIS FILE): NapiProviderModels data (NAPI-level)
 *
 * Both are complementary - use provider-type-fixtures for UI component tests,
 * use this file for NAPI mock configuration.
 */

import type { NapiModelInfo, NapiProviderModels } from '@sengac/codelet-napi';

// =============================================================================
// NAPI MODEL INFO BUILDERS
// =============================================================================

/**
 * Creates a NapiModelInfo with sensible defaults for tool-calling models.
 *
 * @example
 * ```ts
 * const model = createNapiModelInfo({ id: 'my-model', name: 'My Model' });
 * ```
 */
export function createNapiModelInfo(
  overrides: Partial<NapiModelInfo> = {}
): NapiModelInfo {
  return {
    id: 'test-model',
    name: 'Test Model',
    reasoning: false,
    toolCall: true,
    attachment: true,
    temperature: true,
    contextWindow: 128000,
    maxOutput: 8192,
    hasVision: false,
    ...overrides,
  };
}

/**
 * Creates a Claude-style model for Anthropic provider.
 */
export function createClaudeNapiModel(
  overrides: Partial<NapiModelInfo> = {}
): NapiModelInfo {
  return createNapiModelInfo({
    id: 'claude-sonnet-4-20250514',
    name: 'Claude Sonnet 4',
    reasoning: true,
    hasVision: true,
    contextWindow: 200000,
    maxOutput: 64000,
    ...overrides,
  });
}

/**
 * Creates a GPT-style model for OpenAI provider.
 */
export function createGptNapiModel(
  overrides: Partial<NapiModelInfo> = {}
): NapiModelInfo {
  return createNapiModelInfo({
    id: 'gpt-4.1',
    name: 'GPT-4.1',
    reasoning: false,
    hasVision: true,
    contextWindow: 128000,
    maxOutput: 16384,
    ...overrides,
  });
}

// =============================================================================
// NAPI PROVIDER MODELS BUILDERS
// =============================================================================

/**
 * Creates Anthropic provider models for NAPI response.
 *
 * This is what modelsListAll() returns for Anthropic.
 */
export function createAnthropicNapiModels(): NapiProviderModels {
  return {
    providerId: 'anthropic',
    providerName: 'Anthropic',
    models: [
      createClaudeNapiModel({
        id: 'claude-sonnet-4-20250514',
        name: 'Claude Sonnet 4',
      }),
      createClaudeNapiModel({
        id: 'claude-opus-4-20250514',
        name: 'Claude Opus 4',
      }),
    ],
  };
}

/**
 * Creates OpenAI provider models for NAPI response.
 *
 * This is what modelsListAll() returns for OpenAI.
 */
export function createOpenAiNapiModels(): NapiProviderModels {
  return {
    providerId: 'openai',
    providerName: 'OpenAI',
    models: [
      createGptNapiModel({
        id: 'gpt-4.1',
        name: 'GPT-4.1',
      }),
      createGptNapiModel({
        id: 'gpt-4.1-mini',
        name: 'GPT-4.1 Mini',
      }),
    ],
  };
}

/**
 * Creates default cloud providers for testing.
 *
 * This is the standard response for modelsListAll() in tests.
 *
 * @example
 * ```ts
 * vi.mock('@sengac/codelet-napi', () => ({
 *   modelsListAll: vi.fn(async () => createDefaultCloudProviders()),
 * }));
 * ```
 */
export function createDefaultCloudProviders(): NapiProviderModels[] {
  return [createAnthropicNapiModels(), createOpenAiNapiModels()];
}

// =============================================================================
// LOCAL SERVER HELPERS
// =============================================================================

/**
 * Creates model IDs for a local Ollama-style server.
 *
 * This is what modelsListLocalOpenai() returns.
 */
export function createLocalServerModels(): string[] {
  return ['llama3', 'codellama', 'mistral'];
}

/**
 * Creates model IDs for a local vLLM-style server with full model paths.
 */
export function createVllmServerModels(): string[] {
  return ['Qwen/Qwen3-80B', 'mistralai/Mistral-7B-v0.1'];
}

// =============================================================================
// TEST MODEL IDS - Constants for assertions
// =============================================================================

/**
 * Model IDs used in test fixtures (for assertions).
 */
export const TEST_MODEL_IDS = {
  // Anthropic models
  claudeSonnet4: 'claude-sonnet-4-20250514',
  claudeOpus4: 'claude-opus-4-20250514',

  // OpenAI models
  gpt41: 'gpt-4.1',
  gpt41Mini: 'gpt-4.1-mini',

  // Local models
  llama3: 'llama3',
  codellama: 'codellama',
} as const;

/**
 * Provider IDs used in test fixtures.
 */
export const TEST_PROVIDER_IDS = {
  anthropic: 'anthropic',
  openai: 'openai',
} as const;

/**
 * Provider display names (as shown in UI).
 */
export const TEST_PROVIDER_NAMES = {
  anthropic: 'Anthropic',
  openai: 'OpenAI',
} as const;
