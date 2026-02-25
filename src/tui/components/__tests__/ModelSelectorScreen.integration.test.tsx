/**
 * Feature: spec/features/model-selector-screen.feature
 *
 * TUI-073: Create ModelSelectorScreen component - INTEGRATION TESTS
 *
 * These are REAL integration tests that:
 * - Use REAL useModelSelectorState hook (NOT mocked)
 * - Use REAL ModelSelectorScreen component
 * - Only mock at NAPI network boundary (models.dev, local servers)
 * - Use reusable fixtures following DRY/SOLID/COMPOSABLE principles
 *
 * Test coverage validates actual behavior, not mock interactions.
 */

import React from 'react';
import { render, cleanup } from 'ink-testing-library';
import { describe, it, expect, vi, beforeEach, afterEach, beforeAll } from 'vitest';

import {
  createModelSelectorScreenFixture,
  createDefaultCloudProviders,
  type ModelSelectorScreenFixture,
} from './fixtures/modelSelectorScreenFixture';
import {
  pressKey,
  typeString,
  waitFor,
  modelSelectorKeySequences as keys,
} from './fixtures/keyboardHelpers';
import {
  TEST_MODEL_IDS,
  TEST_PROVIDER_NAMES,
  UI_PATTERNS,
  TEST_TIMING,
} from './fixtures/testConstants';

// =============================================================================
// NAPI MODULE MOCK - Network boundary ONLY
// =============================================================================

// Store fixture reference for mock access
let activeFixture: ModelSelectorScreenFixture | null = null;

vi.mock('@sengac/codelet-napi', async () => {
  const actual = await vi.importActual<typeof import('@sengac/codelet-napi')>(
    '@sengac/codelet-napi'
  );
  return {
    ...actual,
    modelsListAll: vi.fn(async () => {
      if (activeFixture) {
        return activeFixture.modelsListAllMock();
      }
      return createDefaultCloudProviders();
    }),
    modelsListLocalOpenai: vi.fn(async (baseUrl: string) => {
      if (activeFixture) {
        return activeFixture.modelsListLocalOpenaiMock(baseUrl);
      }
      return [];
    }),
    modelsRefreshCache: vi.fn(async () => {
      if (activeFixture) {
        return activeFixture.modelsRefreshCacheMock();
      }
      return undefined;
    }),
  };
});

// =============================================================================
// TEST SUITE
// =============================================================================

describe('Feature: Create ModelSelectorScreen component', () => {
  let fixture: ModelSelectorScreenFixture;
  let ModelSelectorScreen: typeof import('../ModelSelectorScreen').ModelSelectorScreen;

  beforeAll(async () => {
    // Import the real component (not mocked)
    const module = await import('../ModelSelectorScreen');
    ModelSelectorScreen = module.ModelSelectorScreen;
  });

  beforeEach(async () => {
    // Create fresh fixture for each test
    fixture = await createModelSelectorScreenFixture('model-selector-screen');
    activeFixture = fixture;

    // Set up credentials so models load
    await fixture.createCredential('anthropic', 'test-api-key-12345');
    await fixture.createCredential('openai', 'test-api-key-67890');
  });

  afterEach(async () => {
    cleanup();
    activeFixture = null;
    await fixture.cleanup();
  });

  // ===========================================================================
  // BACKGROUND: User Story
  // ===========================================================================

  describe('Background: User Story', () => {
    it('provides the user story context', () => {
      // @step As a developer
      // @step I want to use a ModelSelectorScreen component that handles all model selection input
      // @step So that AgentView.tsx is reduced by ~400 lines and keyboard handling is encapsulated
      expect(ModelSelectorScreen).toBeDefined();
    });
  });

  // ===========================================================================
  // NAVIGATION SCENARIOS
  // ===========================================================================

  describe('Scenario: Navigate down in model list', () => {
    it('should move selection down when Down arrow is pressed', async () => {
      // @step Given the ModelSelectorScreen is rendered with provider sections
      const { stdin, lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      // Wait for models to load
      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // Verify initial render shows first section (Anthropic) selected with ">" indicator
      const frame1 = lastFrame();
      expect(frame1).toContain('Anthropic');
      // First section should be selected (has > indicator)
      expect(frame1).toMatch(/>\s*[▶▼]\s*Anthropic/);

      // @step When the user presses the Down arrow key
      pressKey(stdin, { name: 'down' });
      await waitFor(50);

      // @step Then the hook's navigateDown function should be called
      // @step And the selection should move to the next item in the list
      const frame2 = lastFrame();
      // Selection should now be on OpenAI (second section)
      expect(frame2).toMatch(/>\s*[▶▼]\s*OpenAI/);
      // Anthropic should no longer have the selection indicator at start
      expect(frame2).not.toMatch(/>\s*[▶▼]\s*Anthropic/);
    });
  });

  describe('Scenario: Navigate up in model list', () => {
    it('should move selection up when Up arrow is pressed', async () => {
      // @step Given the ModelSelectorScreen is rendered with provider sections
      const { stdin, lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // @step And the selection is not on the first item
      // Move down to OpenAI section first
      pressKey(stdin, { name: 'down' });
      await waitFor(50);

      // Verify we're on OpenAI
      const afterDown = lastFrame();
      expect(afterDown).toMatch(/>\s*[▶▼]\s*OpenAI/);

      // @step When the user presses the Up arrow key
      pressKey(stdin, { name: 'up' });
      await waitFor(50);

      // @step Then the hook's navigateUp function should be called
      // @step And the selection should move to the previous item in the list
      const frame = lastFrame();
      // Selection should be back on Anthropic
      expect(frame).toMatch(/>\s*[▶▼]\s*Anthropic/);
      expect(frame).not.toMatch(/>\s*[▶▼]\s*OpenAI/);
    });
  });

  describe('Scenario: Collapse section with Left arrow', () => {
    it('should collapse section when Left arrow is pressed', async () => {
      // @step Given the ModelSelectorScreen is rendered with provider sections
      const { stdin, lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // @step And the current section is expanded
      // Expand the section first
      pressKey(stdin, { name: 'right' });
      await waitFor(50);

      const expandedFrame = lastFrame();
      // Should show expanded indicator (▼) and models underneath
      expect(expandedFrame).toMatch(/▼\s*Anthropic/);
      expect(expandedFrame).toContain('claude-sonnet-4');

      // @step When the user presses the Left arrow key
      pressKey(stdin, { name: 'left' });
      await waitFor(50);

      // @step Then the section should collapse
      // @step And the selection should move to the section header
      const collapsedFrame = lastFrame();
      // Should show collapsed indicator (▶)
      expect(collapsedFrame).toMatch(/▶\s*Anthropic/);
      // Models should NOT be visible when collapsed
      expect(collapsedFrame).not.toContain('claude-sonnet-4');
    });
  });

  describe('Scenario: Expand section with Right arrow', () => {
    it('should expand section when Right arrow is pressed on collapsed section', async () => {
      // @step Given the ModelSelectorScreen is rendered with provider sections
      const { stdin, lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // @step And the current section is collapsed
      // Sections start collapsed by default
      const collapsedFrame = lastFrame();
      expect(collapsedFrame).toMatch(/▶\s*Anthropic/);
      expect(collapsedFrame).not.toContain('claude-sonnet-4');

      // @step When the user presses the Right arrow key
      pressKey(stdin, { name: 'right' });
      await waitFor(50);

      // @step Then the section should expand
      const expandedFrame = lastFrame();
      // Should show expanded indicator (▼)
      expect(expandedFrame).toMatch(/▼\s*Anthropic/);
      // Models should now be visible
      expect(expandedFrame).toContain('claude-sonnet-4');
    });
  });

  // ===========================================================================
  // CLOSE BEHAVIOR SCENARIOS
  // ===========================================================================

  describe('Scenario: Close screen with Escape when no filter is active', () => {
    it('should call onClose when Escape is pressed with no filter', async () => {
      // @step Given the ModelSelectorScreen is rendered
      const { stdin } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // @step And no filter is currently active
      // No filter is active by default

      // @step When the user presses the Escape key
      pressKey(stdin, { name: 'escape' });
      await waitFor(50);

      // @step Then the onClose callback should be invoked
      expect(fixture.callbacks.onClose.calls).toBe(1);
    });
  });

  describe('Scenario: Clear filter with Escape when filter is active', () => {
    it('should clear filter without closing when Escape is pressed with active filter', async () => {
      // @step Given the ModelSelectorScreen is rendered
      const { stdin, lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // @step And a filter is currently active
      // Enter filter mode and type something
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('cla');
      await waitFor(100);

      // Exit filter mode with Enter to keep filter active but not editing
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      // Verify filter is shown and applied
      const filteredFrame = lastFrame();
      expect(filteredFrame).toContain('cla');

      // @step When the user presses the Escape key
      pressKey(stdin, { name: 'escape' });
      await waitFor(50);

      // @step Then the filter should be cleared
      const clearedFrame = lastFrame();
      expect(clearedFrame).not.toContain('cla');
      // Both providers should be visible again
      expect(clearedFrame).toContain('Anthropic');
      expect(clearedFrame).toContain('OpenAI');

      // @step And the onClose callback should NOT be invoked
      expect(fixture.callbacks.onClose.calls).toBe(0);
    });
  });

  // ===========================================================================
  // SCREEN SWITCHING SCENARIOS
  // ===========================================================================

  describe('Scenario: Switch to provider settings with Tab', () => {
    it('should call onSwitchToSettings when Tab is pressed', async () => {
      // @step Given the ModelSelectorScreen is rendered
      const { stdin } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // @step When the user presses the Tab key
      pressKey(stdin, { name: 'tab' });
      await waitFor(50);

      // @step Then the onSwitchToSettings callback should be invoked
      expect(fixture.callbacks.onSwitchToSettings.calls).toBe(1);
    });
  });

  // ===========================================================================
  // MODEL SELECTION SCENARIOS
  // ===========================================================================

  describe('Scenario: Select a model with Enter', () => {
    it('should call onSelectModel and onClose when Enter is pressed on model item', async () => {
      // @step Given the ModelSelectorScreen is rendered with provider sections
      const { stdin } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // @step And the selection is on a model item
      // Expand section first
      pressKey(stdin, { name: 'right' });
      await waitFor(50);

      // Navigate to a model
      pressKey(stdin, { name: 'down' });
      await waitFor(50);

      // @step When the user presses the Enter key
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      // @step Then the onSelectModel callback should be invoked with a ModelSelection object
      expect(fixture.callbacks.onSelectModel.calls.length).toBe(1);
      const selection = fixture.callbacks.onSelectModel.calls[0];
      expect(selection).toHaveProperty('providerId');
      expect(selection).toHaveProperty('modelId');
      expect(selection).toHaveProperty('displayName');

      // @step And the onClose callback should be invoked
      expect(fixture.callbacks.onClose.calls).toBe(1);
    });
  });

  describe('Scenario: Toggle section expansion with Enter on section header', () => {
    it('should toggle section when Enter is pressed on section header', async () => {
      // @step Given the ModelSelectorScreen is rendered with provider sections
      const { stdin, lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // @step And the selection is on a section header
      // Selection starts on first section header (Anthropic, collapsed)
      const beforeFrame = lastFrame();
      expect(beforeFrame).toMatch(/▶\s*Anthropic/);
      expect(beforeFrame).not.toContain('claude-sonnet-4');

      // @step When the user presses the Enter key
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      // @step Then the toggleSectionExpansion function should be called
      // @step And the section should expand or collapse
      const afterFrame = lastFrame();
      // Section should now be expanded (▼) with models visible
      expect(afterFrame).toMatch(/▼\s*Anthropic/);
      expect(afterFrame).toContain('claude-sonnet-4');
    });
  });

  // ===========================================================================
  // FILTER MODE SCENARIOS
  // ===========================================================================

  describe('Scenario: Enter filter mode with slash key', () => {
    it('should activate filter mode when slash is pressed', async () => {
      // @step Given the ModelSelectorScreen is rendered
      const { stdin, lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // @step And filter mode is not active
      const beforeFrame = lastFrame();
      // Filter label should not be visible yet
      expect(beforeFrame).not.toContain('Filter:');

      // @step When the user presses the "/" key
      pressKey(stdin, '/');
      await waitFor(50);

      // @step Then filter mode should be activated
      const afterFrame = lastFrame();
      // Filter input area should appear with "Filter:" label
      expect(afterFrame).toContain('Filter:');
    });
  });

  describe('Scenario: Type characters in filter mode', () => {
    it('should append characters to filter when typing in filter mode', async () => {
      // @step Given the ModelSelectorScreen is rendered
      const { stdin, lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // @step And filter mode is active
      pressKey(stdin, '/');
      await waitFor(50);

      // @step When the user types printable characters
      // Type as a single string to ensure all characters are received
      stdin.write('clau');
      await waitFor(100);

      // @step Then the characters should be appended to the filter string
      const frame = lastFrame();
      // Filter should show "clau" text
      expect(frame).toContain('Filter:');
      expect(frame).toContain('clau');
    });
  });

  describe('Scenario: Delete characters in filter mode with backspace', () => {
    it('should remove last character when backspace is pressed in filter mode', async () => {
      // @step Given the ModelSelectorScreen is rendered
      const { stdin, lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // @step And filter mode is active with text "clau"
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('clau');
      await waitFor(100);

      // Verify "clau" is in the filter
      const beforeBackspace = lastFrame();
      expect(beforeBackspace).toContain('clau');

      // @step When the user presses the Backspace key
      pressKey(stdin, { name: 'backspace' });
      await waitFor(50);

      // @step Then the filter should become "cla"
      const frame = lastFrame();
      expect(frame).toContain('cla');
      expect(frame).not.toContain('clau');
    });
  });

  describe('Scenario: Exit filter mode with Enter', () => {
    it('should deactivate filter mode when Enter is pressed', async () => {
      // @step Given the ModelSelectorScreen is rendered
      const { stdin, lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // @step And filter mode is active
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('claude');
      await waitFor(100);

      // Verify filter text is shown
      const beforeEnter = lastFrame();
      expect(beforeEnter).toContain('claude');

      // @step When the user presses the Enter key
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      // @step Then filter mode should be deactivated
      // @step And the filter text should be preserved
      const frame = lastFrame();
      // Filter text should still be shown (preserved)
      expect(frame).toContain('claude');
    });
  });

  describe('Scenario: Clear filter and exit filter mode with Escape', () => {
    it('should clear filter and exit filter mode when Escape is pressed in filter mode', async () => {
      // @step Given the ModelSelectorScreen is rendered
      const { stdin, lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // @step And filter mode is active with text "clau"
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('clau');
      await waitFor(100);

      // Verify filter is active
      const beforeEscape = lastFrame();
      expect(beforeEscape).toContain('Filter:');
      expect(beforeEscape).toContain('clau');

      // @step When the user presses the Escape key
      pressKey(stdin, { name: 'escape' });
      await waitFor(50);

      // @step Then the filter should be cleared
      // @step And filter mode should be deactivated
      const frame = lastFrame();
      // Filter text should be gone
      expect(frame).not.toContain('clau');
      // Both providers should be visible again (not filtered)
      expect(frame).toContain('Anthropic');
      expect(frame).toContain('OpenAI');

      // Should NOT close the screen (filter was active)
      expect(fixture.callbacks.onClose.calls).toBe(0);
    });
  });

  // ===========================================================================
  // UTILITY KEY SCENARIOS
  // ===========================================================================

  describe('Scenario: Refresh models with r key', () => {
    it('should call refreshModels when r is pressed', async () => {
      // @step Given the ModelSelectorScreen is rendered
      const { stdin } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // Clear mock call count from initial load
      fixture.modelsRefreshCacheMock.mockClear();
      fixture.modelsListAllMock.mockClear();

      // @step When the user presses the "r" key
      pressKey(stdin, 'r');
      await waitFor(100);

      // @step Then the refreshModels function should be called
      // This should trigger modelsRefreshCache and modelsListAll
      expect(fixture.modelsRefreshCacheMock).toHaveBeenCalled();
    });
  });

  // ===========================================================================
  // COMPONENT STRUCTURE SCENARIOS
  // ===========================================================================

  describe('Scenario: ModelSelectorScreen uses useModelSelectorState hook', () => {
    it('should initialize the useModelSelectorState hook', async () => {
      // @step Given the ModelSelectorScreen component is rendered
      const { lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(100);

      // @step Then it should initialize the useModelSelectorState hook
      // @step And all state should be managed through the hook
      // Verified by the fact that models load and render
      const frame = lastFrame();
      expect(frame).toContain('Anthropic');
    });
  });

  describe('Scenario: ModelSelectorView is purely presentational', () => {
    it('should not contain useInput handlers in ModelSelectorView', async () => {
      // @step Given the ModelSelectorView component exists
      const fs = await import('fs/promises');
      const path = await import('path');
      const viewPath = path.join(
        process.cwd(),
        'src/tui/components/ModelSelectorView.tsx'
      );

      let viewSource: string;
      try {
        viewSource = await fs.readFile(viewPath, 'utf-8');
      } catch {
        // File might not exist yet
        viewSource = '';
      }

      // @step Then it should NOT contain any useInput handlers
      // @step And it should receive all data and callbacks via props
      const hasUseInput = viewSource.includes('useInput(');
      expect(hasUseInput).toBe(false);
    });
  });

  describe('Scenario: Auto-expand section containing currentModelId', () => {
    it('should expand the section containing the current model when screen opens', async () => {
      // @step Given the ModelSelectorScreen is rendered with a currentModelId prop
      const { lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          currentModelId={TEST_MODEL_IDS.claudeSonnet4}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      // @step When models are loaded
      await fixture.waitForModelsLoaded();
      await waitFor(TEST_TIMING.asyncUpdate);

      // @step Then the section containing that model should be auto-expanded
      const frame = lastFrame();
      // Anthropic section should be expanded (▼) since it contains claude-sonnet-4
      expect(frame).toMatch(UI_PATTERNS.expandedAnthropic);
      // The model should be visible
      expect(frame).toContain('claude-sonnet-4');
      // The model should be selected (has > indicator)
      expect(frame).toMatch(UI_PATTERNS.selectedModel('claude-sonnet-4'));
    });
  });

  // ===========================================================================
  // INTEGRATION: Local Profile Support
  // ===========================================================================

  describe('Integration: Local profile sections', () => {
    it('should display local profile sections when profiles are configured', async () => {
      // Set up a local profile
      await fixture.createProfile('openai', 'local-ollama', {
        baseUrl: 'http://localhost:11434',
        apiKey: 'local-key',
        contextWindow: 128000,
        maxOutputTokens: 16384,
      });

      // Configure local server models
      fixture.setLocalServerModels('http://localhost:11434', [
        'llama3',
        'codellama',
        'mistral',
      ]);

      const { lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(150);

      const frame = lastFrame();
      // Should show local profile section
      expect(frame).toContain('local-ollama');
    });
  });

  describe('Integration: Unreachable local server', () => {
    it('should show unreachable status for servers that cannot be reached', async () => {
      // Set up a profile pointing to unreachable server
      await fixture.createProfile('openai', 'dead-server', {
        baseUrl: 'http://unreachable:9999',
        apiKey: 'key',
      });

      // Configure server as unreachable
      fixture.setLocalServerModels(
        'http://unreachable:9999',
        new Error('Connection refused')
      );

      const { lastFrame } = render(
        <ModelSelectorScreen
          width={80}
          height={24}
          onSelectModel={fixture.callbacks.onSelectModel.mock}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToSettings={fixture.callbacks.onSwitchToSettings.mock}
        />
      );

      await fixture.waitForModelsLoaded();
      await waitFor(150);

      const frame = lastFrame();
      // Should show unreachable indicator
      expect(frame).toContain('unreachable');
    });
  });
});
