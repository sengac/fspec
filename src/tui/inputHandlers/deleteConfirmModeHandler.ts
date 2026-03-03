/**
 * Delete confirmation mode input handler
 *
 * TUI-074: Handles keyboard input in delete confirmation mode
 */

import type { Key } from 'ink';
import type { UseProviderSettingsStateReturn } from '../hooks/useProviderSettingsState';
import type { HookMode } from '../types/settingsMode';

/**
 * Generic confirmation handler — extracts the y/n/Esc pattern.
 */
function handleConfirmation(
  input: string,
  key: Key,
  onConfirm: () => Promise<void>,
  onCancel: () => void
): boolean {
  if (input === 'y' || input === 'Y') {
    void onConfirm().then(onCancel);
    return true;
  }
  if (key.escape || input === 'n' || input === 'N') {
    onCancel();
    return true;
  }
  return true; // Consume all input in confirmation mode
}

/**
 * Handles input in delete confirmation mode
 * Supports: delete-profile, delete-api-key, disconnect-oauth
 * @returns true if input was handled (mode is active)
 */
export function handleDeleteConfirmMode(
  mode: HookMode,
  input: string,
  key: Key,
  providerSettings: UseProviderSettingsStateReturn
): boolean {
  const cancel = () => {
    providerSettings.setMode({ type: 'list' });
  };

  if (mode.type === 'delete-profile') {
    return handleConfirmation(
      input,
      key,
      () => providerSettings.removeProfile(mode.providerId, mode.profileName),
      cancel
    );
  }

  if (mode.type === 'delete-api-key') {
    return handleConfirmation(
      input,
      key,
      () => providerSettings.removeApiKey(mode.providerId),
      cancel
    );
  }

  if (mode.type === 'disconnect-oauth') {
    return handleConfirmation(
      input,
      key,
      () => providerSettings.disconnectOauth(mode.providerId),
      cancel
    );
  }

  return false;
}
