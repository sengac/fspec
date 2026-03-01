/**
 * useProviderSettingsInput - Keyboard input handling for provider settings
 *
 * TUI-074: Extracts keyboard handling from AgentView.tsx
 * Feature: spec/features/provider-settings-screen.feature
 */

import { useInput } from 'ink';
import type { UseProviderSettingsStateReturn } from './useProviderSettingsState';
import {
  handleDeleteConfirmMode,
  handleApiKeyEditMode,
  handleProfileFormMode,
  handleFilterMode,
  handleListMode,
  handleOauthMode,
} from '../inputHandlers';

export interface UseProviderSettingsInputOptions {
  providerSettings: UseProviderSettingsStateReturn;
  visibleHeight: number;
  onClose: () => void;
  onSwitchToModels: () => void;
}

/**
 * Hook that handles all keyboard input for provider settings screen
 */
export function useProviderSettingsInput({
  providerSettings,
  visibleHeight,
  onClose,
  onSwitchToModels,
}: UseProviderSettingsInputOptions): void {
  useInput(
    (input, key) => {
      const currentItem = providerSettings.getCurrentItem();
      const currentProvider = providerSettings.getCurrentProvider();
      const currentProfile = providerSettings.getCurrentProfile();
      const { mode } = providerSettings;

      // Handle each mode in priority order
      if (handleOauthMode(input, key, providerSettings)) {
        return;
      }
      if (handleDeleteConfirmMode(mode, input, key, providerSettings)) {
        return;
      }
      if (handleApiKeyEditMode(mode, input, key, providerSettings)) {
        return;
      }
      if (handleProfileFormMode(mode, input, key, providerSettings)) {
        return;
      }
      if (handleFilterMode(input, key, providerSettings)) {
        return;
      }

      // List mode handles remaining input
      handleListMode({
        input,
        key,
        providerSettings,
        currentItem,
        currentProvider,
        currentProfile,
        visibleHeight,
        onClose,
        onSwitchToModels,
      });
    },
    { isActive: true }
  );
}
