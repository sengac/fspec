/**
 * OAuth login label registry (PROV-054).
 *
 * Single source of truth mapping OAuth-capable provider IDs to the
 * user-facing labels and the set of supported login methods.
 *
 * Replaces the hard-coded `isAnthropic ? 'Login with Claude' : 'Login
 * with ChatGPT'` ternary that previously lived inside `buildNavItems`.
 * Adding a new OAuth provider now means adding a row here — no editing
 * of the nav-builder.
 *
 * SoC: this module owns ONLY label/method copy. It knows nothing about
 * navigation, mode transitions, or NAPI calls.
 */

import type { SettingsNavItem } from '../components/ProviderSettingsPanel';

/**
 * Discriminator used by the listModeHandler to decide whether Enter on
 * an oauth-login row launches the browser flow or the device/headless flow.
 */
export type OauthLoginMethod = 'browser' | 'headless';

/**
 * One concrete login row to render under an expanded OAuth provider.
 */
export interface OauthLoginEntry {
  method: OauthLoginMethod;
  label: string;
}

/**
 * Per-provider login entries. Keyed by `ProviderRegistryEntry.id`.
 *
 * - Anthropic exposes both browser (1455 server) and headless (paste code).
 * - Codex exposes both browser (1455 server) and device (poll user_code).
 * - GitHub Copilot is device-flow only — no browser variant exists.
 */
const OAUTH_LOGIN_REGISTRY: Record<string, OauthLoginEntry[]> = {
  anthropic: [
    { method: 'browser', label: 'Login with Claude (browser)' },
    { method: 'headless', label: 'Login with Claude (headless)' },
  ],
  codex: [
    { method: 'browser', label: 'Login with ChatGPT (browser)' },
    { method: 'headless', label: 'Login with ChatGPT (headless)' },
  ],
  'github-copilot': [
    { method: 'headless', label: 'Login with GitHub Copilot (device flow)' },
  ],
};

/**
 * Build the oauth-login nav items for a given OAuth-capable provider.
 *
 * Returns an empty array if the provider has no registered login entries
 * (e.g. a new OAuth provider was added to the registry but its labels
 * have not been wired up here yet — fail closed, do not show garbled
 * fallback labels).
 */
export function buildOauthLoginNavItems(providerId: string): SettingsNavItem[] {
  const entries = OAUTH_LOGIN_REGISTRY[providerId];
  if (!entries) {
    return [];
  }
  return entries.map(entry => ({
    type: 'oauth-login',
    providerId,
    method: entry.method,
    label: entry.label,
  }));
}
