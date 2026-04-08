/**
 * Copilot OAuth mode input handler (PROV-054).
 *
 * Handles keyboard input for the two new GitHub Copilot OAuth modes:
 *
 * - `oauth-deployment-type-select` — up/down arrows toggle between
 *   github.com (index 0) and enterprise (index 1); Enter submits the
 *   choice via `submitCopilotDeploymentType`; Esc cancels back to list.
 *
 * - `oauth-enterprise-url-entry` — printable ASCII characters append to
 *   `urlInput`; backspace/delete pop the last character; Enter submits the
 *   typed URL via `submitCopilotEnterpriseUrl`; Esc cancels back to list.
 *
 * SoC: this file contains ONLY input dispatch. The actual state
 * transitions live in `utils/copilotLoginFlow.ts`.
 */

import type { Key } from 'ink';
import type { UseProviderSettingsStateReturn } from '../hooks/useProviderSettingsState';
import {
  submitCopilotDeploymentType,
  submitCopilotEnterpriseUrl,
} from '../utils/copilotLoginFlow';

/**
 * Handle input in the Copilot deployment-type-select / enterprise-url-entry
 * modes. Returns true if the input was handled, false otherwise.
 */
export function handleCopilotOauthMode(
  input: string,
  key: Key,
  providerSettings: UseProviderSettingsStateReturn
): boolean {
  const { mode } = providerSettings;

  // Deployment-type radio prompt
  if (mode.type === 'oauth-deployment-type-select') {
    if (key.escape) {
      providerSettings.cancelOauth();
      return true;
    }
    if (key.upArrow) {
      providerSettings.setMode({
        ...mode,
        selectedIndex: 0,
      });
      return true;
    }
    if (key.downArrow) {
      providerSettings.setMode({
        ...mode,
        selectedIndex: 1,
      });
      return true;
    }
    if (key.return) {
      const choice = mode.selectedIndex === 0 ? 'github.com' : 'enterprise';
      void submitCopilotDeploymentType(providerSettings, choice).catch(() => {
        // submitCopilotDeploymentType already routes errors to oauth-error mode
      });
      return true;
    }
    // Absorb everything else while in this prompt
    return true;
  }

  // Enterprise URL text input
  if (mode.type === 'oauth-enterprise-url-entry') {
    if (key.escape) {
      providerSettings.cancelOauth();
      return true;
    }
    if (key.return) {
      if (mode.urlInput.length === 0) {
        providerSettings.setMode({
          ...mode,
          validationError: 'URL or domain is required',
        });
        return true;
      }
      void submitCopilotEnterpriseUrl(providerSettings, mode.urlInput).catch(
        () => {
          // submitCopilotEnterpriseUrl already routes errors to oauth-error mode
        }
      );
      return true;
    }
    if (key.backspace || key.delete) {
      if (mode.urlInput.length > 0) {
        providerSettings.setMode({
          ...mode,
          urlInput: mode.urlInput.slice(0, -1),
          validationError: undefined,
        });
      }
      return true;
    }
    // Append printable characters (ASCII 32-126)
    if (input && !key.ctrl && !key.meta) {
      const filtered = input
        .split('')
        .filter(ch => {
          const code = ch.charCodeAt(0);
          return code >= 32 && code <= 126;
        })
        .join('');
      if (filtered.length > 0) {
        providerSettings.setMode({
          ...mode,
          urlInput: mode.urlInput + filtered,
          validationError: undefined,
        });
      }
    }
    return true;
  }

  return false;
}
