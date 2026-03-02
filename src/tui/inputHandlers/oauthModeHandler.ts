/**
 * OAuth mode input handler
 *
 * PROV-017: Handles keyboard input for OAuth login flow modes:
 * - oauth-browser-waiting: Esc to cancel
 * - oauth-device-waiting: Esc to cancel
 * - oauth-headless-code-entry: character input, Enter to submit, Esc to cancel
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

  // OAuth headless code entry (Anthropic): character input, Enter to submit, Esc to cancel
  if (mode.type === 'oauth-headless-code-entry') {
    if (key.escape) {
      providerSettings.cancelOauth();
      return true;
    }
    if (key.return) {
      if (mode.codeInput.length > 0) {
        providerSettings.submitHeadlessCode(mode.codeInput, mode.pkceVerifier);
      }
      return true;
    }
    if (key.backspace || key.delete) {
      if (mode.codeInput.length > 0) {
        providerSettings.setMode({
          ...mode,
          codeInput: mode.codeInput.slice(0, -1),
        });
      }
      return true;
    }
    // Regular character input — append to codeInput
    if (input && !key.ctrl && !key.meta) {
      providerSettings.setMode({
        ...mode,
        codeInput: mode.codeInput + input,
      });
    }
    // Absorb all input during code entry
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
