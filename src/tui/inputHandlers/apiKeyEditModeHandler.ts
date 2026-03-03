/**
 * API key edit mode input handler
 *
 * TUI-074: Handles keyboard input in API key edit mode
 */

import type { Key } from 'ink';
import type { UseProviderSettingsStateReturn } from '../hooks/useProviderSettingsState';
import type { HookMode } from '../types/settingsMode';
import { filterPrintableChars } from '../utils/providerSettingsHelpers';

/**
 * Handles input in API key edit mode
 * @returns true if input was handled (mode is active)
 */
export function handleApiKeyEditMode(
  mode: HookMode,
  input: string,
  key: Key,
  providerSettings: UseProviderSettingsStateReturn
): boolean {
  if (mode.type !== 'edit-api-key') {
    return false;
  }

  if (key.escape) {
    providerSettings.setMode({ type: 'list' });
    providerSettings.setEditingApiKey('');
    return true;
  }

  if (key.return) {
    const apiKey = providerSettings.editingApiKey.trim();
    if (apiKey) {
      void providerSettings.saveApiKey(mode.providerId, apiKey).then(() => {
        providerSettings.setMode({ type: 'list' });
        providerSettings.setEditingApiKey('');
      });
    } else {
      providerSettings.setMode({ type: 'list' });
      providerSettings.setEditingApiKey('');
    }
    return true;
  }

  if (key.backspace || key.delete) {
    providerSettings.setEditingApiKey(prev => prev.slice(0, -1));
    return true;
  }

  const cleanApiKey = filterPrintableChars(input);
  if (cleanApiKey) {
    providerSettings.setEditingApiKey(prev => prev + cleanApiKey);
  }
  return true;
}
