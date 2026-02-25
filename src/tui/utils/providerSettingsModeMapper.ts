/**
 * Mode mapper for provider settings
 *
 * TUI-074: Maps hook mode types to panel mode types
 */

import type { PanelMode } from '../components/ProviderSettingsPanel';
import type { UseProviderSettingsStateReturn } from '../hooks/useProviderSettingsState';

/**
 * Maps hook state to the effective panel mode for rendering
 */
export function mapToEffectivePanelMode(
  providerSettings: UseProviderSettingsStateReturn
): PanelMode {
  const hookMode = providerSettings.mode;

  if (hookMode.type === 'create-profile' || hookMode.type === 'edit-profile') {
    return {
      type: 'profile-form',
      providerId: hookMode.providerId,
      profileName: providerSettings.profileName,
      values: providerSettings.formValues,
      activeField: providerSettings.formFieldIndex,
      isEditingName: providerSettings.isEditingName,
      isNew: hookMode.type === 'create-profile',
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
      currentValue: providerSettings.editingApiKey,
    };
  }

  return { type: 'list' };
}
