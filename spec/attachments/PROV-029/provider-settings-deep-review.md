# Provider Settings TUI — Deep Review Findings

## Overview

Deep code review of the entire Provider Settings TUI subsystem, covering:
- `ProviderSettingsPanel.tsx` (presentation)
- `ProviderSettingsScreen.tsx` (orchestrator)
- `useProviderSettingsState.ts` (active state hook)
- `useProviderSettingsInput.ts` (keyboard routing)
- `listModeHandler.ts` (list actions)
- `oauthModeHandler.ts` (OAuth flow input)
- `apiKeyEditModeHandler.ts`, `profileFormModeHandler.ts`, `filterModeHandler.ts`, `deleteConfirmModeHandler.ts`
- `providerSettingsHelpers.ts`, `providerSettingsModeMapper.ts`
- `ProviderSettingsView.tsx` (legacy view) + `useProviderProfiles.ts` (legacy hook)
- `provider-config.ts` (registry + profile CRUD)
- `credentials.ts` (API key management)
- `provider.ts` (types)
- `providerSettings.ts` (constants)

---

## 🔴 Critical Issues

### 1. PROV-029 (already filed): Profiles display for OAuth providers

See PROV-029 card. Stale profile shown under Anthropic, missing guards at 5 locations.

---

### 2. Legacy `ProviderSettingsView.tsx` is dead code — never imported, diverged, and dangerously wrong

**`ProviderSettingsView.tsx`** (880 lines) is not imported by anything. `ProviderSettingsScreen.tsx` is the active code path, which uses `ProviderSettingsPanel.tsx` + `useProviderSettingsState.ts`. Yet the legacy view still exists, accumulates stale fixes, and diverges:

**Divergences from the active path:**
- **No OAuth mode rendering** — no browser-waiting, device-waiting, headless-code-entry, success, or error screens. Legacy view has zero OAuth UI.
- **No `oauth-login` or `oauth-status` nav items** — `navItems` in the legacy view only knows `provider`, `profile`, `add-profile`. No OAuth items.
- **No OAuth-guarded "Create Profile"** — legacy view always adds the `add-profile` item for every expanded provider (line 161-164). The active path guards with `!isOAuthProvider()`.
- **Calls `codexOauthGetTokens()` for ALL OAuth providers** — `useProviderProfiles.ts` line 137 calls `codexOauthGetTokens()` even for Anthropic (which should call `claudeOauthGetTokens()`). The active path in `useProviderSettingsState.ts` correctly branches on `providerId === 'anthropic'`.
- **`disconnectOauth()` calls `codexOauthClearTokens()` for ALL OAuth providers** — `useProviderProfiles.ts` line 377. Should branch for Anthropic vs Codex. The active path does this correctly.
- **`startBrowserLogin()` is a no-op stub** — `useProviderProfiles.ts` line 364-368 just logs a warning. Yet the legacy view's `'e'` handler for OAuth providers calls it (line 495).

**Recommendation:** Delete `ProviderSettingsView.tsx` and `useProviderProfiles.ts` entirely. They are unused, misleading, and their OAuth support is broken.

---

### 3. `'n'` key creates profiles for ANY provider — including OAuth providers

In `listModeHandler.ts` line 166-168:
```typescript
if (input === 'n' || input === 'N') {
    initializeNewProfile(providerSettings, currentItem.providerId);
    return;
}
```

This creates a new profile form for whatever provider the cursor is currently on — including Anthropic and Codex. There's no `isOAuthProvider()` check. If the user presses `n` while the cursor is on the `✓ OAuth [Claude]` status row, the `🔑 Login with Claude (browser)` row, or the provider header row, it opens the profile creation form.

The `buildNavItems()` function correctly hides the `+ Create new profile` button for OAuth providers, but the `n` keybind bypasses that entirely.

`initializeNewProfile()` in `providerSettingsHelpers.ts` also has no guard.

---

### 4. `'d'` on API-key provider deletes key with NO confirmation

In `listModeHandler.ts` lines 186-191:
```typescript
} else if (
  currentItem.type === 'provider' &&
  currentProvider?.status.hasKey
) {
  void providerSettings.removeApiKey(currentItem.providerId);
}
```

Pressing `d` on a provider with an API key (e.g., OpenAI, Google Gemini) **immediately and silently deletes the API key** with zero confirmation. Compare to profile deletion which shows a `y/n` confirmation dialog. API keys are arguably MORE important to confirm than profiles — losing your API key means re-entering it.

Same issue in `ProviderSettingsView.tsx` line 529-530 (legacy, but same pattern).

The OAuth disconnect (line 185) also has no confirmation, though that's less severe since OAuth can be re-authenticated via browser.

---

### 5. `ProviderDisplayStatus.source` type mismatch between declaration and usage

In `ProviderSettingsPanel.tsx` line 21:
```typescript
source?: 'env' | 'file' | 'dotenv' | 'ChatGPT' | 'Claude';
```

But in `credentials.ts` `ProviderConfigResult` line 58:
```typescript
source?: 'explicit' | 'file' | 'env' | 'dotenv';
```

And in `useProviderSettingsState.ts` line 264, the source from `getProviderConfig()` is cast with `as`:
```typescript
source: providerConfig.source as 'env' | 'file' | 'dotenv' | undefined,
```

The cast silently discards `'explicit'` which is a valid return from `getProviderConfig()`. If a credential source is `'explicit'`, it'll be set but the display type narrowing will lie about it.

Then for OAuth, the source is hardcoded to `'Claude'` or `'ChatGPT'` (lines 283-284, 290-291), which are strings that only exist in the Panel's type but not in credentials. These are display-only strings being shoved into a field that's supposed to represent credential source.

The legacy `useProviderProfiles.ts` has yet another `source` type: `'env' | 'config' | 'profile'` (from `provider.ts` line 112). Three different type definitions for the same concept.

---

## 🟡 Medium Issues

### 6. `edit-api-key` mode doesn't pass `currentValue` from the hook mode

In `providerSettingsModeMapper.ts` lines 38-44:
```typescript
if (hookMode.type === 'edit-api-key') {
    return {
      type: 'edit-api-key',
      providerId: hookMode.providerId,
      currentValue: providerSettings.editingApiKey,
    };
  }
```

The `currentValue` is set to `providerSettings.editingApiKey` — which is `''` (empty string) on entry because `listModeHandler.ts` line 148 does `providerSettings.setEditingApiKey('')` before setting the mode. The panel renders this as an empty masked field (line 205: `'•'.repeat(mode.currentValue.length)` = nothing). 

The user sees a blank field with no indication of whether a key currently exists. They have to type a new key from scratch. There's no way to see the masked current key and decide "I don't want to change it." This is a UX issue — the edit mode should pre-populate with the current masked value or at least indicate a key exists.

---

### 7. Duplicate `maskApiKey()` implementations

There are two independent implementations:
1. `credentials.ts` lines 259-273 — sophisticated, prefix-aware (`sk-ant-`, `sk-`, `gsk_`, `AIza`, `xai-`)
2. `useProviderProfiles.ts` lines 407-414 — simple, just first 4 + last 4 chars

`useProviderSettingsState.ts` imports from `credentials.ts` (correct).
`useProviderProfiles.ts` has its own local copy (diverged).

This means the same API key would be masked differently depending on which code path renders it. Inconsistent UX.

---

### 8. Duplicate type definitions across active and legacy systems

Three separate type systems that describe the same concepts:

| Concept | Active (`ProviderSettingsPanel.tsx`) | Legacy (`provider.ts`) |
|---------|-------------------------------------|------------------------|
| Provider display | `ProviderDisplayInfo` | `ProviderWithProfiles` |
| Profile display | `ProfileDisplayInfo` | `ProfileDisplay` |
| Status | `ProviderDisplayStatus` | `ProviderStatus` |
| Test result | `TestResult` | `ConnectionTestResult` |
| Nav item | `SettingsNavItem` | `NavItem` (inline) |
| Panel mode | `PanelMode` | `SettingsViewMode` |
| Form field | `FIELD_LABELS` (inline array) | `PROFILE_FORM_FIELDS` (typed array in provider.ts) |

Plus `providerSettings.ts` constants has its own `PROFILE_FORM_FIELDS` array.

Three different form field definitions:
1. `ProviderSettingsPanel.tsx` line 137 — `FIELD_LABELS`
2. `provider.ts` line 160 — `PROFILE_FORM_FIELDS`
3. `providerSettings.ts` line 12 — `PROFILE_FORM_FIELDS` (just keys, no labels)

---

### 9. `'e'` on OAuth provider auto-starts browser login — confusing UX

In `listModeHandler.ts` line 144-146:
```typescript
if (isOAuthProvider(currentItem.providerId)) {
    providerSettings.startBrowserLogin(currentItem.providerId);
}
```

When user presses `e` (edit) on an OAuth provider expecting to see some config screen, it silently launches a browser OAuth flow. The user probably just wanted to see the provider's settings, not immediately start authentication. This is the same action as pressing Enter on `🔑 Login with Claude (browser)`. The `e` key should do nothing on OAuth providers, or expand them.

---

### 10. No action when Enter/e/d/t pressed on `oauth-status` item

The `oauth-status` nav item type was added in PROV-028 but `handleActions()` in `listModeHandler.ts` has no handler for it:
- `Enter` on `oauth-status`: falls through all conditions, does nothing
- `e` on `oauth-status`: falls through (not a provider, not a profile)
- `d` on `oauth-status`: falls through
- `t` on `oauth-status`: falls through

The status row is selectable but completely inert. Pressing `d` on it should probably offer to disconnect OAuth. Pressing `t` should test the connection. Pressing Enter could expand to show token details.

---

### 11. Profile form placeholder baseUrl is `http://localhost:8888` — same as OAuth form server

Three locations hardcode `http://localhost:8888` as the default profile baseUrl:
1. `providerSettings.ts` line 22: `DEFAULT_PROFILE_BASE_URL = 'http://localhost:8888'`
2. `ProviderSettingsPanel.tsx` line 274: placeholder text
3. `ProviderSettingsView.tsx` line 231: form initialization
4. `provider.ts` line 166: placeholder text

Port 8888 is the port used by `claude_oauth_server.rs` for the local OAuth form server. This creates confusion — the Anthropic stale profile bug (PROV-029) has `baseUrl: http://localhost:8888` which may have been created by someone using the default. A more common default like `http://localhost:11434` (Ollama) or just empty would be less confusing.

---

### 12. `PROVIDER_ENV_VARS` in credentials.ts is missing `codex`

`credentials.ts` line 65 has the `PROVIDER_ENV_VARS` map covering all providers, but `codex` is not listed. The provider registry in `provider-config.ts` lists `CODEX_API_KEY` as the env var for codex (line 313), but `credentials.ts` won't check for it in `getProviderConfig()`. This means codex API keys set via environment variables won't be detected by the credentials resolution chain.

---

## 🔵 Low / Informational

### 13. Header shows "(25 items)" which counts sub-items, not providers

The screenshot shows `Provider Settings (25 items)`. There are 21 providers but Anthropic is expanded with 4 sub-items (oauth-status, browser login, headless login, profile), making it 24... or 25 with the profile. The count mixes providers with sub-items, which is confusing. It should either say "(21 providers)" or not show a count at all.

### 14. No scrollbar in legacy `ProviderSettingsView.tsx`

The active `ProviderSettingsPanel.tsx` has a scrollbar (lines 646-666). The legacy view has none (just a flat column). Dead code, so only matters if someone ever resurrects it.

### 15. `ProviderSettingsView.tsx` uses inline `NavItem` type (line 42-46)

The legacy view defines its own `NavItem` type locally, which lacks `oauth-login` and `oauth-status` variants. This makes the legacy view TypeScript-incompatible with the PROV-028 fixes if anyone tried to merge them in.
