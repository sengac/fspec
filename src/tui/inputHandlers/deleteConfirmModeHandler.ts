/**
 * Delete confirmation mode input handler
 *
 * TUI-074: Handles keyboard input in delete confirmation mode
 */

import type { Key } from 'ink';
import type { UseProviderSettingsStateReturn } from '../hooks/useProviderSettingsState';
import type { PanelMode } from '../components/ProviderSettingsPanel';

/**
 * Handles input in delete confirmation mode
 * @returns true if input was handled (mode is active)
 */
export function handleDeleteConfirmMode(
  mode: PanelMode,
  input: string,
  key: Key,
  providerSettings: UseProviderSettingsStateReturn
): boolean {
  if (mode.type !== 'delete-profile') {
    return false;
  }

  if (input === 'y' || input === 'Y') {
    void providerSettings
      .removeProfile(mode.providerId, mode.profileName)
      .then(() => {
        providerSettings.setMode({ type: 'list' });
      });
    return true;
  }

  if (key.escape || input === 'n' || input === 'N') {
    providerSettings.setMode({ type: 'list' });
    return true;
  }

  return true; // Consume all input in delete mode
}
