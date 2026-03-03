/**
 * Settings mode types for provider settings TUI
 *
 * PROV-029: Separates hook-side mode (HookMode) from panel-side mode (PanelMode).
 *
 * HookMode = the internal state machine of the hook (what the user is DOING)
 * PanelMode = the rendering mode of the panel (what the user SEES)
 *
 * The mapper (providerSettingsModeMapper.ts) translates HookMode → PanelMode,
 * enriching with form state where needed.
 */

/**
 * Hook-side mode — all possible states the provider settings hook can be in.
 *
 * Used by: useProviderSettingsState, input handlers, mode mapper
 */
export type HookMode =
  | { type: 'list' }
  | { type: 'edit-api-key'; providerId: string }
  | { type: 'delete-api-key'; providerId: string }
  | { type: 'disconnect-oauth'; providerId: string }
  | { type: 'create-profile'; providerId: string }
  | { type: 'edit-profile'; providerId: string; profileName: string }
  | { type: 'delete-profile'; providerId: string; profileName: string }
  | { type: 'oauth-browser-waiting'; providerId: string }
  | {
      type: 'oauth-device-waiting';
      providerId: string;
      userCode: string;
      verificationUrl: string;
    }
  | { type: 'oauth-success'; providerId: string }
  | { type: 'oauth-error'; providerId: string; error: string }
  | {
      type: 'oauth-headless-code-entry';
      providerId: string;
      authorizeUrl: string;
      pkceVerifier: string;
      codeInput: string;
    };
