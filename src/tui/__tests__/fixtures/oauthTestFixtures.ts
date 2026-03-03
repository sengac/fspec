/**
 * Shared test fixtures for OAuth TUI tests
 *
 * Provides reusable builders for Ink key objects and ProviderDisplayInfo arrays.
 * Used by oauth-tui-broken-flows.test.ts and anthropic-oauth-tui.test.ts.
 *
 * SOLID: Single responsibility — test data builders only.
 * DRY: One place for shared test helpers instead of duplicating across files.
 */

import type { Key } from 'ink';
import type {
  ProviderDisplayInfo,
  PanelMode,
} from '../../components/ProviderSettingsPanel';

/**
 * Build a Key object with all flags defaulted to false.
 * Override specific flags as needed.
 */
export function buildKey(overrides: Partial<Key> = {}): Key {
  return {
    upArrow: false,
    downArrow: false,
    leftArrow: false,
    rightArrow: false,
    pageDown: false,
    pageUp: false,
    return: false,
    escape: false,
    ctrl: false,
    shift: false,
    tab: false,
    backspace: false,
    delete: false,
    meta: false,
    ...overrides,
  };
}

/**
 * Build a ProviderDisplayInfo fixture with sensible defaults.
 * Override specific fields as needed.
 */
export function buildProvider(
  overrides: Partial<ProviderDisplayInfo> &
    Pick<ProviderDisplayInfo, 'id' | 'name'>
): ProviderDisplayInfo {
  return {
    status: { hasKey: false },
    profiles: [],
    isExpanded: false,
    hasOAuthTokens: false,
    ...overrides,
  };
}

/**
 * Pre-built provider fixtures for common test scenarios
 */
export const PROVIDER_FIXTURES = {
  /** Codex with OAuth tokens and expanded */
  codexWithTokensExpanded: (): ProviderDisplayInfo =>
    buildProvider({
      id: 'codex',
      name: 'Codex (ChatGPT)',
      status: { hasKey: true, maskedKey: 'OAuth', source: 'ChatGPT' },
      isExpanded: true,
      hasOAuthTokens: true,
    }),

  /** Codex with OAuth tokens, collapsed */
  codexWithTokensCollapsed: (): ProviderDisplayInfo =>
    buildProvider({
      id: 'codex',
      name: 'Codex (ChatGPT)',
      status: { hasKey: true, maskedKey: 'OAuth', source: 'ChatGPT' },
      isExpanded: false,
      hasOAuthTokens: true,
    }),

  /** Anthropic with OAuth tokens, expanded */
  anthropicWithTokensExpanded: (): ProviderDisplayInfo =>
    buildProvider({
      id: 'anthropic',
      name: 'Anthropic',
      status: { hasKey: true, maskedKey: 'OAuth', source: 'Claude' },
      isExpanded: true,
      hasOAuthTokens: true,
    }),

  /** Anthropic with no tokens, expanded */
  anthropicNoTokensExpanded: (): ProviderDisplayInfo =>
    buildProvider({
      id: 'anthropic',
      name: 'Anthropic',
      status: { hasKey: false },
      isExpanded: true,
      hasOAuthTokens: false,
    }),

  /** Non-OAuth provider (e.g. OpenAI) expanded */
  openaiExpanded: (): ProviderDisplayInfo =>
    buildProvider({
      id: 'openai',
      name: 'OpenAI',
      status: { hasKey: false },
      isExpanded: true,
      hasOAuthTokens: false,
    }),
} as const;

/**
 * Pre-built PanelMode fixtures for headless code entry
 */
export const MODE_FIXTURES = {
  headlessCodeEntry: (
    overrides: Partial<{
      authorizeUrl: string;
      pkceVerifier: string;
      codeInput: string;
    }> = {}
  ): PanelMode => ({
    type: 'oauth-headless-code-entry' as const,
    providerId: 'anthropic',
    authorizeUrl:
      overrides.authorizeUrl ??
      'https://claude.ai/oauth/authorize?client_id=test&state=abc',
    pkceVerifier: overrides.pkceVerifier ?? 'test-verifier',
    codeInput: overrides.codeInput ?? '',
  }),

  oauthError: (error: string): PanelMode => ({
    type: 'oauth-error' as const,
    providerId: 'anthropic',
    error,
  }),

  browserWaiting: (): PanelMode => ({
    type: 'oauth-browser-waiting' as const,
    providerId: 'anthropic',
  }),
} as const;
