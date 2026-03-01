/**
 * OAuth mode input handler
 *
 * PROV-017: Handles keyboard input for OAuth login flow modes:
 * - oauth-browser-waiting: Esc to cancel
 * - oauth-device-waiting: Esc to cancel
 * - oauth-success: Enter/Esc to return to list
 * - oauth-error: Enter to retry, Esc to go back
 */

import type { Key } from 'ink';
import type { UseProviderSettingsStateReturn } from '../hooks/useProviderSettingsState';

/**
 * Handles input in OAuth flow modes.
 * Returns true if the input was handled, false if it should pass through.
 */
export function handleOauthMode(
  input: string,
  key: Key,
  providerSettings: UseProviderSettingsStateReturn
): boolean {
  const { mode } = providerSettings;

  // OAuth waiting states (browser or device): only Esc to cancel
  if (
    mode.type === 'oauth-browser-waiting' ||
    mode.type === 'oauth-device-waiting'
  ) {
    if (key.escape) {
      providerSettings.cancelOauth();
    }
    // Absorb all input during waiting
    return true;
  }

  // OAuth success: Enter or Esc to return to list
  if (mode.type === 'oauth-success') {
    if (key.return || key.escape) {
      providerSettings.setMode({ type: 'list' });
    }
    // Absorb all input
    return true;
  }

  // OAuth error: Enter to retry, Esc to go back
  if (mode.type === 'oauth-error') {
    if (key.return) {
      providerSettings.retryOauth();
    } else if (key.escape) {
      providerSettings.cancelOauth();
    }
    // Absorb all input
    return true;
  }

  return false;
}
