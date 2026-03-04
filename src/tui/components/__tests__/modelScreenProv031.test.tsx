/**
 * Feature: spec/features/model-screen-stale-profile-sections-from-non-openai-providers-footer-text-unreachable-section-filtering.feature
 *
 * PROV-031: Model Screen — stale profile sections from non-OpenAI providers,
 * footer text, unreachable section filtering.
 *
 * Tests validate 4 targeted bug fixes:
 *   Fix 1: loadProfileSections() only loads 'openai' profiles (not all SUPPORTED_PROVIDERS)
 *   Fix 2: Unreachable + 0 models sections are filtered from initializeModels()
 *   Fix 3: ModelSelectorView footer says 'Tab: Switch to providers' not 'Tab: settings'
 *   Fix 4: Model count shows only model rows, not section headers, labeled 'N models'
 *
 * Scenarios 1–3 test the model initialization service.
 * Scenarios 4–5 test rendered ModelSelectorView component behaviour via ink-testing-library.
 */

import React from 'react';
import { render, cleanup } from 'ink-testing-library';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { join } from 'path';
import { mkdir, writeFile } from 'fs/promises';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../../test-helpers/universal-test-setup';
import { useModelStore } from '../../store/modelStore';
import type { ModelSelectorItem, ProviderSection } from '../../types/provider';
import type { NapiModelInfo } from '@sengac/codelet-napi';
import { ModelSelectorView } from '../ModelSelectorView';

// =============================================================================
// NAPI MOCKS
// =============================================================================

const napiMocks = vi.hoisted(() => ({
  modelsListAll: vi.fn(),
  modelsListLocalOpenai: vi.fn(),
  codexOauthGetTokens: vi.fn().mockReturnValue(null),
  claudeOauthGetTokens: vi.fn().mockResolvedValue(null),
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

import { initializeModels } from '../../services/modelInitializationService';

// =============================================================================
// HELPERS
// =============================================================================

function makeOpenAICloudProvider() {
  return {
    providerId: 'openai',
    providerName: 'OpenAI',
    models: [
      {
        id: 'gpt-4o',
        name: 'GPT-4o',
        reasoning: false,
        toolCall: true,
        attachment: false,
        temperature: true,
        contextWindow: 128000,
        maxOutput: 16384,
        hasVision: true,
      },
    ],
  };
}

/**
 * Build a flat item list with N sections and M model rows each.
 *
 * FIX-7: section.models is populated to match the flat model items so that
 * any code reading item.section.models sees consistent data with the flat list.
 * Previously section.models was always [] even when model items existed —
 * a structural inconsistency that was a future trap.
 */
function buildFlatItems(
  sectionCount: number,
  modelsPerSection: number
): ModelSelectorItem[] {
  const items: ModelSelectorItem[] = [];
  for (let s = 0; s < sectionCount; s++) {
    // Build models array first so section.models and flat model items share the same objects
    const models: NapiModelInfo[] = Array.from(
      { length: modelsPerSection },
      (_, m) => ({
        id: `model-${s}-${m}`,
        name: `Model ${s}-${m}`,
        reasoning: false,
        toolCall: true,
        attachment: false,
        temperature: true,
        contextWindow: 128000,
        maxOutput: 4096,
        hasVision: false,
      })
    );

    const section: ProviderSection = {
      providerId: `provider-${s}`,
      providerName: `Provider ${s}`,
      internalName: `provider-${s}`,
      models, // FIX-7: populated — consistent with flat model items
      hasCredentials: true,
    };

    // Section header item
    items.push({
      type: 'section',
      sectionIdx: s,
      section,
      isExpanded: true,
    } as ModelSelectorItem);

    // Model items — reference same NapiModelInfo objects as section.models
    for (let m = 0; m < modelsPerSection; m++) {
      items.push({
        type: 'model',
        sectionIdx: s,
        modelIdx: m,
        section,
        model: models[m],
      } as ModelSelectorItem);
    }
  }
  return items;
}

// =============================================================================
// TESTS
// =============================================================================

describe('Feature: Model Screen — PROV-031 bug fixes', () => {
  let setup: TestDirectorySetup;
  let originalHome: string | undefined;
  let originalCwd: string;
  let originalEnvVars: Record<string, string | undefined>;

  const credentialEnvVars = [
    'ANTHROPIC_API_KEY',
    'CLAUDE_CODE_OAUTH_TOKEN',
    'OPENAI_API_KEY',
    'GOOGLE_API_KEY',
    'GEMINI_API_KEY',
  ];

  beforeEach(async () => {
    useModelStore.getState().reset();
    napiMocks.modelsListAll.mockReset();
    napiMocks.modelsListLocalOpenai.mockReset();
    napiMocks.codexOauthGetTokens.mockReturnValue(null);
    napiMocks.claudeOauthGetTokens.mockResolvedValue(null);

    originalEnvVars = {};
    for (const envVar of credentialEnvVars) {
      originalEnvVars[envVar] = process.env[envVar];
      delete process.env[envVar];
    }

    setup = await setupTestDirectory('prov-031');
    originalHome = process.env.HOME;
    originalCwd = process.cwd();

    process.env.HOME = setup.testDir;
    process.chdir(setup.testDir);

    await mkdir(join(setup.testDir, '.fspec', 'credentials'), {
      recursive: true,
    });
  });

  afterEach(async () => {
    cleanup(); // Clean up any rendered Ink components
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

  // =====================================================
  // Fix 1: Stale non-OpenAI profiles must never appear
  // =====================================================

  describe('Scenario: Stale non-OpenAI provider profiles are never loaded into the model screen', () => {
    it('should produce zero profile sections for anthropic and gemini, only load openai profiles', async () => {
      // @step Given the user config has a stale anthropic profile pointing to localhost:8888 from OAuth dev work
      const configContent = {
        providers: {
          anthropic: {
            profiles: {
              'test-profile': {
                baseUrl: 'http://localhost:8888',
                apiKey: 'test-key',
              },
            },
          },
          // @step And the user config has a stale gemini profile pointing to a local server
          gemini: {
            profiles: {
              'test-profile': {
                baseUrl: 'http://localhost:9999',
                apiKey: 'test-key',
              },
            },
          },
          openai: {
            profiles: {},
          },
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'fspec-config.json'),
        JSON.stringify(configContent, null, 2)
      );

      // @step When loadProfileSections() runs in modelInitializationService.ts
      napiMocks.modelsListAll.mockResolvedValue([makeOpenAICloudProvider()]);
      const credentialsContent = {
        version: 1,
        providers: {
          openai: {
            apiKey: 'sk-openai-test',
            lastUpdated: new Date().toISOString(),
          },
        },
      };
      await writeFile(
        join(setup.testDir, '.fspec', 'credentials', 'credentials.json'),
        JSON.stringify(credentialsContent, null, 2),
        { mode: 0o600 }
      );

      const result = await initializeModels();

      // @step Then zero profile sections are generated for anthropic
      const anthropicProfiles = result.sections.filter(
        s => s.providerId === 'anthropic' && s.profileName !== undefined
      );
      expect(anthropicProfiles).toHaveLength(0);

      // @step And zero profile sections are generated for gemini
      const geminiProfiles = result.sections.filter(
        s => s.providerId === 'gemini' && s.profileName !== undefined
      );
      expect(geminiProfiles).toHaveLength(0);

      // @step And only 'openai' provider profiles are iterated during profile loading
      // No call should have been made to localhost:8888 (anthropic) or localhost:9999 (gemini)
      const calledUrls = napiMocks.modelsListLocalOpenai.mock.calls.map(
        (c: unknown[]) => c[0]
      );
      expect(calledUrls).not.toContain('http://localhost:8888');
      expect(calledUrls).not.toContain('http://localhost:9999');
    });
  });

  // =====================================================
  // Fix 2: Unreachable + 0 models filter
  // =====================================================

  describe('Scenario: Unreachable OpenAI profile with zero models is filtered from the model screen', () => {
    it('should not include an unreachable openai profile section that has zero models', async () => {
      // @step Given the user config has an openai profile 'qwen3-coder-next' pointing to an offline server
      const configContent = {
        providers: {
          openai: {
            profiles: {
              'qwen3-coder-next': {
                baseUrl: 'http://localhost:7777',
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

      // @step When the model screen initializes
      napiMocks.modelsListAll.mockResolvedValue([makeOpenAICloudProvider()]);

      // Server is offline — modelsListLocalOpenai throws
      napiMocks.modelsListLocalOpenai.mockRejectedValue(
        new Error('ECONNREFUSED')
      );

      const result = await initializeModels();

      // @step Then the profile section for 'qwen3-coder-next' does not appear in the model list
      const offlineSection = result.sections.find(
        s => s.profileName === 'qwen3-coder-next'
      );
      expect(offlineSection).toBeUndefined();

      // @step And no '(unreachable) (0 models)' entry is shown
      const unreachableEmptySections = result.sections.filter(
        s => s.isUnreachable === true && s.models.length === 0
      );
      expect(unreachableEmptySections).toHaveLength(0);
    });
  });

  describe('Scenario: Reachable OpenAI profile with models always appears in the model screen', () => {
    it('should include openai profile section when server is reachable and returns models', async () => {
      // @step Given the user config has an openai profile 'work-vllm' pointing to a live server
      const configContent = {
        providers: {
          openai: {
            profiles: {
              'work-vllm': {
                baseUrl: 'http://localhost:8000',
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

      // @step And the live server returns 5 models
      napiMocks.modelsListAll.mockResolvedValue([]);
      napiMocks.modelsListLocalOpenai.mockResolvedValue([
        'model-a',
        'model-b',
        'model-c',
        'model-d',
        'model-e',
      ]);

      // @step When the model screen initializes
      const result = await initializeModels();

      // @step Then the profile section 'openai: work-vllm (5 models)' appears at the top of the model list
      const workVllmSection = result.sections.find(
        s => s.profileName === 'work-vllm'
      );
      expect(workVllmSection).toBeDefined();
      expect(workVllmSection?.models).toHaveLength(5);
      expect(workVllmSection?.isUnreachable).toBeFalsy();
      // Profile sections appear first in the list
      expect(result.sections[0].profileName).toBe('work-vllm');
    });
  });

  // =====================================================
  // Fix 3: ModelSelectorView footer — rendered component behaviour
  // =====================================================

  describe('Scenario: Model screen footer shows symmetric Tab hint', () => {
    it('should render footer text containing Tab: Switch to providers', () => {
      // @step Given the model selector screen is rendered
      // Use width=120 so the footer string (87 chars) fits without wrapping
      const { lastFrame } = render(
        <ModelSelectorView
          width={120}
          height={24}
          flatItems={[]}
          selectedSectionIdx={0}
          selectedModelIdx={-1}
          expandedProviders={new Set()}
          scrollOffset={0}
          visibleHeight={10}
          filter=""
          isFilterMode={false}
          isRefreshing={false}
        />
      );
      const frame = lastFrame() ?? '';

      // @step When the footer is displayed
      // @step Then the footer text reads 'Enter: select | ←→: collapse/expand | r: refresh | Tab: Switch to providers | / filter | Esc: close'
      expect(frame).toContain('Tab: Switch to providers');
      expect(frame).not.toContain('Tab: settings');
      expect(frame).toContain('Enter: select');
      expect(frame).toContain('r: refresh');
      expect(frame).toContain('Esc: close');
      // Full footer string from the spec (FIX-5: assert complete string, not just one fragment)
      expect(frame).toContain(
        'r: refresh | Tab: Switch to providers | / filter | Esc: close'
      );
    });
  });

  // =====================================================
  // Fix 4: Header count — rendered component behaviour
  // =====================================================

  describe('Scenario: Model screen header counts only selectable model items, not section headers', () => {
    it('should count only model-type items and label them models', () => {
      // @step Given the model selector screen is rendered with provider sections and model rows
      // Build flatItems: 3 sections × 8 models each = 3 section headers + 24 model rows = 27 total
      const flatItems = buildFlatItems(3, 8);
      expect(flatItems).toHaveLength(27); // Sanity: 3 headers + 24 models

      const { lastFrame } = render(
        <ModelSelectorView
          width={120}
          height={24}
          flatItems={flatItems}
          selectedSectionIdx={0}
          selectedModelIdx={-1}
          expandedProviders={new Set()}
          scrollOffset={0}
          visibleHeight={20}
          filter=""
          isFilterMode={false}
          isRefreshing={false}
        />
      );
      const frame = lastFrame() ?? '';

      // @step When the header count is displayed
      // @step Then the count label reads 'N models' where N is the number of model rows only
      expect(frame).toContain('24 models');
      // @step And section header rows are not included in the count
      expect(frame).not.toContain('27 items');
      expect(frame).not.toContain('27 models');
    });
  });
});
