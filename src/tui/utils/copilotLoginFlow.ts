/**
 * GitHub Copilot login flow orchestrator (PROV-054).
 *
 * Single-responsibility module that drives the multi-step Copilot OAuth
 * device flow from the TUI:
 *
 *   1. startCopilotLogin(ps, providerId)
 *      → transitions the hook into 'oauth-deployment-type-select'
 *
 *   2. submitCopilotDeploymentType(ps, deployment)
 *      → if 'github.com'  → calls copilotOauthDeviceLoginStart(null) →
 *        transitions to 'oauth-device-waiting' and begins polling
 *      → if 'enterprise' → transitions to 'oauth-enterprise-url-entry'
 *
 *   3. submitCopilotEnterpriseUrl(ps, url)
 *      → normalizes the URL via copilotNormalizeEnterpriseDomain →
 *        calls copilotOauthDeviceLoginStart(<host>) →
 *        transitions to 'oauth-device-waiting' and begins polling
 *
 * Polling delegates to copilotOauthDeviceLoginPoll, which persists the
 * credential to ~/.fspec/credentials/copilot_auth.json (mode 0600) on
 * success and returns the credential to the TS layer.
 *
 * SoC: this file owns ONLY the state-machine transitions for Copilot
 * login. It does not render UI, does not capture keyboard input, and does
 * not touch the filesystem directly. All IO goes through the NAPI bridge.
 */

import {
  copilotOauthDeviceLoginStart,
  copilotOauthDeviceLoginPoll,
  copilotNormalizeEnterpriseDomain,
} from '@sengac/codelet-napi';
import type { UseProviderSettingsStateReturn } from '../hooks/useProviderSettingsState';

/**
 * GitHub Copilot deployment options the user picks at login time.
 */
export type CopilotDeployment = 'github.com' | 'enterprise';

/**
 * Begin the Copilot login flow by entering the deployment-type selection
 * mode. Called when the user activates the 'Login with GitHub Copilot
 * (device flow)' row in the TUI.
 */
export function startCopilotLogin(
  ps: UseProviderSettingsStateReturn,
  providerId: string
): void {
  ps.setMode({
    type: 'oauth-deployment-type-select',
    providerId,
    selectedIndex: 0,
  });
}

/**
 * Drive the second step of the flow once the user has chosen a deployment.
 *
 * - For 'github.com' the device-code request is issued immediately against
 *   github.com and we transition to the polling state.
 * - For 'enterprise' we transition to the enterprise URL entry state and
 *   wait for the user to type a host before issuing the device-code request.
 */
export async function submitCopilotDeploymentType(
  ps: UseProviderSettingsStateReturn,
  deployment: CopilotDeployment
): Promise<void> {
  if (deployment === 'enterprise') {
    ps.setMode({
      type: 'oauth-enterprise-url-entry',
      providerId: 'github-copilot',
      urlInput: '',
    });
    return;
  }
  await beginCopilotDevicePolling(ps, null);
}

/**
 * Drive the third step of the flow for enterprise deployments. The raw
 * input is normalized via the Rust helper (so the TS and Rust sides agree
 * on what counts as a valid host) before issuing the device-code request.
 */
export async function submitCopilotEnterpriseUrl(
  ps: UseProviderSettingsStateReturn,
  rawUrl: string
): Promise<void> {
  const host = copilotNormalizeEnterpriseDomain(rawUrl);
  await beginCopilotDevicePolling(ps, host);
}

/**
 * Internal: issue the device-code request and transition into the polling
 * state. Shared by the github.com and enterprise paths to keep the flow
 * DRY.
 */
async function beginCopilotDevicePolling(
  ps: UseProviderSettingsStateReturn,
  enterpriseHost: string | null
): Promise<void> {
  try {
    const start = await copilotOauthDeviceLoginStart(enterpriseHost);
    ps.setMode({
      type: 'oauth-device-waiting',
      providerId: 'github-copilot',
      userCode: start.userCode,
      verificationUrl: start.verificationUrl,
    });
    // Fire-and-forget the polling promise so the TUI stays responsive.
    void (async () => {
      try {
        await copilotOauthDeviceLoginPoll(
          start.deviceCode,
          start.interval,
          start.hostUrl,
          start.enterpriseHost ?? null
        );
        ps.setMode({ type: 'oauth-success', providerId: 'github-copilot' });
        await ps.reload();
      } catch (err) {
        const message =
          err instanceof Error ? err.message : 'Copilot device login failed';
        ps.setMode({
          type: 'oauth-error',
          providerId: 'github-copilot',
          error: message,
        });
      }
    })();
  } catch (err) {
    const message =
      err instanceof Error
        ? err.message
        : 'Failed to start Copilot device login';
    ps.setMode({
      type: 'oauth-error',
      providerId: 'github-copilot',
      error: message,
    });
  }
}
