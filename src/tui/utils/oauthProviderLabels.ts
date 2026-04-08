/**
 * OAuth provider display label registry (PROV-054).
 *
 * Single source of truth for the user-facing names used to label OAuth
 * provider state in the TUI:
 *
 * - The "source" string shown next to a connected token (e.g. 'Claude',
 *   'ChatGPT', 'GitHub Copilot') — surfaced via `provider.status.source`.
 * - The "title" string used by OAuth confirmation/waiting/success screens
 *   (e.g. 'Claude OAuth Login', 'GitHub Copilot Device Login').
 * - The "disconnect" label asked of the user (e.g. 'Disconnect Claude
 *   OAuth?').
 *
 * Replaces hard-coded `mode.providerId === 'anthropic' ? 'X' : 'Y'`
 * binary ternaries that previously lived in ProviderSettingsPanel.tsx and
 * useProviderSettingsState.ts.
 *
 * SoC: this module owns ONLY display copy. It does not perform any
 * provider lookup beyond registry indexing.
 */

/**
 * Display labels surfaced by various OAuth-related TUI screens.
 */
export interface OauthProviderLabels {
  /** Source label shown in `provider.status.source` (e.g. 'Claude'). */
  source: string;
  /** Browser-OAuth waiting screen title (e.g. 'Claude OAuth Login'). */
  browserWaitingTitle: string;
  /** Device-flow waiting screen title (e.g. 'Codex Device Login'). */
  deviceWaitingTitle: string;
  /** Disconnect confirmation label (e.g. 'Disconnect Claude OAuth?'). */
  disconnectLabel: string;
  /** Success screen label (e.g. '✓ Connected to Claude'). */
  successLabel: string;
}

const REGISTRY: Record<string, OauthProviderLabels> = {
  anthropic: {
    source: 'Claude',
    browserWaitingTitle: 'Claude OAuth Login',
    deviceWaitingTitle: 'Claude Device Login',
    disconnectLabel: 'Disconnect Claude OAuth?',
    successLabel: '✓ Connected to Claude',
  },
  codex: {
    source: 'ChatGPT',
    browserWaitingTitle: 'Codex OAuth Login',
    deviceWaitingTitle: 'Codex Device Login',
    disconnectLabel: 'Disconnect ChatGPT OAuth?',
    successLabel: '✓ Connected to ChatGPT',
  },
  'github-copilot': {
    source: 'GitHub Copilot',
    browserWaitingTitle: 'GitHub Copilot OAuth Login',
    deviceWaitingTitle: 'GitHub Copilot Device Login',
    disconnectLabel: 'Disconnect GitHub Copilot OAuth?',
    successLabel: '✓ Connected to GitHub Copilot',
  },
};

/**
 * Default labels used when a provider id is not in the registry. These
 * intentionally fall back to neutral copy rather than guessing the wrong
 * brand name (the previous binary ternary defaulted everyone-not-anthropic
 * to 'ChatGPT').
 */
const FALLBACK: OauthProviderLabels = {
  source: 'OAuth',
  browserWaitingTitle: 'OAuth Login',
  deviceWaitingTitle: 'Device Login',
  disconnectLabel: 'Disconnect OAuth?',
  successLabel: '✓ Connected',
};

/**
 * Look up display labels for an OAuth provider id.
 *
 * Returns the FALLBACK labels for unknown ids — never throws.
 */
export function getOauthProviderLabels(
  providerId: string
): OauthProviderLabels {
  return REGISTRY[providerId] ?? FALLBACK;
}
