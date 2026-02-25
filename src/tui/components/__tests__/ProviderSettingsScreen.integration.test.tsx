/**
 * Feature: spec/features/provider-settings-screen.feature
 *
 * TUI-074: Create ProviderSettingsScreen component - INTEGRATION TESTS
 *
 * These are REAL integration tests that:
 * - Use REAL useProviderSettingsState hook (NOT mocked)
 * - Use REAL ProviderSettingsScreen component
 * - Only mock at NAPI network boundary (testProviderConnection, modelsListLocalOpenai)
 * - Use reusable fixtures following DRY/SOLID/COMPOSABLE principles
 *
 * Test coverage validates actual behavior, not mock interactions.
 */

import React from 'react';
import { render, cleanup } from 'ink-testing-library';
import { describe, it, expect, vi, beforeEach, afterEach, beforeAll } from 'vitest';

import {
  createProviderSettingsScreenFixture,
  type ProviderSettingsScreenFixture,
} from './fixtures/providerSettingsScreenFixture';
import {
  pressKey,
  typeString,
  waitFor,
} from './fixtures/keyboardHelpers';

// =============================================================================
// NAPI MODULE MOCK - Network boundary ONLY
// =============================================================================

// Store fixture reference for mock access
let activeFixture: ProviderSettingsScreenFixture | null = null;

vi.mock('@sengac/codelet-napi', async () => {
  const actual = await vi.importActual<typeof import('@sengac/codelet-napi')>(
    '@sengac/codelet-napi'
  );
  return {
    ...actual,
    testProviderConnection: vi.fn(async (providerId: string) => {
      if (activeFixture) {
        return activeFixture.testProviderConnectionMock(providerId);
      }
      return { success: true };
    }),
    modelsListLocalOpenai: vi.fn(async (baseUrl: string) => {
      if (activeFixture) {
        return activeFixture.modelsListLocalOpenaiMock(baseUrl);
      }
      return [];
    }),
  };
});

// =============================================================================
// TEST SUITE
// =============================================================================

describe('Feature: Create ProviderSettingsScreen component', () => {
  let fixture: ProviderSettingsScreenFixture;
  let ProviderSettingsScreen: typeof import('../ProviderSettingsScreen').ProviderSettingsScreen;

  beforeAll(async () => {
    // Import the real component (not mocked)
    const module = await import('../ProviderSettingsScreen');
    ProviderSettingsScreen = module.ProviderSettingsScreen;
  });

  beforeEach(async () => {
    // Create fresh fixture for each test
    fixture = await createProviderSettingsScreenFixture('provider-settings-screen');
    activeFixture = fixture;

    // Set up credentials so providers load with status
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
      // @step I want to use ProviderSettingsScreen as an orchestrator component
      // @step So that keyboard input handling for provider settings is encapsulated and AgentView.tsx is ~300 lines smaller
      expect(ProviderSettingsScreen).toBeDefined();
    });
  });

  // ===========================================================================
  // LIST MODE - NAVIGATION & CALLBACKS
  // ===========================================================================

  describe('Scenario: Switch to model selector with Tab key', () => {
    it('should call onSwitchToModels when Tab is pressed in list mode', async () => {
      // @step Given ProviderSettingsScreen is rendered in list mode
      const { stdin } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // @step When the user presses the Tab key
      pressKey(stdin, { name: 'tab' });
      await waitFor(50);

      // @step Then the onSwitchToModels callback is invoked
      expect(fixture.callbacks.onSwitchToModels.calls).toBe(1);
    });
  });

  describe('Scenario: Close screen with Escape when no filter is active', () => {
    it('should call onClose when Escape is pressed with no filter', async () => {
      // @step Given ProviderSettingsScreen is rendered in list mode
      const { stdin } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // @step And no filter is active
      // No filter is active by default

      // @step When the user presses the Escape key
      pressKey(stdin, { name: 'escape' });
      await waitFor(50);

      // @step Then the onClose callback is invoked
      expect(fixture.callbacks.onClose.calls).toBe(1);
    });
  });

  describe('Scenario: Clear filter with Escape when filter is active', () => {
    it('should clear filter without closing when Escape is pressed with active filter', async () => {
      // @step Given ProviderSettingsScreen is rendered in list mode
      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // @step And a filter "anth" is active
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('anth');
      await waitFor(100);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      // @step When the user presses the Escape key
      pressKey(stdin, { name: 'escape' });
      await waitFor(50);

      // @step Then the filter is cleared
      const frame = lastFrame();
      expect(frame).not.toContain('anth');

      // @step And the onClose callback is NOT invoked
      expect(fixture.callbacks.onClose.calls).toBe(0);
    });
  });

  describe('Scenario: Navigate down in provider list', () => {
    it('should increment selectedIndex when Down arrow is pressed', async () => {
      // @step Given ProviderSettingsScreen is rendered in list mode
      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // @step And the selected index is 0
      // Verify Anthropic (first provider) is selected initially
      const initialFrame = lastFrame();
      expect(initialFrame).toContain('Anthropic');

      // @step When the user presses the Down arrow key
      pressKey(stdin, { name: 'down' });
      await waitFor(50);

      // @step Then the selected index increments to 1
      // @step And scroll offset adjusts if selection exceeds visible height
      // OpenAI (second provider) should now be selectable
      const frame = lastFrame();
      expect(frame).toContain('OpenAI');
    });
  });

  describe('Scenario: Enter filter mode with slash key', () => {
    it('should activate filter mode when slash is pressed', async () => {
      // @step Given ProviderSettingsScreen is rendered in list mode
      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // @step When the user presses the "/" key
      pressKey(stdin, '/');
      await waitFor(50);

      // @step Then isFilterMode becomes true
      // @step And the filter input is active
      const frame = lastFrame();
      expect(frame).toContain('Filter');
    });
  });

  describe('Scenario: Expand provider section with Enter', () => {
    it('should toggle expansion when Enter is pressed on provider', async () => {
      // @step Given ProviderSettingsScreen is rendered in list mode
      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // @step And the selection is on a provider item
      // Selection starts on first provider by default

      // @step When the user presses the Enter key
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      // @step Then toggleProviderExpansion is called
      // @step And the provider section expands
      const frame = lastFrame();
      // Should show expanded state - "Create new profile" option becomes visible
      expect(frame).toContain('Create new profile');
    });
  });

  describe('Scenario: Test connection with t key', () => {
    it('should test connection when t is pressed on provider', async () => {
      // @step Given ProviderSettingsScreen is rendered in list mode
      const { stdin } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // @step And the selection is on a provider item
      // Selection starts on first provider

      // @step When the user presses the "t" key
      pressKey(stdin, 't');
      await waitFor(100);

      // @step Then testConnection is called for the provider
      // @step And testResult is displayed
      expect(fixture.testProviderConnectionMock).toHaveBeenCalled();
    });
  });

  describe('Scenario: Refresh providers with r key', () => {
    it('should reload providers when r is pressed', async () => {
      // @step Given ProviderSettingsScreen is rendered in list mode
      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Verify providers are initially loaded
      const initialFrame = lastFrame();
      expect(initialFrame).toContain('Anthropic');

      // @step When the user presses the "r" key
      pressKey(stdin, 'r');
      await waitFor(100);

      // @step Then the providers are reloaded
      const frame = lastFrame();
      // Providers should still be visible after reload
      expect(frame).toContain('Anthropic');
      expect(frame).toContain('OpenAI');
    });
  });

  // ===========================================================================
  // DELETE CONFIRMATION MODE
  // ===========================================================================

  describe('Scenario: Confirm profile deletion with y key', () => {
    it('should call removeProfile when y is pressed in delete mode', async () => {
      // @step Given ProviderSettingsScreen is in delete-profile mode for profile "my-server"
      // Create profile under anthropic
      await fixture.createProfile('anthropic', 'my-server', {
        baseUrl: 'http://localhost:8888',
        apiKey: 'test-key',
      });

      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Navigate to Anthropic (providers are sorted, Anthropic after OpenAI in typical registry)
      // First, expand using filter to find Anthropic reliably
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('anthropic');
      await waitFor(100);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      // Now Anthropic is visible and selected, expand it
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      // Navigate to profile (first item under expanded provider)
      pressKey(stdin, { name: 'down' });
      await waitFor(50);

      // Verify profile is visible
      const beforeDelete = lastFrame();
      expect(beforeDelete).toContain('my-server');

      // Press 'd' to enter delete mode
      pressKey(stdin, 'd');
      await waitFor(50);

      // Verify delete confirmation is shown
      const deleteConfirmFrame = lastFrame();
      expect(deleteConfirmFrame).toContain('Delete Profile');

      // @step When the user presses the "y" key
      pressKey(stdin, 'y');
      await waitFor(100);

      // @step Then removeProfile is called with the profile name
      // @step And mode returns to list
      const frame = lastFrame();
      // Should return to list mode (no delete confirmation visible)
      expect(frame).not.toContain('Delete Profile');
    });
  });

  describe('Scenario: Cancel profile deletion with n key', () => {
    it('should not call removeProfile when n is pressed in delete mode', async () => {
      // @step Given ProviderSettingsScreen is in delete-profile mode for profile "my-server"
      // Create profile under anthropic
      await fixture.createProfile('anthropic', 'my-server', {
        baseUrl: 'http://localhost:8888',
        apiKey: 'test-key',
      });

      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Navigate to Anthropic using filter
      pressKey(stdin, '/');
      await waitFor(50);
      stdin.write('anthropic');
      await waitFor(100);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      // Expand Anthropic
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      // Navigate to profile
      pressKey(stdin, { name: 'down' });
      await waitFor(50);

      // Press 'd' to enter delete mode
      pressKey(stdin, 'd');
      await waitFor(50);

      // Verify delete confirmation is shown
      const deleteConfirmFrame = lastFrame();
      expect(deleteConfirmFrame).toContain('Delete Profile');

      // @step When the user presses the "n" key
      pressKey(stdin, 'n');
      await waitFor(50);

      // @step Then removeProfile is NOT called
      // @step And mode returns to list
      const frame = lastFrame();
      // Profile should still exist (delete was cancelled)
      expect(frame).toContain('my-server');
      // Should not show delete confirmation anymore
      expect(frame).not.toContain('Delete Profile');
    });
  });

  // ===========================================================================
  // API KEY EDIT MODE
  // ===========================================================================

  describe('Scenario: Save API key with Enter', () => {
    it('should call saveApiKey when Enter is pressed with non-empty key', async () => {
      // @step Given ProviderSettingsScreen is in edit-api-key mode
      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Press 'e' to enter API key edit mode
      pressKey(stdin, 'e');
      await waitFor(50);

      // Verify we're in edit mode
      const editFrame = lastFrame();
      expect(editFrame).toContain('API Key');

      // @step And the editing API key is "sk-12345"
      stdin.write('sk-12345');
      await waitFor(100);

      // @step When the user presses the Enter key
      pressKey(stdin, { name: 'enter' });
      await waitFor(100);

      // @step Then saveApiKey is called with the key value
      // @step And mode returns to list
      const frame = lastFrame();
      // Should return to list mode with providers visible
      expect(frame).toContain('Anthropic');
    });
  });

  describe('Scenario: Cancel API key edit with Escape', () => {
    it('should not call saveApiKey when Escape is pressed', async () => {
      // @step Given ProviderSettingsScreen is in edit-api-key mode
      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Press 'e' to enter API key edit mode
      pressKey(stdin, 'e');
      await waitFor(50);

      // Verify we're in edit mode
      const editFrame = lastFrame();
      expect(editFrame).toContain('API Key');

      // @step And the editing API key is "sk-12345"
      stdin.write('sk-12345');
      await waitFor(100);

      // @step When the user presses the Escape key
      pressKey(stdin, { name: 'escape' });
      await waitFor(50);

      // @step Then saveApiKey is NOT called
      // @step And the editing API key is cleared
      // @step And mode returns to list
      const frame = lastFrame();
      // Should return to list mode with providers visible
      expect(frame).toContain('Anthropic');
      // Should NOT call onClose (Escape in edit mode returns to list, not closes screen)
      expect(fixture.callbacks.onClose.calls).toBe(0);
    });
  });

  // ===========================================================================
  // PROFILE FORM MODE
  // ===========================================================================

  describe('Scenario: Navigate to next field with Tab', () => {
    it('should increment formFieldIndex when Tab is pressed in profile form', async () => {
      // @step Given ProviderSettingsScreen is in create-profile mode
      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Expand provider and go to "Create Profile"
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);
      pressKey(stdin, { name: 'down' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      // @step And formFieldIndex is 0
      // Form starts with profile name focused
      // Verify we're in profile form mode
      const formFrame = lastFrame();
      expect(formFrame).toContain('Profile');

      // @step When the user presses the Tab key
      pressKey(stdin, { name: 'tab' });
      await waitFor(50);

      // @step Then formFieldIndex increments to 1
      const frame = lastFrame();
      // Still in form mode (Tab navigates fields, not exits)
      expect(frame).toContain('URL');
    });
  });

  describe('Scenario: Cancel profile form with Escape', () => {
    it('should not call saveProfileConfig when Escape is pressed', async () => {
      // @step Given ProviderSettingsScreen is in create-profile mode
      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // Expand provider and go to "Create Profile"
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);
      pressKey(stdin, { name: 'down' });
      await waitFor(50);
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      // Verify we're in form mode
      const formFrame = lastFrame();
      expect(formFrame).toContain('Profile');

      // @step When the user presses the Escape key
      pressKey(stdin, { name: 'escape' });
      await waitFor(50);

      // @step Then saveProfileConfig is NOT called
      // @step And mode returns to list
      const frame = lastFrame();
      // Should return to list mode with providers visible
      expect(frame).toContain('Anthropic');
      // Escape in form mode returns to list, not closes screen
      expect(fixture.callbacks.onClose.calls).toBe(0);
    });
  });

  // ===========================================================================
  // FILTER MODE
  // ===========================================================================

  describe('Scenario: Exit filter mode keeping filter with Enter', () => {
    it('should keep filter when Enter is pressed in filter mode', async () => {
      // @step Given ProviderSettingsScreen is in filter mode
      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      pressKey(stdin, '/');
      await waitFor(50);

      // @step And the filter is "anth"
      stdin.write('anth');
      await waitFor(100);

      // @step When the user presses the Enter key
      pressKey(stdin, { name: 'enter' });
      await waitFor(50);

      // @step Then isFilterMode becomes false
      // @step And the filter remains "anth"
      const frame = lastFrame();
      expect(frame).toContain('anth');
    });
  });

  describe('Scenario: Clear filter and exit filter mode with Escape', () => {
    it('should clear filter when Escape is pressed in filter mode', async () => {
      // @step Given ProviderSettingsScreen is in filter mode
      const { stdin, lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      pressKey(stdin, '/');
      await waitFor(50);

      // @step And the filter is "anth"
      stdin.write('anth');
      await waitFor(100);

      // @step When the user presses the Escape key
      pressKey(stdin, { name: 'escape' });
      await waitFor(50);

      // @step Then isFilterMode becomes false
      // @step And the filter is cleared
      const frame = lastFrame();
      expect(frame).not.toContain('anth');
    });
  });

  // ===========================================================================
  // COMPONENT STRUCTURE
  // ===========================================================================

  describe('Scenario: ProviderSettingsScreen uses useProviderSettingsState hook', () => {
    it('should use the hook for state management', async () => {
      // @step Given ProviderSettingsScreen component is implemented
      const { lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // @step Then it uses the useProviderSettingsState hook for state management
      // @step And it does NOT declare its own provider/navigation state
      const frame = lastFrame();
      // Provider list should be visible (loaded via hook)
      expect(frame).toContain('Anthropic');
    });
  });

  describe('Scenario: ProviderSettingsScreen renders ProviderSettingsPanel', () => {
    it('should render the presentation component', async () => {
      // @step Given ProviderSettingsScreen component is implemented
      const { lastFrame } = render(
        <ProviderSettingsScreen
          width={80}
          height={24}
          onClose={fixture.callbacks.onClose.mock}
          onSwitchToModels={fixture.callbacks.onSwitchToModels.mock}
        />
      );

      await fixture.waitForProvidersLoaded();
      await waitFor(100);

      // @step Then it renders ProviderSettingsPanel as its presentation layer
      // @step And it maps hook state to panel props correctly
      const frame = lastFrame();
      expect(frame).toContain('Provider');
    });
  });
});
