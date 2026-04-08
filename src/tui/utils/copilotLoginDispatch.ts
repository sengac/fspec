/**
 * Copilot login dispatch helper (PROV-057).
 *
 * Pure functions that decide whether AgentView's `handleModelSelect` should
 * dispatch the GitHub Copilot OAuth login flow instead of running the normal
 * `selectModel` path.
 *
 * Decision rule:
 *   - The selected provider is `github-copilot` AND
 *   - There is no `github-copilot` section with `hasCredentials: true`
 *
 * The dispatcher is intentionally split from `startCopilotLogin` itself
 * (which lives in `copilotLoginFlow.ts`) so AgentView does NOT need to own a
 * `useProviderSettingsState()` hook instance — it just supplies a callback
 * that takes the provider id and ultimately calls `startCopilotLogin`.
 */

import type { ProviderSection, ModelSelection } from '../types/provider';

const COPILOT_PROVIDER_ID = 'github-copilot';

/**
 * Returns true when the user has selected a github-copilot model AND no
 * credentials are currently registered for github-copilot in the provider
 * sections snapshot.
 *
 * Missing-section is treated as "no credentials" — we still want to launch
 * the login flow rather than silently fall through to a "requires
 * credentials" error.
 */
export function shouldDispatchCopilotLogin(
  providerSections: ProviderSection[],
  selection: ModelSelection
): boolean {
  if (selection.providerId !== COPILOT_PROVIDER_ID) {
    return false;
  }

  const section = providerSections.find(
    s => s.providerId === COPILOT_PROVIDER_ID
  );

  if (!section) {
    // Section absent → no credentials known → still need to launch login.
    return true;
  }

  return !section.hasCredentials;
}

/**
 * Conditionally invoke the copilot login callback.
 *
 * Returns `true` when the callback was dispatched (caller should NOT continue
 * with the normal selectModel flow), `false` otherwise.
 */
export function dispatchCopilotLoginIfNeeded(
  providerSections: ProviderSection[],
  selection: ModelSelection,
  startLogin: (providerId: string) => void
): boolean {
  if (!shouldDispatchCopilotLogin(providerSections, selection)) {
    return false;
  }

  startLogin(COPILOT_PROVIDER_ID);
  return true;
}
