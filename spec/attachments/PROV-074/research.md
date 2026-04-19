# PROV-074 — Render custom providers in ProviderSettingsScreen

## Problem

After PROV-073 makes the TS registry Rust-backed, `ProviderSettingsScreen` still needs layout rules for a new kind of row: a **custom provider** that has per-config metadata (facade, baseUrl, script path, origin badge) and that can be deleted/edited/validated in-place.

The current screen is hard-coded around two patterns:

- **API-key providers** (16 of 17): one "API Key" row that supports edit/delete.
- **OAuth providers** (3: anthropic, codex, github-copilot): "Logout" status + login button(s).
- **OpenAI**: additionally shows profile rows + "Create Profile".

Custom providers don't fit any of these exactly — they may be facade-backed (no Rhai), may carry OAuth via Rhai script, may define multiple models in-config, and need actions the screen has never supported: **Validate**, **Test Connection** (with model listing), **Open in editor**, **Delete**, **Duplicate to project scope**.

## Current navigation model

`SettingsNavItem` union (`ProviderSettingsPanel.tsx:120-135`):

```ts
| { type: 'provider'; providerId; name }
| { type: 'profile'; providerId; profileName }
| { type: 'add-profile'; providerId }
| { type: 'api-key'; providerId }
| { type: 'oauth-login'; providerId; method; label }
| { type: 'oauth-status'; providerId; label }
```

`buildNavItems` (`useProviderSettingsState.ts:132-206`) synthesizes these from `ProviderDisplayInfo[]` + `filter`.

## New variants

Extend `SettingsNavItem`:

```ts
| { type: 'custom-provider-meta'; providerId }      // displays facade/baseUrl/origin/configPath
| { type: 'custom-provider-model'; providerId; alias }  // per-model row showing id/ctx/max-out
| { type: 'custom-provider-validate'; providerId }  // triggers validateProvider()
| { type: 'custom-provider-test'; providerId }      // triggers testProvider()
| { type: 'custom-provider-script'; providerId }    // opens .rhai source (read-only preview)
| { type: 'custom-provider-delete'; providerId }    // danger action
```

An "api-key" row is still used when `authType === 'bearer' | 'api-key-header'`. An `oauth-status` / `oauth-login` row is still used when `authType === 'oauth-*'`.

## Extended expansion shape for custom providers

When `provider.isExpanded && provider.isCustom`:

```
▼ my-provider               [custom, global]
    facade: openai  base: https://api.example.com/v1
    script: ~/.fspec/providers/my-provider.rhai      <-- if present
    API Key: MY_PROVIDER_API_KEY (set via env)
    ────────────────────────────────────────
    Models:
      ▸ default (id=default, ctx=128000, maxOut=4096)
      ▸ fast (id=lite-v1, ctx=32000, maxOut=2048)
    ────────────────────────────────────────
    [ Validate ]  [ Test connection ]  [ View script ]  [ Delete ]
```

Origin badge: `[global]` vs `[project]` pulled from `JsProviderInfo.origin` (PROV-072).

When a provider is invalid (surfaced via `listProviderFiles()`, see PROV-072) the row renders with `⚠ invalid: <reason>` and offers only `[ Open file ]` / `[ Delete ]` actions — it never contributes models or connection-test rows.

## Action dispatch

The `useProviderSettingsInput` keyboard hook currently dispatches to functions on `useProviderSettingsState`. New actions:

```ts
validateCustomProvider(id): Promise<{ valid: boolean; error?: string }>;
testCustomProvider(id): Promise<JsProviderTestResult>;
readCustomProviderScript(id): Promise<string | undefined>;  // opens inline preview mode
deleteCustomProvider(id, scope: 'global' | 'project' | 'both'): Promise<void>;
```

All four wrap NAPI calls from PROV-072. `deleteCustomProvider` must:
1. Confirm via a modal-style overlay (reuse existing confirmation UI pattern).
2. Call `deleteProvider(id, scope)`.
3. Call `invalidateProviderRegistry()` (PROV-073).
4. Reload.

## Script preview mode

Add a new `HookMode` variant:

```ts
| { type: 'custom-provider-script-view'; providerId; content: string }
```

Renders the Rhai source in a read-only scrolling view (reuse `CheckpointViewer`-style scrolling component from PROV-010).

## Empty state CTA

If `providers.length > 0` but no custom providers exist, render a terminal-bottom hint:

```
Tip: create your own provider with  /provider init <name>
(scripts live at ~/.fspec/providers/<name>.json)
```

## Test plan

- Render a custom provider with `facade = "openai"` and verify rows: meta, api-key, 2 models, validate/test/delete.
- Render an **invalid** custom provider and verify only `⚠ invalid` + `[ Open file ]` + `[ Delete ]` are available.
- Trigger `validate` action — confirm `validateProvider` NAPI called and error surfaced on failure.
- Trigger `test` action — confirm `testProvider` NAPI called and `matchedModels` / `statusCode` / `reachable` rendered.
- Trigger `delete` action — confirm confirmation prompt, NAPI call, cache invalidation, reload.
- Regression: 17 built-in providers still render identically to today.

## Acceptance summary

- Custom providers appear in `/provider` with full metadata (facade, baseUrl, origin, configPath, scriptPath).
- Expansion reveals model list + 4 action rows (Validate, Test, Script, Delete).
- Invalid configs render a degraded row with open-file + delete actions only.
- Actions are wired to NAPI and invalidate the Rust-backed registry on mutation.
- Confirmation modal prevents accidental deletion.

## Dependencies

- PROV-072 (NAPI enrichment + delete + read-script + list-files)
- PROV-073 (Rust-backed registry + invalidation hook)

## References

- `src/tui/components/ProviderSettingsPanel.tsx:120-135` (SettingsNavItem)
- `src/tui/hooks/useProviderSettingsState.ts:132-206` (buildNavItems)
- `src/tui/hooks/useProviderSettingsInput.ts` (keyboard dispatch)
- `src/tui/utils/providerSettingsModeMapper.ts` (mode mapping helper)
- `src/tui/components/ProviderSettingsScreen.tsx:41-101`
