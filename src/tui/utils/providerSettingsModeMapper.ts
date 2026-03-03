/**
 * Mode mapper for provider settings
 *
 * TUI-074: Maps hook mode types to panel mode types.
 * This is the ONLY place that translates between HookMode and PanelMode.
 */

import type { PanelMode } from '../components/ProviderSettingsPanel';
import type { HookMode } from '../types/settingsMode';
import type { UseProviderSettingsStateReturn } from '../hooks/useProviderSettingsState';

/**
 * Maps hook state to the effective panel mode for rendering.
 *
 * Input: HookMode (from the hook's internal state machine)
 * Output: PanelMode (the rendering variant for ProviderSettingsPanel)
 */
export function mapToEffectivePanelMode(
  providerSettings: UseProviderSettingsStateReturn
): PanelMode {
  const hookMode: HookMode = providerSettings.mode;

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

  if (hookMode.type === 'delete-api-key') {
    return hookMode;
  }

  if (hookMode.type === 'disconnect-oauth') {
    return hookMode;
  }

  if (hookMode.type === 'edit-api-key') {
    return {
      type: 'edit-api-key',
      providerId: hookMode.providerId,
      currentValue: providerSettings.editingApiKey,
    };
  }

  // OAuth modes pass through directly (they are already PanelMode-compatible)
  if (
    hookMode.type === 'oauth-browser-waiting' ||
    hookMode.type === 'oauth-device-waiting' ||
    hookMode.type === 'oauth-headless-code-entry' ||
    hookMode.type === 'oauth-success' ||
    hookMode.type === 'oauth-error'
  ) {
    return hookMode;
  }

  return { type: 'list' };
}
