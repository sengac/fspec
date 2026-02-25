/**
 * PROV-007: Provider Settings TUI Mode Type Tests
 *
 * Unit tests for provider settings TUI mode type handling.
 * These tests verify the mode type synchronization between
 * useProviderSettingsState hook and AgentView input handler.
 *
 * Validates that mode types are correctly mapped and handled.
 *
 * SOLID: Tests use composable fixtures, no mocks for pure logic
 * DRY: Reuses fixtures across scenarios
 */

import { describe, it, expect } from 'vitest';

import {
  mapHookModeToPanelMode,
  isProfileFormMode,
  isNewProfileMode,
  isDeleteMode,
  isApiKeyEditMode,
  createDefaultFormState,
  createNewProfileFormState,
  createEditProfileFormState,
  createProfileModeTransition,
  editProfileModeTransition,
  deleteProfileModeTransition,
  simulateTyping,
  simulateBackspace,
  simulateFieldNavigation,
  type SettingsViewMode,
} from '../../test-helpers/provider-settings-state-fixtures';

describe('Feature: Provider Settings TUI Mode Type Handling', () => {
  // ============================================
  // MODE TYPE MAPPING
  // ============================================

  describe('Scenario: Mode types are correctly mapped from hook to panel', () => {
    it('should map create-profile mode to profile-form with isNew=true', () => {
      // @step Given I have a create-profile mode from the hook
      const hookMode: SettingsViewMode = {
        type: 'create-profile',
        providerId: 'openai',
      };
      const formState = createNewProfileFormState('openai');

      // @step When I map the hook mode to panel mode
      const panelMode = mapHookModeToPanelMode(hookMode, formState);

      // @step Then the panel mode should be profile-form
      expect(panelMode.type).toBe('profile-form');

      // @step And isNew should be true
      if (panelMode.type === 'profile-form') {
        expect(panelMode.isNew).toBe(true);
        expect(panelMode.providerId).toBe('openai');
      }
    });

    it('should map edit-profile mode to profile-form with isNew=false', () => {
      // @step Given I have an edit-profile mode from the hook
      const hookMode: SettingsViewMode = {
        type: 'edit-profile',
        providerId: 'openai',
        profileName: 'work-vllm',
      };
      const formState = createEditProfileFormState('work-vllm', {
        baseUrl: 'http://work:8888',
        apiKey: 'test-key',
      });

      // @step When I map the hook mode to panel mode
      const panelMode = mapHookModeToPanelMode(hookMode, formState);

      // @step Then the panel mode should be profile-form
      expect(panelMode.type).toBe('profile-form');

      // @step And isNew should be false
      if (panelMode.type === 'profile-form') {
        expect(panelMode.isNew).toBe(false);
        expect(panelMode.profileName).toBe('work-vllm');
      }
    });

    it('should map delete-profile mode to delete-confirm', () => {
      // @step Given I have a delete-profile mode from the hook
      const hookMode: SettingsViewMode = {
        type: 'delete-profile',
        providerId: 'openai',
        profileName: 'work-vllm',
      };
      const formState = createDefaultFormState();

      // @step When I map the hook mode to panel mode
      const panelMode = mapHookModeToPanelMode(hookMode, formState);

      // @step Then the panel mode should be delete-confirm
      expect(panelMode.type).toBe('delete-confirm');

      // @step And it should have the profile name
      if (panelMode.type === 'delete-confirm') {
        expect(panelMode.profileName).toBe('work-vllm');
      }
    });

    it('should map edit-api-key mode with current value', () => {
      // @step Given I have an edit-api-key mode from the hook
      const hookMode: SettingsViewMode = {
        type: 'edit-api-key',
        providerId: 'openai',
      };
      const formState = {
        ...createDefaultFormState(),
        editingApiKey: 'sk-partial',
      };

      // @step When I map the hook mode to panel mode
      const panelMode = mapHookModeToPanelMode(hookMode, formState);

      // @step Then the panel mode should be edit-api-key
      expect(panelMode.type).toBe('edit-api-key');

      // @step And it should have the current value
      if (panelMode.type === 'edit-api-key') {
        expect(panelMode.currentValue).toBe('sk-partial');
      }
    });

    it('should map list mode unchanged', () => {
      // @step Given I have a list mode from the hook
      const hookMode: SettingsViewMode = { type: 'list' };
      const formState = createDefaultFormState();

      // @step When I map the hook mode to panel mode
      const panelMode = mapHookModeToPanelMode(hookMode, formState);

      // @step Then the panel mode should be list
      expect(panelMode.type).toBe('list');
    });
  });

  // ============================================
  // MODE TYPE PREDICATES
  // ============================================

  describe('Scenario: Input handler correctly identifies profile form modes', () => {
    it('should identify create-profile as profile form mode', () => {
      // @step Given I have a create-profile mode
      const mode: SettingsViewMode = {
        type: 'create-profile',
        providerId: 'openai',
      };

      // @step When I check if it's a profile form mode
      const result = isProfileFormMode(mode);

      // @step Then it should return true
      expect(result).toBe(true);

      // @step And isNewProfileMode should also be true
      expect(isNewProfileMode(mode)).toBe(true);
    });

    it('should identify edit-profile as profile form mode', () => {
      // @step Given I have an edit-profile mode
      const mode: SettingsViewMode = {
        type: 'edit-profile',
        providerId: 'openai',
        profileName: 'work-vllm',
      };

      // @step When I check if it's a profile form mode
      const result = isProfileFormMode(mode);

      // @step Then it should return true
      expect(result).toBe(true);

      // @step And isNewProfileMode should be false
      expect(isNewProfileMode(mode)).toBe(false);
    });

    it('should NOT identify list mode as profile form mode', () => {
      // @step Given I have a list mode
      const mode: SettingsViewMode = { type: 'list' };

      // @step When I check if it's a profile form mode
      const result = isProfileFormMode(mode);

      // @step Then it should return false
      expect(result).toBe(false);
    });

    it('should NOT identify delete-profile as profile form mode', () => {
      // @step Given I have a delete-profile mode
      const mode: SettingsViewMode = {
        type: 'delete-profile',
        providerId: 'openai',
        profileName: 'work-vllm',
      };

      // @step When I check if it's a profile form mode
      expect(isProfileFormMode(mode)).toBe(false);

      // @step But it should be identified as delete mode
      expect(isDeleteMode(mode)).toBe(true);
    });

    it('should identify edit-api-key mode correctly', () => {
      // @step Given I have an edit-api-key mode
      const mode: SettingsViewMode = {
        type: 'edit-api-key',
        providerId: 'openai',
      };

      // @step When I check mode type predicates
      expect(isProfileFormMode(mode)).toBe(false);
      expect(isApiKeyEditMode(mode)).toBe(true);
      expect(isDeleteMode(mode)).toBe(false);
    });
  });

  // ============================================
  // MODE TRANSITIONS
  // ============================================

  describe('Scenario: Create a new profile - mode transition', () => {
    it('should set up correct mode and form state for new profile', () => {
      // @step Given I am viewing the "openai" provider in /provider screen
      // @step When I create a new profile
      const { mode, formState } = createProfileModeTransition('openai');

      // @step Then the mode should be create-profile
      expect(mode.type).toBe('create-profile');
      expect(mode.providerId).toBe('openai');

      // @step And the form state should be initialized for new profile
      expect(formState.formValues.baseUrl).toBe('http://localhost:8888');
      expect(formState.formValues.apiKey).toBe('');
      expect(formState.profileName).toBe('');
      expect(formState.isEditingName).toBe(true);
      expect(formState.formFieldIndex).toBe(0);
    });
  });

  describe('Scenario: Edit an existing profile - mode transition', () => {
    it('should set up correct mode and form state for edit', () => {
      // @step Given I have a profile "work-vllm" configured for "openai" provider
      const existingConfig = {
        baseUrl: 'http://work:8888',
        apiKey: 'local-key',
        contextWindow: 32768,
        maxOutputTokens: 8192,
      };

      // @step When I edit the "work-vllm" profile
      const { mode, formState } = editProfileModeTransition(
        'openai',
        'work-vllm',
        existingConfig
      );

      // @step Then the mode should be edit-profile
      expect(mode.type).toBe('edit-profile');
      if (mode.type === 'edit-profile') {
        expect(mode.providerId).toBe('openai');
        expect(mode.profileName).toBe('work-vllm');
      }

      // @step And the form state should have the existing values
      expect(formState.formValues.baseUrl).toBe('http://work:8888');
      expect(formState.formValues.apiKey).toBe('local-key');
      expect(formState.formValues.contextWindow).toBe(32768);
      expect(formState.profileName).toBe('work-vllm');
      expect(formState.isEditingName).toBe(false);
    });
  });

  describe('Scenario: Delete a profile - mode transition', () => {
    it('should set up correct mode for delete confirmation', () => {
      // @step Given I have a profile "home-ollama" configured for "openai" provider
      // @step When I delete the "home-ollama" profile
      const { mode } = deleteProfileModeTransition('openai', 'home-ollama');

      // @step Then the mode should be delete-profile
      expect(mode.type).toBe('delete-profile');
      if (mode.type === 'delete-profile') {
        expect(mode.providerId).toBe('openai');
        expect(mode.profileName).toBe('home-ollama');
      }
    });
  });

  // ============================================
  // FORM INPUT SIMULATION
  // ============================================

  describe('Scenario: Form field input handling', () => {
    it('should simulate typing characters into profile name', () => {
      // @step Given I am editing the profile name field
      let profileName = '';

      // @step When I type "work-vllm"
      profileName = simulateTyping(profileName, 'work-vllm');

      // @step Then the profile name should be "work-vllm"
      expect(profileName).toBe('work-vllm');
    });

    it('should simulate backspace on form field', () => {
      // @step Given I have typed "work-vllm" in the profile name field
      let profileName = 'work-vllm';

      // @step When I press backspace
      profileName = simulateBackspace(profileName);

      // @step Then the profile name should be "work-vll"
      expect(profileName).toBe('work-vll');
    });

    it('should simulate Tab to navigate between fields', () => {
      // @step Given I am on the first field (index 0)
      let fieldIndex = 0;
      const maxIndex = 3; // baseUrl, apiKey, contextWindow, maxOutputTokens

      // @step When I press Tab
      fieldIndex = simulateFieldNavigation(fieldIndex, 'next', maxIndex);

      // @step Then I should be on the second field
      expect(fieldIndex).toBe(1);

      // @step When I press Tab again
      fieldIndex = simulateFieldNavigation(fieldIndex, 'next', maxIndex);

      // @step Then I should be on the third field
      expect(fieldIndex).toBe(2);
    });

    it('should simulate Shift+Tab to navigate backwards', () => {
      // @step Given I am on the third field (index 2)
      let fieldIndex = 2;
      const maxIndex = 3;

      // @step When I press Shift+Tab
      fieldIndex = simulateFieldNavigation(fieldIndex, 'prev', maxIndex);

      // @step Then I should be on the second field
      expect(fieldIndex).toBe(1);
    });

    it('should not navigate past field boundaries', () => {
      // @step Given I am on the first field (index 0)
      let fieldIndex = 0;
      const maxIndex = 3;

      // @step When I press Shift+Tab
      fieldIndex = simulateFieldNavigation(fieldIndex, 'prev', maxIndex);

      // @step Then I should still be on the first field
      expect(fieldIndex).toBe(0);

      // @step Given I am on the last field
      fieldIndex = 3;

      // @step When I press Tab
      fieldIndex = simulateFieldNavigation(fieldIndex, 'next', maxIndex);

      // @step Then I should still be on the last field
      expect(fieldIndex).toBe(3);
    });
  });

  // ============================================
  // PANEL MODE WITH FORM STATE
  // ============================================

  describe('Scenario: Panel mode includes current form state', () => {
    it('should include current form values in panel mode', () => {
      // @step Given I am editing a profile with modified values
      const hookMode: SettingsViewMode = {
        type: 'edit-profile',
        providerId: 'openai',
        profileName: 'work-vllm',
      };
      const formState = {
        formValues: {
          baseUrl: 'http://new-server:9000',
          apiKey: 'updated-key',
          contextWindow: 65536,
        },
        profileName: 'work-vllm',
        formFieldIndex: 2,
        isEditingName: false,
        editingApiKey: '',
      };

      // @step When I map to panel mode
      const panelMode = mapHookModeToPanelMode(hookMode, formState);

      // @step Then the panel mode should have the updated values
      if (panelMode.type === 'profile-form') {
        expect(panelMode.values.baseUrl).toBe('http://new-server:9000');
        expect(panelMode.values.apiKey).toBe('updated-key');
        expect(panelMode.values.contextWindow).toBe(65536);
        expect(panelMode.activeField).toBe(2);
      }
    });
  });
});
