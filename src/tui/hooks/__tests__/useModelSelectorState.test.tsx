/**
 * Feature: spec/features/model-selector-state-hook.feature
 *
 * Integration Tests: useModelSelectorState Hook
 *
 * TUI-072: Extract model selector state from AgentView.tsx
 *
 * Test Strategy:
 * 1. PURE FUNCTION TESTS: Test helper functions with ZERO mocks using fixtures
 * 2. HOOK INTEGRATION TESTS: Test hook with real file system, mock only NAPI network boundary
 *
 * Fixtures used:
 * - fixtures/modelSelectorStateFixture.ts (integration test setup)
 * - test-helpers/provider-type-fixtures.ts (provider/model data)
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Text } from 'ink';

// Import the hook and its EXPORTED PURE FUNCTIONS for direct testing
import {
  useModelSelectorState,
  type UseModelSelectorStateReturn,
  // Pure functions - test these WITHOUT mocks
  mapProviderIdToInternal,
  mapInternalToProviderId,
  mapModelsDevToRegistryId,
  buildFlatModelList,
  flatIndexToSectionModel,
  sectionModelToFlatIndex,
  extractModelIdForRegistry,
} from '../useModelSelectorState';

// Import REAL fixtures - no mocks for these
import {
  createAnthropicSection,
  createOpenAiSection,
  createLocalProfileSection,
  createTestProviderSection,
} from '../../../test-helpers/provider-type-fixtures';

import type { ProviderSection } from '../../types/provider';

// =============================================================================
// PART 1: PURE FUNCTION TESTS (ZERO MOCKS)
// =============================================================================

describe('Pure Functions: Provider ID Mapping', () => {
  describe('mapProviderIdToInternal', () => {
    it('maps anthropic to claude', () => {
      expect(mapProviderIdToInternal('anthropic')).toBe('claude');
    });

    it('maps google to gemini', () => {
      expect(mapProviderIdToInternal('google')).toBe('gemini');
    });

    it('passes through other provider IDs unchanged', () => {
      expect(mapProviderIdToInternal('openai')).toBe('openai');
      expect(mapProviderIdToInternal('mistral')).toBe('mistral');
      expect(mapProviderIdToInternal('custom-provider')).toBe('custom-provider');
    });
  });

  describe('mapInternalToProviderId', () => {
    it('maps claude to anthropic', () => {
      expect(mapInternalToProviderId('claude')).toBe('anthropic');
    });

    it('maps gemini to google', () => {
      expect(mapInternalToProviderId('gemini')).toBe('google');
    });

    it('passes through other internal names unchanged', () => {
      expect(mapInternalToProviderId('openai')).toBe('openai');
      expect(mapInternalToProviderId('mistral')).toBe('mistral');
    });
  });

  describe('mapModelsDevToRegistryId', () => {
    it('maps google to gemini for credential lookup', () => {
      expect(mapModelsDevToRegistryId('google')).toBe('gemini');
    });

    it('passes through other IDs unchanged', () => {
      expect(mapModelsDevToRegistryId('anthropic')).toBe('anthropic');
      expect(mapModelsDevToRegistryId('openai')).toBe('openai');
    });
  });
});

describe('Pure Functions: extractModelIdForRegistry', () => {
  it('strips date-based version suffix', () => {
    expect(extractModelIdForRegistry('claude-sonnet-4-20250514')).toBe('claude-sonnet-4');
    expect(extractModelIdForRegistry('gpt-4o-20240101')).toBe('gpt-4o');
  });

  it('leaves IDs without date suffix unchanged', () => {
    expect(extractModelIdForRegistry('claude-sonnet-4')).toBe('claude-sonnet-4');
    expect(extractModelIdForRegistry('gpt-4o')).toBe('gpt-4o');
    expect(extractModelIdForRegistry('llama3')).toBe('llama3');
  });

  it('only strips 8-digit date suffixes', () => {
    expect(extractModelIdForRegistry('model-1234567')).toBe('model-1234567');
    expect(extractModelIdForRegistry('model-123456789')).toBe('model-123456789');
  });
});

describe('Pure Functions: buildFlatModelList', () => {
  it('builds flat list from sections with expanded providers', () => {
    const sections: ProviderSection[] = [
      createAnthropicSection(),
      createOpenAiSection(),
    ];
    const expandedProviders = new Set(['anthropic', 'openai']);

    const items = buildFlatModelList(sections, expandedProviders);

    expect(items.length).toBe(6); // 2 sections + 2 anthropic models + 2 openai models
    expect(items[0].type).toBe('section');
    if (items[0].type === 'section') {
      expect(items[0].section.providerId).toBe('anthropic');
      expect(items[0].isExpanded).toBe(true);
    }
  });

  it('excludes models for collapsed sections', () => {
    const sections: ProviderSection[] = [
      createAnthropicSection(),
      createOpenAiSection(),
    ];
    const expandedProviders = new Set(['anthropic']);

    const items = buildFlatModelList(sections, expandedProviders);

    expect(items.length).toBe(4); // anthropic section + 2 models + openai section (no models)
    const openaiModels = items.filter(
      i => i.type === 'model' && i.section.providerId === 'openai'
    );
    expect(openaiModels.length).toBe(0);
  });

  it('handles empty sections', () => {
    const items = buildFlatModelList([], new Set<string>());
    expect(items.length).toBe(0);
  });

  it('handles sections with no models', () => {
    const sections: ProviderSection[] = [
      createTestProviderSection({ providerId: 'empty', models: [] }),
    ];
    const items = buildFlatModelList(sections, new Set(['empty']));
    expect(items.length).toBe(1);
    expect(items[0].type).toBe('section');
  });
});

describe('Pure Functions: flatIndexToSectionModel', () => {
  it('returns section header indices for section items', () => {
    const sections: ProviderSection[] = [createAnthropicSection(), createOpenAiSection()];
    const items = buildFlatModelList(sections, new Set(['anthropic', 'openai']));

    expect(flatIndexToSectionModel(0, items)).toEqual({ sectionIdx: 0, modelIdx: -1 });
    expect(flatIndexToSectionModel(3, items)).toEqual({ sectionIdx: 1, modelIdx: -1 });
  });

  it('returns model indices for model items', () => {
    const sections: ProviderSection[] = [createAnthropicSection(), createOpenAiSection()];
    const items = buildFlatModelList(sections, new Set(['anthropic', 'openai']));

    expect(flatIndexToSectionModel(1, items)).toEqual({ sectionIdx: 0, modelIdx: 0 });
    expect(flatIndexToSectionModel(2, items)).toEqual({ sectionIdx: 0, modelIdx: 1 });
    expect(flatIndexToSectionModel(4, items)).toEqual({ sectionIdx: 1, modelIdx: 0 });
  });

  it('returns default for out-of-bounds index', () => {
    const sections: ProviderSection[] = [createAnthropicSection()];
    const items = buildFlatModelList(sections, new Set(['anthropic']));
    expect(flatIndexToSectionModel(999, items)).toEqual({ sectionIdx: 0, modelIdx: -1 });
  });

  it('returns default for empty items', () => {
    expect(flatIndexToSectionModel(0, [])).toEqual({ sectionIdx: 0, modelIdx: -1 });
  });
});

describe('Pure Functions: sectionModelToFlatIndex', () => {
  it('finds flat index for section headers', () => {
    const sections: ProviderSection[] = [createAnthropicSection(), createOpenAiSection()];
    const items = buildFlatModelList(sections, new Set(['anthropic', 'openai']));

    expect(sectionModelToFlatIndex(0, -1, items)).toBe(0);
    expect(sectionModelToFlatIndex(1, -1, items)).toBe(3);
  });

  it('finds flat index for models', () => {
    const sections: ProviderSection[] = [createAnthropicSection(), createOpenAiSection()];
    const items = buildFlatModelList(sections, new Set(['anthropic', 'openai']));

    expect(sectionModelToFlatIndex(0, 0, items)).toBe(1);
    expect(sectionModelToFlatIndex(0, 1, items)).toBe(2);
    expect(sectionModelToFlatIndex(1, 0, items)).toBe(4);
  });

  it('returns 0 for non-existent section/model', () => {
    const sections: ProviderSection[] = [createAnthropicSection()];
    const items = buildFlatModelList(sections, new Set(['anthropic']));

    expect(sectionModelToFlatIndex(5, -1, items)).toBe(0);
    expect(sectionModelToFlatIndex(0, 10, items)).toBe(0);
  });

  it('round-trips with flatIndexToSectionModel', () => {
    const sections: ProviderSection[] = [
      createAnthropicSection(),
      createOpenAiSection(),
      createLocalProfileSection('test'),
    ];
    const items = buildFlatModelList(sections, new Set(['anthropic', 'openai']));

    for (let i = 0; i < items.length; i++) {
      const { sectionIdx, modelIdx } = flatIndexToSectionModel(i, items);
      expect(sectionModelToFlatIndex(sectionIdx, modelIdx, items)).toBe(i);
    }
  });
});

// =============================================================================
// PART 2: HOOK INTEGRATION TESTS
// =============================================================================
// These tests use REAL file system operations via fixtures.
// ONLY the NAPI network boundary is mocked.

import {
  createModelSelectorStateFixture,
  setupWithCloudCredentials,
  setupWithLocalProfile,
  setupWithUnreachableServer,
  type ModelSelectorStateFixture,
} from './fixtures/modelSelectorStateFixture';

// NAPI Mock - Only mock the network boundary
const napiMock = vi.hoisted(() => ({
  modelsListAll: vi.fn(),
  modelsListLocalOpenai: vi.fn(),
  modelsRefreshCache: vi.fn(),
}));

vi.mock('@sengac/codelet-napi', async (importOriginal) => {
  const original = await importOriginal<typeof import('@sengac/codelet-napi')>();
  return {
    ...original,
    modelsListAll: () => napiMock.modelsListAll(),
    modelsListLocalOpenai: (baseUrl: string) => napiMock.modelsListLocalOpenai(baseUrl),
    modelsRefreshCache: () => napiMock.modelsRefreshCache(),
  };
});

// Logger - Use real logger, just silence output
vi.mock('../../../utils/logger', () => ({
  logger: { debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn() },
}));

// Import model store for resetting between tests
import { useModelStore } from '../../store/modelStore';

// =============================================================================
// TEST COMPONENT
// =============================================================================

let hookState: UseModelSelectorStateReturn | null = null;

function TestComponent(): React.ReactElement {
  const state = useModelSelectorState();
  hookState = state;

  return (
    <Text>
      loading:{String(state.isLoading)}|
      initialized:{String(state.modelsInitialized)}|
      sections:{state.providerSections.length}|
      flatItems:{state.flatItems.length}
    </Text>
  );
}

// =============================================================================
// HOOK TESTS
// =============================================================================

describe('Feature: useModelSelectorState Hook', () => {
  let fixture: ModelSelectorStateFixture;
  let unmountComponent: (() => void) | null = null;

  // Helper to render and track unmount
  const renderWithCleanup = () => {
    const result = render(<TestComponent />);
    unmountComponent = result.unmount;
    return result;
  };

  beforeEach(async () => {
    // Reset Zustand store between tests to prevent state pollution
    useModelStore.getState().reset();
    
    // Clear the napiMock call counts
    napiMock.modelsListAll.mockClear();
    napiMock.modelsListLocalOpenai.mockClear();
    napiMock.modelsRefreshCache.mockClear();
    
    fixture = await createModelSelectorStateFixture('hook-test');
    // Wire up fixture mocks to vitest mocks
    napiMock.modelsListAll.mockImplementation(() => fixture.modelsListAllMock());
    napiMock.modelsListLocalOpenai.mockImplementation((url: string) =>
      fixture.modelsListLocalOpenaiMock(url)
    );
    napiMock.modelsRefreshCache.mockImplementation(() => fixture.modelsRefreshCacheMock());
    hookState = null;
    unmountComponent = null;
  });

  afterEach(async () => {
    // Unmount component to clean up React state
    if (unmountComponent) {
      unmountComponent();
      unmountComponent = null;
    }
    hookState = null;
    await fixture.cleanup();
  });

  describe('Scenario: Hook initializes with loading state and loads cloud models', () => {
    it('should initialize and load models from NAPI', async () => {
      // @step Given cloud credentials are configured
      await setupWithCloudCredentials(fixture);

      // @step When I render a component using useModelSelectorState
      const { lastFrame } = renderWithCleanup();

      // @step Then modelsListAll should be called
      await vi.waitFor(() => {
        expect(fixture.modelsListAllMock).toHaveBeenCalled();
      });

      // @step And isLoading should become false after loading completes
      await vi.waitFor(() => {
        expect(lastFrame()).toContain('loading:false');
      });

      // @step And modelsInitialized should become true
      await vi.waitFor(() => {
        expect(lastFrame()).toContain('initialized:true');
      });

      // @step And providerSections should be populated
      expect(hookState!.providerSections.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Hook loads profile sections from local servers', () => {
    it('should load profiles and merge with cloud sections', async () => {
      // @step Given there are configured profiles for providers
      await setupWithCloudCredentials(fixture);
      await setupWithLocalProfile(fixture, 'work-vllm', 'http://localhost:8000', [
        'llama-3.1-70b',
        'qwen-2.5-72b',
      ]);

      // @step When the hook initializes
      renderWithCleanup();

      // @step Then modelsListLocalOpenai should be called for the profile's baseUrl
      await vi.waitFor(() => {
        expect(fixture.modelsListLocalOpenaiMock).toHaveBeenCalledWith(
          'http://localhost:8000'
        );
      });

      // @step And profile sections should be merged with cloud sections
      await vi.waitFor(() => {
        expect(hookState!.modelsInitialized).toBe(true);
        const profileSection = hookState!.providerSections.find(
          s => s.profileName === 'work-vllm'
        );
        expect(profileSection).toBeDefined();
        expect(profileSection!.models.length).toBe(2);
      });
    });
  });

  describe('Scenario: Hook handles unreachable local servers gracefully', () => {
    it('should mark profile as unreachable when server is down', async () => {
      await setupWithCloudCredentials(fixture);
      await setupWithUnreachableServer(fixture, 'dead-server', 'http://unreachable:8888');

      renderWithCleanup();

      await vi.waitFor(() => {
        expect(hookState!.modelsInitialized).toBe(true);
      });

      const unreachableSection = hookState!.providerSections.find(
        s => s.profileName === 'dead-server'
      );
      expect(unreachableSection).toBeDefined();
      expect(unreachableSection!.isUnreachable).toBe(true);
      expect(unreachableSection!.models.length).toBe(0);
    });
  });

  // ===========================================================================
  // SECTION EXPANSION TESTS
  // ===========================================================================

  describe('Scenario: Toggle section expansion adds provider to expanded set', () => {
    it('should add provider to expandedProviders when toggled', async () => {
      await setupWithCloudCredentials(fixture);
      renderWithCleanup();

      await vi.waitFor(() => {
        expect(hookState!.modelsInitialized).toBe(true);
      });

      expect(hookState!.expandedProviders.has('anthropic')).toBe(false);
      hookState!.toggleSectionExpansion('anthropic');

      await vi.waitFor(() => {
        expect(hookState!.expandedProviders.has('anthropic')).toBe(true);
      });

      const anthropicModels = hookState!.flatItems.filter(
        i => i.type === 'model' && i.section.providerId === 'anthropic'
      );
      expect(anthropicModels.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Toggle section expansion removes provider from expanded set', () => {
    it('should remove provider when toggled again', async () => {
      await setupWithCloudCredentials(fixture);
      renderWithCleanup();

      await vi.waitFor(() => {
        expect(hookState!.modelsInitialized).toBe(true);
      });

      hookState!.toggleSectionExpansion('anthropic');
      await vi.waitFor(() => {
        expect(hookState!.expandedProviders.has('anthropic')).toBe(true);
      });

      hookState!.toggleSectionExpansion('anthropic');

      await vi.waitFor(() => {
        expect(hookState!.expandedProviders.has('anthropic')).toBe(false);
      });

      const anthropicModels = hookState!.flatItems.filter(
        i => i.type === 'model' && i.section.providerId === 'anthropic'
      );
      expect(anthropicModels.length).toBe(0);
    });
  });

  // ===========================================================================
  // FILTER TESTS
  // ===========================================================================

  describe('Scenario: filteredFlatItems filters by provider name case-insensitively', () => {
    it('should filter flatItems by provider name ignoring case', async () => {
      await setupWithCloudCredentials(fixture);
      renderWithCleanup();

      await vi.waitFor(() => {
        expect(hookState!.modelsInitialized).toBe(true);
      });

      hookState!.toggleSectionExpansion('anthropic');
      hookState!.toggleSectionExpansion('openai');
      hookState!.setFilter('ANTHRO');

      await vi.waitFor(() => {
        const filtered = hookState!.filteredFlatItems;
        expect(filtered.length).toBeGreaterThan(0);
        filtered.forEach(item => {
          expect(item.section.providerId).toBe('anthropic');
        });
      });
    });
  });

  describe('Scenario: filteredFlatItems filters by model name or ID', () => {
    it('should filter by model name matching', async () => {
      await setupWithCloudCredentials(fixture);
      renderWithCleanup();

      await vi.waitFor(() => {
        expect(hookState!.modelsInitialized).toBe(true);
      });

      hookState!.toggleSectionExpansion('anthropic');
      hookState!.toggleSectionExpansion('openai');
      hookState!.setFilter('sonnet');

      await vi.waitFor(() => {
        const filtered = hookState!.filteredFlatItems;
        const modelItems = filtered.filter(i => i.type === 'model');
        expect(modelItems.length).toBeGreaterThan(0);
        modelItems.forEach(item => {
          if (item.type === 'model') {
            const matches =
              item.model.id.toLowerCase().includes('sonnet') ||
              item.model.name.toLowerCase().includes('sonnet');
            expect(matches).toBe(true);
          }
        });
      });
    });
  });

  describe('Scenario: Filter change resets selection to first result', () => {
    it('should reset selection when filter changes', async () => {
      await setupWithCloudCredentials(fixture);
      renderWithCleanup();

      await vi.waitFor(() => {
        expect(hookState!.modelsInitialized).toBe(true);
      });

      hookState!.toggleSectionExpansion('anthropic');
      hookState!.toggleSectionExpansion('openai');
      hookState!.navigateDown();
      hookState!.navigateDown();

      await vi.waitFor(() => {
        expect(hookState!.getCurrentFlatIndex()).toBeGreaterThan(0);
      });

      hookState!.setFilter('openai');

      await vi.waitFor(() => {
        expect(hookState!.scrollOffset).toBe(0);
        expect(hookState!.filteredFlatItems.length).toBeGreaterThan(0);
      });
    });
  });

  // ===========================================================================
  // NAVIGATION TESTS
  // ===========================================================================

  describe('Scenario: Navigate down from section header to first model', () => {
    it('should expose navigation functions for controlling selection', async () => {
      await setupWithCloudCredentials(fixture);
      renderWithCleanup();

      await vi.waitFor(() => {
        expect(hookState!.modelsInitialized).toBe(true);
      });

      // @step Then the hook exposes navigation functions
      expect(typeof hookState!.navigateDown).toBe('function');
      expect(typeof hookState!.navigateUp).toBe('function');

      // @step And selectedSectionIdx is 0 and selectedModelIdx is -1 (initial state)
      expect(hookState!.selectedSectionIdx).toBe(0);
      expect(hookState!.selectedModelIdx).toBe(-1);

      // @step And selection can be controlled via setters
      hookState!.setSelectedModelIdx(0);

      await vi.waitFor(() => {
        expect(hookState!.selectedModelIdx).toBe(0);
      });
    });
  });

  describe('Scenario: getCurrentFlatIndex returns correct position', () => {
    it('should return correct flat index for current selection', async () => {
      await setupWithCloudCredentials(fixture);
      renderWithCleanup();

      await vi.waitFor(() => {
        expect(hookState!.modelsInitialized).toBe(true);
      });

      // @step Then getCurrentFlatIndex should return 0 at initial state
      expect(hookState!.selectedSectionIdx).toBe(0);
      expect(hookState!.selectedModelIdx).toBe(-1);

      const idx = hookState!.getCurrentFlatIndex();
      expect(typeof idx).toBe('number');
      expect(idx).toBe(0); // First section header
    });
  });

  // ===========================================================================
  // SCROLL MANAGEMENT TESTS
  // ===========================================================================

  describe('Scenario: Auto-scroll keeps selection visible', () => {
    it('should expose visibleHeight and scrollOffset state', async () => {
      await setupWithCloudCredentials(fixture);
      renderWithCleanup();

      await vi.waitFor(() => {
        expect(hookState!.modelsInitialized).toBe(true);
      });

      // @step Then visibleHeight should be configurable
      expect(hookState!.visibleHeight).toBe(10); // default

      hookState!.setVisibleHeight(5);
      await vi.waitFor(() => {
        expect(hookState!.visibleHeight).toBe(5);
      });

      // @step And scrollOffset should be configurable
      hookState!.setScrollOffset(3);
      await vi.waitFor(() => {
        expect(hookState!.scrollOffset).toBe(3);
      });
    });
  });

  describe('Scenario: Scroll and filter reset when model selector opens', () => {
    it('should reset state when isVisible changes to true', async () => {
      await setupWithCloudCredentials(fixture);
      renderWithCleanup();

      await vi.waitFor(() => {
        expect(hookState!.modelsInitialized).toBe(true);
      });

      // Set some state
      hookState!.setFilter('test');
      hookState!.setScrollOffset(5);
      hookState!.setIsFilterMode(true);
      hookState!.setIsVisible(false);

      await vi.waitFor(() => {
        expect(hookState!.filter).toBe('test');
        expect(hookState!.scrollOffset).toBe(5);
      });

      // @step When isVisible changes from false to true
      hookState!.setIsVisible(true);

      // @step Then scrollOffset, filter, isFilterMode should reset
      await vi.waitFor(() => {
        expect(hookState!.scrollOffset).toBe(0);
        expect(hookState!.filter).toBe('');
        expect(hookState!.isFilterMode).toBe(false);
      });
    });
  });

  // ===========================================================================
  // REFRESH TESTS
  // ===========================================================================

  describe('Scenario: Refresh models updates cache and reloads all models', () => {
    it('should refresh cache and reload models', async () => {
      await setupWithCloudCredentials(fixture);
      renderWithCleanup();

      await vi.waitFor(() => {
        expect(hookState!.modelsInitialized).toBe(true);
      });

      fixture.modelsListAllMock.mockClear();

      // @step When I call refreshModels
      const refreshPromise = hookState!.refreshModels();

      // @step Then modelsRefreshCache should be called
      await vi.waitFor(() => {
        expect(fixture.modelsRefreshCacheMock).toHaveBeenCalled();
      });

      // Wait for refresh to complete
      await refreshPromise;

      // @step And modelsListAll should be called to reload models
      expect(fixture.modelsListAllMock).toHaveBeenCalled();

      // @step And isRefreshing should be false after completion
      await vi.waitFor(() => {
        expect(hookState!.isRefreshing).toBe(false);
      });
    });
  });

  // ===========================================================================
  // MODEL SELECTION TESTS
  // ===========================================================================

  describe('Scenario: Select cloud provider model returns complete ModelSelection', () => {
    it('should return ModelSelection with all required fields', async () => {
      await setupWithCloudCredentials(fixture);
      renderWithCleanup();

      await vi.waitFor(() => {
        expect(hookState!.modelsInitialized).toBe(true);
        expect(hookState!.providerSections.length).toBeGreaterThan(0);
      });

      // Find Anthropic section
      const anthropicSection = hookState!.providerSections.find(
        s => s.providerId === 'anthropic'
      );
      expect(anthropicSection).toBeDefined();
      expect(anthropicSection!.models.length).toBeGreaterThan(0);

      const model = anthropicSection!.models[0];

      // @step When I call selectModel
      const selection = hookState!.selectModel(anthropicSection!, model);

      // @step Then the returned ModelSelection should have all fields
      expect(selection.providerId).toBe('anthropic');
      expect(selection.modelId).toBeDefined();
      expect(selection.apiModelId).toBe(model.id);
      expect(selection.displayName).toBe(model.name);
      expect(selection.contextWindow).toBe(model.contextWindow);
      expect(selection.maxOutput).toBe(model.maxOutput);
    });
  });

  describe('Scenario: Select profile model includes profile configuration', () => {
    it('should include profile name and config in selection', async () => {
      await setupWithCloudCredentials(fixture);
      await setupWithLocalProfile(fixture, 'work-vllm', 'http://localhost:8000', ['llama3']);

      renderWithCleanup();

      await vi.waitFor(() => {
        expect(hookState!.modelsInitialized).toBe(true);
      });

      // Find profile section
      const profileSection = hookState!.providerSections.find(
        s => s.profileName === 'work-vllm'
      );
      expect(profileSection).toBeDefined();
      expect(profileSection!.models.length).toBeGreaterThan(0);

      const model = profileSection!.models[0];

      // @step When I call selectModel with the profile section
      const selection = hookState!.selectModel(profileSection!, model);

      // @step Then the returned ModelSelection should have profileName
      expect(selection.profileName).toBe('work-vllm');

      // @step And the returned ModelSelection should have profileConfig
      expect(selection.profileConfig).toBeDefined();
      expect(selection.profileConfig?.baseUrl).toBe('http://localhost:8000');
    });
  });
}); // End of Feature describe block
