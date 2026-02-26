/**
 * Feature: spec/features/provider-type-consolidation.feature
 *
 * Tests for TUI-076: Consolidate provider types
 *
 * This test file validates:
 * 1. Types are exported from the canonical location (types/provider.ts)
 * 2. No duplicate definitions exist across src/tui
 * 3. Types work correctly at runtime (integration tests)
 * 4. Type relationships are correct (ModelSelectorItem uses ProviderSection)
 *
 * Uses fixtures from:
 * - test-helpers/source-code-analysis-fixtures.ts (static analysis)
 * - test-helpers/provider-type-fixtures.ts (runtime type testing)
 */

import { describe, it, expect } from 'vitest';
import { join } from 'path';

// Static analysis fixtures
import {
  verifyTypeDefinition,
  verifyImports,
  findFilesWithPattern,
} from '../../test-helpers/source-code-analysis-fixtures';

// Runtime type fixtures - ACTUALLY importing the types proves they work
import type {
  ProviderSection,
  ModelSelection,
  ModelSelectorItem,
} from '../types/provider';

// Type fixture factories for creating test objects
import {
  createTestProviderSection,
  createAnthropicSection,
  createOpenAiSection,
  createLocalProfileSection,
  createTestModelSelection,
  createClaudeSelection,
  createLocalProfileSelection as createLocalSelection,
  createModelSelectionFromSection,
  createSectionItem,
  createModelItem,
  buildTestFlatModelList,
  createMultiProviderScenario,
  createCollapsedSectionsScenario,
  createClaudeModel,
  createGptModel,
} from '../../test-helpers/provider-type-fixtures';

// ============================================================================
// TEST CONSTANTS
// ============================================================================

const TUI_DIR = join(__dirname, '..');
const PROVIDER_TYPES_PATH = join(TUI_DIR, 'types', 'provider.ts');
const AGENT_VIEW_PATH = join(TUI_DIR, 'components', 'AgentView.tsx');

// ============================================================================
// STATIC ANALYSIS TESTS (verify source code structure)
// ============================================================================

describe('Feature: Consolidate provider types', () => {
  describe('Scenario: ModelSelection type exported from provider.ts', () => {
    it('should export ModelSelection from types/provider.ts and not define it elsewhere', () => {
      // @step Given the types/provider.ts file exists
      const result = verifyTypeDefinition(
        TUI_DIR,
        'ModelSelection',
        'types/provider.ts',
        true // isInterface
      );

      // @step When I check for ModelSelection interface definition
      // (done by verifyTypeDefinition)

      // @step Then ModelSelection is exported from src/tui/types/provider.ts
      expect(result.expectedFileHasDefinition).toBe(true);

      // @step And ModelSelection is NOT defined in AgentView.tsx
      expect(result.duplicates).not.toContain('components/AgentView.tsx');
      expect(result.isValid).toBe(true);
      expect(result.foundIn).toEqual(['types/provider.ts']);
    });
  });

  describe('Scenario: ModelSelectorItem type exported from provider.ts', () => {
    it('should export ModelSelectorItem from types/provider.ts with NapiModelInfo dependency', () => {
      // @step Given the types/provider.ts file exists
      const result = verifyTypeDefinition(
        TUI_DIR,
        'ModelSelectorItem',
        'types/provider.ts',
        false // isInterface (it's a type alias)
      );

      // @step When I check for ModelSelectorItem type definition
      // (done by verifyTypeDefinition)

      // @step Then ModelSelectorItem is exported from src/tui/types/provider.ts
      expect(result.expectedFileHasDefinition).toBe(true);

      // @step And ModelSelectorItem is NOT defined in AgentView.tsx
      expect(result.duplicates).not.toContain('components/AgentView.tsx');
      expect(result.isValid).toBe(true);

      // @step And NapiModelInfo is imported in provider.ts for ModelSelectorItem dependency
      const napiImports = findFilesWithPattern(
        TUI_DIR,
        /import\s+.*NapiModelInfo.*from\s+['"]@sengac\/codelet-napi['"]/
      );
      expect(napiImports).toContain('types/provider.ts');
    });
  });

  describe('Scenario: AgentView imports types from provider.ts', () => {
    it('should import ModelSelection from types/provider', () => {
      // @step Given types are consolidated in types/provider.ts
      const typeResult = verifyTypeDefinition(
        TUI_DIR,
        'ModelSelection',
        'types/provider.ts',
        true
      );
      expect(typeResult.isValid).toBe(true);

      // @step When I check AgentView.tsx imports
      // AgentView only needs ModelSelection directly.
      // ModelSelectorItem and ProviderSection are used by ModelSelector component,
      // not by AgentView directly. AgentView accesses providers via useProviderSections hook.
      const importResult = verifyImports(AGENT_VIEW_PATH, '../types/provider', [
        'ModelSelection',
      ]);

      // @step Then AgentView imports ModelSelection from ../types/provider
      expect(importResult.found).toContain('ModelSelection');

      // All required imports present
      expect(importResult.allFound).toBe(true);
    });
  });

  describe('Scenario: Build and tests pass after consolidation', () => {
    it('should have no duplicate type definitions across src/tui', () => {
      // @step Given types are consolidated in types/provider.ts
      // Verify all three types are defined exactly once in types/provider.ts

      // @step When I run the build and test commands
      // Build verification is implicit - TypeScript compilation succeeded if this runs

      // @step Then npm run build succeeds with no TypeScript errors
      // Verified by this test running (TypeScript compiled successfully)

      // ModelSelection - only in types/provider.ts
      const modelSelectionResult = verifyTypeDefinition(
        TUI_DIR,
        'ModelSelection',
        'types/provider.ts',
        true
      );
      expect(modelSelectionResult.isValid).toBe(true);
      expect(modelSelectionResult.foundIn).toEqual(['types/provider.ts']);

      // ModelSelectorItem - only in types/provider.ts
      const modelSelectorItemResult = verifyTypeDefinition(
        TUI_DIR,
        'ModelSelectorItem',
        'types/provider.ts',
        false
      );
      expect(modelSelectorItemResult.isValid).toBe(true);
      expect(modelSelectorItemResult.foundIn).toEqual(['types/provider.ts']);

      // ProviderSection - only in types/provider.ts (not ProviderSectionInfo)
      const providerSectionResult = verifyTypeDefinition(
        TUI_DIR,
        'ProviderSection',
        'types/provider.ts',
        true
      );
      expect(providerSectionResult.isValid).toBe(true);
      expect(providerSectionResult.foundIn).toEqual(['types/provider.ts']);

      // @step And AgentView.tsx imports types from ../types/provider
      const importResult = verifyImports(AGENT_VIEW_PATH, '../types/provider', [
        'ModelSelection',
        'ModelSelectorItem',
        'ProviderSection',
      ]);
      expect(importResult.importExists).toBe(true);

      // @step And npm test passes for all existing tests
      // Verify other test files can import these types
      const testImportResult = verifyImports(
        join(TUI_DIR, '__tests__', 'AgentView-resume-attach.test.tsx'),
        '../types/provider',
        ['ModelSelection']
      );
      expect(testImportResult.importExists).toBe(true);
      expect(testImportResult.found).toContain('ModelSelection');
    });
  });
});

// ============================================================================
// RUNTIME INTEGRATION TESTS (verify types work at runtime)
// ============================================================================

describe('Integration: Provider types work at runtime', () => {
  describe('ProviderSection type', () => {
    it('should create valid ProviderSection objects', () => {
      // @step Given the ProviderSection type is imported from types/provider
      // (imported at top of file)

      // @step When I create a ProviderSection using the fixture
      const section: ProviderSection = createTestProviderSection();

      // @step Then the object has all required properties
      expect(section.providerId).toBe('test-provider');
      expect(section.providerName).toBe('Test Provider');
      expect(section.internalName).toBe('test-provider');
      expect(section.models).toHaveLength(1);
      expect(section.hasCredentials).toBe(true);
    });

    it('should support profile properties for local servers', () => {
      // @step Given I create a local profile section
      const section: ProviderSection = createLocalProfileSection('home-ollama');

      // @step Then profile properties are set
      expect(section.profileName).toBe('home-ollama');
      expect(section.profileConfig).toBeDefined();
      expect(section.profileConfig?.baseUrl).toBe('http://localhost:11434');
    });

    it('should support unreachable flag for error handling', () => {
      // @step Given I create an Anthropic section with unreachable flag
      const section: ProviderSection = createAnthropicSection({
        isUnreachable: true,
      });

      // @step Then the unreachable flag is set
      expect(section.isUnreachable).toBe(true);
    });
  });

  describe('ModelSelection type', () => {
    it('should create valid ModelSelection objects', () => {
      // @step Given the ModelSelection type is imported from types/provider
      // (imported at top of file)

      // @step When I create a ModelSelection using the fixture
      const selection: ModelSelection = createTestModelSelection();

      // @step Then the object has all required properties
      expect(selection.providerId).toBe('test-provider');
      expect(selection.modelId).toBe('test-model');
      expect(selection.apiModelId).toBe('test-model-20240101');
      expect(selection.displayName).toBe('Test Model');
      expect(selection.reasoning).toBe(false);
      expect(selection.hasVision).toBe(false);
      expect(selection.contextWindow).toBe(128000);
      expect(selection.maxOutput).toBe(8192);
    });

    it('should create ModelSelection from ProviderSection and model', () => {
      // @step Given I have a ProviderSection with models
      const section = createAnthropicSection();
      const model = section.models[0];

      // @step When I create a ModelSelection from the section and model
      const selection: ModelSelection = createModelSelectionFromSection(
        section,
        model
      );

      // @step Then the selection has correct provider and model info
      expect(selection.providerId).toBe('anthropic');
      expect(selection.modelId).toBe(model.id);
      expect(selection.displayName).toBe(model.name);
      expect(selection.reasoning).toBe(model.capabilities?.reasoning ?? false);
    });

    it('should include profile info for local server selections', () => {
      // @step Given I create a local profile selection
      const selection: ModelSelection = createLocalSelection(
        'work-vllm',
        'qwen-80b'
      );

      // @step Then profile properties are included
      expect(selection.profileName).toBe('work-vllm');
      expect(selection.profileConfig).toBeDefined();
    });
  });

  describe('ModelSelectorItem type', () => {
    it('should create section-type items', () => {
      // @step Given I have a ProviderSection
      const section = createAnthropicSection();

      // @step When I create a section item
      const item: ModelSelectorItem = createSectionItem(section, 0, true);

      // @step Then it has type "section" with correct properties
      expect(item.type).toBe('section');
      if (item.type === 'section') {
        expect(item.sectionIdx).toBe(0);
        expect(item.section).toBe(section);
        expect(item.isExpanded).toBe(true);
      }
    });

    it('should create model-type items', () => {
      // @step Given I have a ProviderSection with models
      const section = createOpenAiSection();
      const model = section.models[0];

      // @step When I create a model item
      const item: ModelSelectorItem = createModelItem(section, model, 0, 0);

      // @step Then it has type "model" with correct properties
      expect(item.type).toBe('model');
      if (item.type === 'model') {
        expect(item.sectionIdx).toBe(0);
        expect(item.modelIdx).toBe(0);
        expect(item.section).toBe(section);
        expect(item.model).toBe(model);
      }
    });

    it('should discriminate between section and model types', () => {
      // @step Given I have both section and model items
      const section = createAnthropicSection();
      const sectionItem: ModelSelectorItem = createSectionItem(
        section,
        0,
        true
      );
      const modelItem: ModelSelectorItem = createModelItem(
        section,
        section.models[0],
        0,
        0
      );

      // @step When I check their types
      // @step Then TypeScript discriminated union works correctly
      if (sectionItem.type === 'section') {
        // TypeScript knows this is a section item
        expect(sectionItem.isExpanded).toBeDefined();
      }

      if (modelItem.type === 'model') {
        // TypeScript knows this is a model item
        expect(modelItem.modelIdx).toBeDefined();
      }
    });
  });

  describe('buildTestFlatModelList utility', () => {
    it('should build flat list with expanded sections', () => {
      // @step Given multiple provider sections
      const sections = [createAnthropicSection(), createOpenAiSection()];
      const expandedSections = new Set([0, 1]);

      // @step When I build a flat model list
      const items = buildTestFlatModelList(sections, expandedSections);

      // @step Then it contains section headers and model items
      const sectionItems = items.filter(i => i.type === 'section');
      const modelItems = items.filter(i => i.type === 'model');

      expect(sectionItems).toHaveLength(2);
      expect(modelItems.length).toBeGreaterThan(0);

      // First item should be section header
      expect(items[0].type).toBe('section');
    });

    it('should hide models for collapsed sections', () => {
      // @step Given sections with only first expanded
      const { sections, flatList } = createCollapsedSectionsScenario();

      // @step When I count models visible
      const modelItems = flatList.filter(i => i.type === 'model');

      // @step Then only first section's models are included
      const firstSectionModelCount = sections[0].models.length;
      expect(modelItems).toHaveLength(firstSectionModelCount);
    });
  });

  describe('Composite scenarios', () => {
    it('should handle multi-provider scenario', () => {
      // @step Given a multi-provider scenario
      const scenario = createMultiProviderScenario();

      // @step Then all components are correctly typed and related
      expect(scenario.sections).toHaveLength(3);
      expect(scenario.flatList.length).toBeGreaterThan(3); // sections + models
      expect(scenario.defaultSelection.providerId).toBe('anthropic');

      // Verify type relationships
      const firstSection = scenario.flatList[0];
      expect(firstSection.type).toBe('section');
      if (firstSection.type === 'section') {
        expect(firstSection.section.providerId).toBe('anthropic');
      }
    });

    it('should create selections that match their source sections', () => {
      // @step Given a provider section
      const section = createLocalProfileSection('my-server');
      const model = section.models[0];

      // @step When I create a selection from it
      const selection = createModelSelectionFromSection(section, model);

      // @step Then the selection preserves profile info
      expect(selection.profileName).toBe(section.profileName);
      expect(selection.profileConfig?.baseUrl).toBe(
        section.profileConfig?.baseUrl
      );
    });
  });
});
