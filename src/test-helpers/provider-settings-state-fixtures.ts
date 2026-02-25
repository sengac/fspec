/**
 * Provider Settings State Test Fixtures
 *
 * PROV-007: Fixtures for testing useProviderSettingsState hook.
 *
 * These fixtures provide:
 * - Mode state verification helpers
 * - Form state verification helpers
 * - Mode type mapping verification
 *
 * SOLID: Single Responsibility - Only handles settings state test setup
 * DRY: Reusable across multiple test files
 * COMPOSABLE: Works with ProviderProfileFixture
 */

import type { ProfileConfig } from '../../utils/provider-config';

// ========================================
// TYPES - MATCHING REAL TYPES
// ========================================

/**
 * Settings view mode types (from types/provider.ts)
 * These are the HOOK mode types
 */
export type SettingsViewMode =
  | { type: 'list' }
  | { type: 'edit-api-key'; providerId: string }
  | { type: 'create-profile'; providerId: string }
  | { type: 'edit-profile'; providerId: string; profileName: string }
  | { type: 'delete-profile'; providerId: string; profileName: string };

/**
 * Panel mode types (from ProviderSettingsPanel.tsx)
 * These are the PANEL mode types used for rendering
 */
export type PanelMode =
  | { type: 'list' }
  | { type: 'edit-api-key'; providerId: string; currentValue: string }
  | {
      type: 'profile-form';
      providerId: string;
      profileName: string;
      isNew: boolean;
      values: Partial<ProfileConfig>;
      activeField: number;
      isEditingName: boolean;
    }
  | { type: 'delete-confirm'; providerId: string; profileName: string };

// ========================================
// MODE TYPE MAPPING
// ========================================

/**
 * Maps hook mode to panel mode.
 * This is the same logic that should be in AgentView.
 *
 * @param hookMode - The mode from useProviderSettingsState
 * @param formState - Current form state
 * @returns The effective panel mode for rendering
 */
export function mapHookModeToPanelMode(
  hookMode: SettingsViewMode,
  formState: {
    formValues: Partial<ProfileConfig>;
    profileName: string;
    formFieldIndex: number;
    isEditingName: boolean;
    editingApiKey: string;
  }
): PanelMode {
  if (hookMode.type === 'create-profile' || hookMode.type === 'edit-profile') {
    return {
      type: 'profile-form',
      providerId: hookMode.providerId,
      profileName: formState.profileName,
      isNew: hookMode.type === 'create-profile',
      values: formState.formValues,
      activeField: formState.formFieldIndex,
      isEditingName: formState.isEditingName,
    };
  }

  if (hookMode.type === 'delete-profile') {
    return {
      type: 'delete-confirm',
      providerId: hookMode.providerId,
      profileName: hookMode.profileName,
    };
  }

  if (hookMode.type === 'edit-api-key') {
    return {
      type: 'edit-api-key',
      providerId: hookMode.providerId,
      currentValue: formState.editingApiKey,
    };
  }

  return { type: 'list' };
}

// ========================================
// MODE TYPE PREDICATES
// ========================================

/**
 * Checks if mode is a profile form mode (create or edit).
 * This is what the input handler should check.
 */
export function isProfileFormMode(mode: SettingsViewMode): boolean {
  return mode.type === 'create-profile' || mode.type === 'edit-profile';
}

/**
 * Checks if mode is a new profile (create mode).
 */
export function isNewProfileMode(mode: SettingsViewMode): boolean {
  return mode.type === 'create-profile';
}

/**
 * Checks if mode is delete confirmation.
 */
export function isDeleteMode(mode: SettingsViewMode): boolean {
  return mode.type === 'delete-profile';
}

/**
 * Checks if mode is API key editing.
 */
export function isApiKeyEditMode(mode: SettingsViewMode): boolean {
  return mode.type === 'edit-api-key';
}

// ========================================
// FORM STATE FACTORY
// ========================================

/**
 * Creates default form state for testing.
 */
export function createDefaultFormState() {
  return {
    formValues: {} as Partial<ProfileConfig>,
    profileName: '',
    formFieldIndex: 0,
    isEditingName: false,
    editingApiKey: '',
  };
}

/**
 * Creates form state for new profile creation.
 */
export function createNewProfileFormState(providerId: string) {
  return {
    formValues: {
      baseUrl: 'http://localhost:8888',
      apiKey: '',
    },
    profileName: '',
    formFieldIndex: 0,
    isEditingName: true,
    editingApiKey: '',
  };
}

/**
 * Creates form state for editing an existing profile.
 */
export function createEditProfileFormState(
  profileName: string,
  config: ProfileConfig
) {
  return {
    formValues: { ...config },
    profileName,
    formFieldIndex: 0,
    isEditingName: false,
    editingApiKey: '',
  };
}

// ========================================
// MODE TRANSITION HELPERS
// ========================================

/**
 * Simulates transitioning to create profile mode.
 */
export function createProfileModeTransition(providerId: string): {
  mode: SettingsViewMode;
  formState: ReturnType<typeof createNewProfileFormState>;
} {
  return {
    mode: { type: 'create-profile', providerId },
    formState: createNewProfileFormState(providerId),
  };
}

/**
 * Simulates transitioning to edit profile mode.
 */
export function editProfileModeTransition(
  providerId: string,
  profileName: string,
  config: ProfileConfig
): {
  mode: SettingsViewMode;
  formState: ReturnType<typeof createEditProfileFormState>;
} {
  return {
    mode: { type: 'edit-profile', providerId, profileName },
    formState: createEditProfileFormState(profileName, config),
  };
}

/**
 * Simulates transitioning to delete profile mode.
 */
export function deleteProfileModeTransition(
  providerId: string,
  profileName: string
): {
  mode: SettingsViewMode;
} {
  return {
    mode: { type: 'delete-profile', providerId, profileName },
  };
}

// ========================================
// INPUT SIMULATION HELPERS
// ========================================

/**
 * Simulates typing characters into a form field.
 */
export function simulateTyping(
  current: string,
  chars: string,
  append = true
): string {
  return append ? current + chars : chars;
}

/**
 * Simulates backspace on a form field.
 */
export function simulateBackspace(current: string): string {
  return current.slice(0, -1);
}

/**
 * Simulates field navigation.
 */
export function simulateFieldNavigation(
  currentIndex: number,
  direction: 'next' | 'prev',
  maxIndex: number
): number {
  if (direction === 'next') {
    return Math.min(currentIndex + 1, maxIndex);
  }
  return Math.max(currentIndex - 1, 0);
}
