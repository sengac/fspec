/**
 * Provider Type Test Fixtures
 *
 * TUI-076: Fixtures for testing provider type consolidation.
 *
 * These fixtures provide:
 * - Factory functions for creating test objects
 * - Type-safe builders for complex nested types
 * - Reusable test data for provider-related components
 *
 * SOLID: Single Responsibility - Only handles provider type test data
 * DRY: Reusable across AgentView, ModelSelector, and integration tests
 * COMPOSABLE: Builders can be chained and customized
 */

import type { NapiModelInfo } from '@sengac/codelet-napi';
import type {
  ProviderSection,
  ProviderModel,
  ModelSelection,
  ModelSelectorItem,
} from '../tui/types/provider';

// ============================================================================
// NAPIMODELIFO FIXTURES
// ============================================================================

/**
 * Creates a minimal NapiModelInfo for testing.
 */
export function createTestModelInfo(
  overrides: Partial<NapiModelInfo> = {}
): NapiModelInfo {
  return {
    id: 'test-model',
    name: 'Test Model',
    description: 'A test model',
    contextWindow: 128000,
    maxOutput: 8192,
    trainingCutoff: '2024-01',
    hasVision: false,
    capabilities: {
      reasoning: false,
      functionCalling: true,
      json: true,
    },
    pricing: {
      inputPerMillion: 1.0,
      outputPerMillion: 2.0,
    },
    ...overrides,
  };
}

/**
 * Creates a Claude-like model for testing.
 */
export function createClaudeModel(
  overrides: Partial<NapiModelInfo> = {}
): NapiModelInfo {
  return createTestModelInfo({
    id: 'claude-sonnet-4',
    name: 'Claude Sonnet 4',
    description: 'Anthropic Claude Sonnet 4',
    contextWindow: 200000,
    maxOutput: 16384,
    hasVision: true,
    capabilities: {
      reasoning: true,
      functionCalling: true,
      json: true,
    },
    ...overrides,
  });
}

/**
 * Creates a GPT-like model for testing.
 */
export function createGptModel(
  overrides: Partial<NapiModelInfo> = {}
): NapiModelInfo {
  return createTestModelInfo({
    id: 'gpt-4.1',
    name: 'GPT-4.1',
    description: 'OpenAI GPT-4.1',
    contextWindow: 128000,
    maxOutput: 16384,
    hasVision: true,
    capabilities: {
      reasoning: false,
      functionCalling: true,
      json: true,
    },
    ...overrides,
  });
}

/**
 * Creates a local/Ollama-like model for testing.
 */
export function createLocalModel(
  overrides: Partial<NapiModelInfo> = {}
): NapiModelInfo {
  return createTestModelInfo({
    id: 'llama3',
    name: 'Llama 3',
    description: 'Meta Llama 3',
    contextWindow: 8192,
    maxOutput: 4096,
    hasVision: false,
    capabilities: {
      reasoning: false,
      functionCalling: false,
      json: true,
    },
    ...overrides,
  });
}

// ============================================================================
// PROVIDER SECTION FIXTURES
// ============================================================================

/**
 * Creates a ProviderSection for testing.
 */
export function createTestProviderSection(
  overrides: Partial<ProviderSection> = {}
): ProviderSection {
  return {
    providerId: 'test-provider',
    providerName: 'Test Provider',
    internalName: 'test-provider',
    models: [createTestModelInfo()],
    hasCredentials: true,
    ...overrides,
  };
}

/**
 * Creates an Anthropic provider section for testing.
 */
export function createAnthropicSection(
  overrides: Partial<ProviderSection> = {}
): ProviderSection {
  return createTestProviderSection({
    providerId: 'anthropic',
    providerName: 'Anthropic',
    internalName: 'anthropic',
    models: [
      createClaudeModel({ id: 'claude-sonnet-4', name: 'Claude Sonnet 4' }),
      createClaudeModel({
        id: 'claude-opus-4',
        name: 'Claude Opus 4',
        capabilities: { reasoning: true, functionCalling: true, json: true },
      }),
    ],
    hasCredentials: true,
    ...overrides,
  });
}

/**
 * Creates an OpenAI provider section for testing.
 */
export function createOpenAiSection(
  overrides: Partial<ProviderSection> = {}
): ProviderSection {
  return createTestProviderSection({
    providerId: 'openai',
    providerName: 'OpenAI',
    internalName: 'openai',
    models: [
      createGptModel({ id: 'gpt-4.1', name: 'GPT-4.1' }),
      createGptModel({ id: 'gpt-4.1-mini', name: 'GPT-4.1 Mini' }),
    ],
    hasCredentials: true,
    ...overrides,
  });
}

/**
 * Creates a local profile section for testing.
 */
export function createLocalProfileSection(
  profileName: string,
  overrides: Partial<ProviderSection> = {}
): ProviderSection {
  return createTestProviderSection({
    providerId: 'openai',
    providerName: `Local: ${profileName}`,
    internalName: 'openai',
    models: [
      createLocalModel({ id: 'llama3', name: 'Llama 3' }),
      createLocalModel({ id: 'codellama', name: 'Code Llama' }),
    ],
    hasCredentials: true,
    profileName,
    profileConfig: {
      baseUrl: 'http://localhost:11434',
      apiKey: 'local-key',
    },
    ...overrides,
  });
}

/**
 * Creates an unreachable local profile section for error testing.
 */
export function createUnreachableProfileSection(
  profileName: string
): ProviderSection {
  return createTestProviderSection({
    providerId: 'openai',
    providerName: `Local: ${profileName}`,
    internalName: 'openai',
    models: [],
    hasCredentials: true,
    profileName,
    profileConfig: {
      baseUrl: 'http://unreachable:8888',
      apiKey: 'key',
    },
    isUnreachable: true,
  });
}

// ============================================================================
// MODEL SELECTION FIXTURES
// ============================================================================

/**
 * Creates a ModelSelection for testing.
 */
export function createTestModelSelection(
  overrides: Partial<ModelSelection> = {}
): ModelSelection {
  return {
    providerId: 'test-provider',
    modelId: 'test-model',
    apiModelId: 'test-model-20240101',
    displayName: 'Test Model',
    reasoning: false,
    hasVision: false,
    contextWindow: 128000,
    maxOutput: 8192,
    ...overrides,
  };
}

/**
 * Creates a ModelSelection from a ProviderSection and model.
 * This simulates the actual selection flow in AgentView.
 */
export function createModelSelectionFromSection(
  section: ProviderSection,
  model: NapiModelInfo
): ModelSelection {
  return {
    providerId: section.providerId,
    modelId: model.id,
    apiModelId: model.id,
    displayName: model.name,
    reasoning: model.capabilities?.reasoning ?? false,
    hasVision: model.hasVision ?? false,
    contextWindow: model.contextWindow ?? 128000,
    maxOutput: model.maxOutput ?? 8192,
    profileName: section.profileName,
    profileConfig: section.profileConfig,
  };
}

/**
 * Creates a Claude ModelSelection for testing.
 */
export function createClaudeSelection(
  overrides: Partial<ModelSelection> = {}
): ModelSelection {
  return createTestModelSelection({
    providerId: 'anthropic',
    modelId: 'claude-sonnet-4',
    apiModelId: 'claude-sonnet-4-20250514',
    displayName: 'Claude Sonnet 4',
    reasoning: true,
    hasVision: true,
    contextWindow: 200000,
    maxOutput: 16384,
    ...overrides,
  });
}

/**
 * Creates a local profile ModelSelection for testing.
 */
export function createLocalProfileSelection(
  profileName: string,
  modelId: string = 'llama3',
  overrides: Partial<ModelSelection> = {}
): ModelSelection {
  return createTestModelSelection({
    providerId: 'openai',
    modelId,
    apiModelId: modelId,
    displayName: `Llama 3 (${profileName})`,
    reasoning: false,
    hasVision: false,
    contextWindow: 8192,
    maxOutput: 4096,
    profileName,
    profileConfig: {
      baseUrl: 'http://localhost:11434',
      apiKey: 'local-key',
    },
    ...overrides,
  });
}

// ============================================================================
// MODEL SELECTOR ITEM FIXTURES
// ============================================================================

/**
 * Creates a section-type ModelSelectorItem.
 */
export function createSectionItem(
  section: ProviderSection,
  sectionIdx: number,
  isExpanded: boolean = true
): ModelSelectorItem {
  return {
    type: 'section',
    sectionIdx,
    section,
    isExpanded,
  };
}

/**
 * Creates a model-type ModelSelectorItem.
 */
export function createModelItem(
  section: ProviderSection,
  model: NapiModelInfo,
  sectionIdx: number,
  modelIdx: number
): ModelSelectorItem {
  return {
    type: 'model',
    sectionIdx,
    modelIdx,
    section,
    model,
  };
}

/**
 * Builds a flat list of ModelSelectorItems from sections.
 * This replicates the buildFlatModelList function in AgentView.
 */
export function buildTestFlatModelList(
  sections: ProviderSection[],
  expandedSections: Set<number>
): ModelSelectorItem[] {
  const items: ModelSelectorItem[] = [];

  for (let sectionIdx = 0; sectionIdx < sections.length; sectionIdx++) {
    const section = sections[sectionIdx];
    const isExpanded = expandedSections.has(sectionIdx);

    items.push(createSectionItem(section, sectionIdx, isExpanded));

    if (isExpanded) {
      for (let modelIdx = 0; modelIdx < section.models.length; modelIdx++) {
        items.push(
          createModelItem(
            section,
            section.models[modelIdx],
            sectionIdx,
            modelIdx
          )
        );
      }
    }
  }

  return items;
}

// ============================================================================
// COMPOSITE FIXTURES
// ============================================================================

/**
 * Creates a complete test scenario with multiple providers.
 */
export function createMultiProviderScenario(): {
  sections: ProviderSection[];
  expandedSections: Set<number>;
  flatList: ModelSelectorItem[];
  defaultSelection: ModelSelection;
} {
  const sections = [
    createAnthropicSection(),
    createOpenAiSection(),
    createLocalProfileSection('home-ollama'),
  ];

  const expandedSections = new Set([0, 1, 2]);
  const flatList = buildTestFlatModelList(sections, expandedSections);

  const defaultSection = sections[0];
  const defaultModel = defaultSection.models[0];
  const defaultSelection = createModelSelectionFromSection(
    defaultSection,
    defaultModel
  );

  return {
    sections,
    expandedSections,
    flatList,
    defaultSelection,
  };
}

/**
 * Creates a scenario with collapsed sections for navigation testing.
 */
export function createCollapsedSectionsScenario(): {
  sections: ProviderSection[];
  expandedSections: Set<number>;
  flatList: ModelSelectorItem[];
} {
  const sections = [
    createAnthropicSection(),
    createOpenAiSection(),
    createLocalProfileSection('work-vllm'),
  ];

  // Only first section expanded
  const expandedSections = new Set([0]);
  const flatList = buildTestFlatModelList(sections, expandedSections);

  return {
    sections,
    expandedSections,
    flatList,
  };
}

/**
 * Creates a scenario with an unreachable local server.
 */
export function createUnreachableServerScenario(): {
  sections: ProviderSection[];
  flatList: ModelSelectorItem[];
} {
  const sections = [
    createAnthropicSection(),
    createUnreachableProfileSection('dead-server'),
  ];

  const expandedSections = new Set([0, 1]);
  const flatList = buildTestFlatModelList(sections, expandedSections);

  return {
    sections,
    flatList,
  };
}
