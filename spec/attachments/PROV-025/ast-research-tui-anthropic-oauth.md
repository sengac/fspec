# AST Research: TUI Anthropic OAuth Integration (PROV-025)

## Overview

This card adds Claude/Anthropic OAuth login support to the TUI provider settings,
mirroring the existing Codex OAuth implementation (PROV-017) but with Claude-specific
differences (async token check, headless code entry instead of device polling).

## Key Files to Modify

### 1. provider-config.ts — Anthropic authType change
**File:** `src/utils/provider-config.ts`
**Line 123:** `authType: 'api-key'` → change to `authType: 'oauth'`

This makes `isOAuthProvider('anthropic')` return true. Affects all 8 call sites:
- `useProviderSettingsState.ts:185` (reload — token check)
- `useProviderSettingsState.ts:260` (navItems — OAuth login options)
- `useProviderSettingsState.ts:283` (navItems — hide Create Profile)
- `useProviderSettingsState.ts:557` (disconnectOauth)
- `listModeHandler.ts:144` ('e' key — startBrowserLogin vs edit-api-key)
- `listModeHandler.ts:182` ('d' key — disconnectOauth vs removeApiKey)
- `useProviderProfiles.ts:135` (profiles loading)
- `useProviderProfiles.ts:376` (profiles loading)

### 2. useProviderSettingsState.ts — Provider-specific OAuth dispatch
**File:** `src/tui/hooks/useProviderSettingsState.ts`

**NAPI imports needed (line ~29-33):**
```
claudeOauthBrowserLogin, claudeOauthHeadlessStart, claudeOauthHeadlessComplete,
claudeOauthGetTokens, claudeOauthClearTokens
```

**reload() (line 156-226):** Currently calls `codexOauthGetTokens()` (sync) for all OAuth providers.
Must dispatch: codex → sync `codexOauthGetTokens()`, anthropic → async `claudeOauthGetTokens()`.
Status labels: codex → `'ChatGPT'`, anthropic → `'Claude'`.

**startBrowserLogin() (line 455-480):** Currently calls `codexOauthBrowserLogin()`.
Must dispatch: codex → `codexOauthBrowserLogin()`, anthropic → `claudeOauthBrowserLogin()`.

**startDeviceLogin() (line 486-523):** Currently calls codex device flow.
For anthropic: calls `claudeOauthHeadlessStart()` (sync), transitions to new
`oauth-headless-code-entry` mode (not `oauth-device-waiting`).

**disconnectOauth() (line 554-566):** Currently calls `codexOauthClearTokens()` (sync).
Must dispatch: codex → sync `codexOauthClearTokens()`, anthropic → async `claudeOauthClearTokens()`.

**New operation needed:** `submitHeadlessCode(codeWithState, pkceVerifier)` that calls
`claudeOauthHeadlessComplete()` — used by the headless code entry input handler.

### 3. ProviderSettingsPanel.tsx — Provider-specific labels and headless entry
**File:** `src/tui/components/ProviderSettingsPanel.tsx`

**PanelMode union (line 58-89):** Add new variant:
```typescript
| { type: 'oauth-headless-code-entry'; providerId: string; authorizeUrl: string; pkceVerifier: string; codeInput: string }
```

**ProviderDisplayStatus.source (line 22):** Add `'Claude'` to union: `'env' | 'file' | 'dotenv' | 'ChatGPT' | 'Claude'`

**Rendering changes:**
- oauth-browser-waiting title: 'Codex OAuth Login' → provider-specific ('Claude OAuth Login' for anthropic)
- oauth-success: '✓ Connected to ChatGPT' → provider-specific ('✓ Connected to Claude' for anthropic)
- New render block for `oauth-headless-code-entry`: shows authorize URL + text input for code#state

**Nav item labels (in useProviderSettingsState navItems):**
- codex: 'Login with ChatGPT (browser)', 'Login with ChatGPT (headless)'
- anthropic: 'Login with Claude (browser)', 'Login with Claude (headless)'

### 4. oauthModeHandler.ts — Handle headless code entry
**File:** `src/tui/inputHandlers/oauthModeHandler.ts`

Add handling for `oauth-headless-code-entry` mode:
- Character input: append to codeInput
- Backspace: remove last character
- Enter: submit code via `submitHeadlessCode()`
- Escape: cancel flow

### 5. providerSettingsModeMapper.ts — Map headless code entry mode
**File:** `src/tui/utils/providerSettingsModeMapper.ts`

Add `oauth-headless-code-entry` to the pass-through list (line 47-54).

## NAPI Bindings (Confirmed Available)

After NAPI rebuild, index.d.ts exports:
- `claudeOauthBrowserLogin(): Promise<NapiClaudeTokens>`
- `claudeOauthHeadlessStart(): NapiClaudeHeadlessStartResult` (sync)
- `claudeOauthHeadlessComplete(codeWithState, pkceVerifier): Promise<NapiClaudeTokens>`
- `claudeOauthGetTokens(): Promise<NapiClaudeTokens | null>` (async — unlike codex sync)
- `claudeOauthClearTokens(): Promise<void>` (async — unlike codex sync)
- `claudeOauthRefreshToken(refreshToken): Promise<NapiClaudeTokens>`

## OpenCode Reference

OpenCode (dialog-provider.tsx) handles multiple auth methods per provider via a plugin system:
- `method.type === 'oauth'` with `authorize.method === 'code'` maps to our headless flow
- `authorize.method === 'auto'` maps to our browser flow
- The code entry uses `DialogPrompt` with a text input for the authorization code

Our approach follows the same pattern but with provider-specific NAPI dispatch instead of plugins.

## Key Difference: Codex vs Claude Headless

| Aspect | Codex (PROV-017) | Claude (PROV-025) |
|--------|-------------------|-------------------|
| Headless approach | Device auth (user_code + polling) | PKCE (authorize URL + code#state paste) |
| Token check | Sync `codexOauthGetTokens()` | Async `claudeOauthGetTokens()` |
| Token clear | Sync `codexOauthClearTokens()` | Async `claudeOauthClearTokens()` |
| New PanelMode | `oauth-device-waiting` | `oauth-headless-code-entry` |
| Status label | `'ChatGPT'` | `'Claude'` |
