# PROV-073 — Replace hard-coded TS provider registry with NAPI-sourced registry

## Problem

`src/utils/provider-registry.ts` contains a **static array of 17 provider entries** that drives the entire Provider Settings Screen. Custom Rhai providers cannot appear because every TUI consumer iterates this hard-coded list via `getProviderRegistry()` / `getProviderRegistryEntry()` / `isOAuthProvider()`.

The fix is architectural: **Rust must become the single source of truth** for provider metadata. TypeScript should be a pure view.

## Current state

### Hard-coded constants

`src/utils/provider-registry.ts:18-36`:

```ts
export const SUPPORTED_PROVIDERS = [
  'openai', 'anthropic', 'cohere', 'gemini', 'mistral', 'xai',
  'together', 'huggingface', 'openrouter', 'groq', 'deepseek',
  'moonshot', 'galadriel', 'azure', 'zai', 'codex', 'github-copilot',
] as const;

export type ProviderId = (typeof SUPPORTED_PROVIDERS)[number];

const PROVIDER_REGISTRY: ProviderRegistryEntry[] = [ /* 17 static entries */ ];
```

### Consumer surface (must all be updated)

| Consumer | Location | Uses |
|---|---|---|
| `useProviderSettingsState` reload | `src/tui/hooks/useProviderSettingsState.ts:251-356` | `getProviderRegistry()`, `getProviderRegistryEntry(id)`, `isOAuthProvider(id)` |
| `buildNavItems` | `src/tui/hooks/useProviderSettingsState.ts:132-206` | `getProviderRegistryEntry`, `isOAuthProvider` |
| `ProviderSettingsPanel` rendering | `src/tui/components/ProviderSettingsPanel.tsx:13` | `getProviderRegistryEntry` |
| `isProviderConfigured` | `src/utils/provider-config.ts:198` | Static list |
| `getAllProvidersWithStatus` | `src/utils/provider-config.ts:233` | Static list |
| `AgentView` provider selector | `src/tui/components/AgentView.tsx:4934-4977` | `availableProviders: string[]` (from `initResult`) |
| `mapProviderIdToInternal` | `src/tui/components/AgentView.tsx:193` | Hard-coded map |
| OAuth provider labels | `src/tui/utils/oauthProviderLabels.ts` | Hard-coded table |
| OAuth login nav items | `src/tui/utils/oauthLoginLabels.ts` | Hard-coded per-provider login methods |

## Target design

### New module: `src/utils/providerRegistry.ts` (Rust-backed)

Replace all hard-coded exports with NAPI-backed ones. The registry is cached per-process but **invalidated** when `rediscoverProviders()` runs (PROV-072).

```ts
import { listProviders, fspecPaths } from '@sengac/codelet-napi';
import type { JsProviderInfo } from '@sengac/codelet-napi';

/** Runtime-loaded equivalent of the old static PROVIDER_REGISTRY. */
let cache: ProviderRegistryEntry[] | null = null;

export interface ProviderRegistryEntry {
  id: string;
  name: string;
  baseUrl: string;
  envVar: string;
  authMethod: AuthMethod;
  authType: AuthType;           // 'api-key' | 'oauth'
  requiresApiKey: boolean;
  description: string;
  // NEW — absent for built-ins:
  isCustom: boolean;
  facade?: string;
  configPath?: string;
  scriptPath?: string;
  origin?: 'global' | 'project';
}

export async function loadProviderRegistry(options?: {
  force?: boolean;
}): Promise<ProviderRegistryEntry[]> {
  if (!options?.force && cache) return cache;
  const napi = await listProviders();
  cache = napi.map(napiToEntry);
  return cache;
}

export function invalidateProviderRegistry(): void { cache = null; }

export async function getProviderRegistry(): Promise<string[]> {
  return (await loadProviderRegistry()).map(e => e.id);
}

export async function getProviderRegistryEntry(
  providerId: string,
): Promise<ProviderRegistryEntry | undefined> {
  return (await loadProviderRegistry()).find(e => e.id === providerId);
}

export async function isOAuthProvider(providerId: string): Promise<boolean> {
  const entry = await getProviderRegistryEntry(providerId);
  return entry?.authType === 'oauth';
}
```

### Async migration

The existing `getProviderRegistry()` / `getProviderRegistryEntry()` / `isOAuthProvider()` are **synchronous**. Making them async ripples into every consumer. Two options:

- **Option A (recommended)**: Convert to async. `useProviderSettingsState.reload()` is already async. `buildNavItems` is pure and runs inside a `useMemo` over `providers` state — no change needed because entries are now embedded in `ProviderDisplayInfo`.
- Option B: Pre-warm cache at session start and keep sync accessors that throw if the cache is cold. More fragile.

We choose Option A.

### Ripple changes

1. **`useProviderSettingsState.reload()`** — replace `getProviderRegistry()` call with `await loadProviderRegistry()`. Include the full entry on `ProviderDisplayInfo` so `buildNavItems` never needs a sync lookup.
2. **`buildNavItems`** — accept `provider.authType` / `provider.requiresApiKey` / `provider.envVar` directly off the `ProviderDisplayInfo` instead of calling `getProviderRegistryEntry` inline. Pure-function invariant preserved.
3. **`ProviderSettingsPanel.tsx`** — remove the `getProviderRegistryEntry` import, read fields from `ProviderDisplayInfo`.
4. **`isProviderConfigured` / `getAllProvidersWithStatus`** — convert to async.
5. **`AgentView` provider selector** — the `availableProviders: string[]` already comes from the session init result (server-side); extend that list to include discovered custom providers via the same NAPI.
6. **`mapProviderIdToInternal`** (`AgentView.tsx:193`) — delete. Providers now map 1:1 to their canonical slug across the codebase. Built-ins keep their current slugs; custom providers use their declared `name`.
7. **OAuth labels** — move per-provider label/login-method data into Rust (`ProviderConfig` already has `auth: AuthConfig`). Expose via `JsProviderInfo.authType`. The TS `oauthProviderLabels.ts` table becomes a 6-entry built-in fallback; custom OAuth providers get their labels from the Rust config.

### Cache invalidation triggers

`invalidateProviderRegistry()` must be called whenever:
- The user runs `/provider init ...` or `/provider delete ...` slash commands (PROV-076).
- The `rediscoverProviders()` NAPI reports changes.
- The session settings screen exits after an OAuth flow has completed.

A single `useEffect` hook at app mount will subscribe to a new `providerRegistryInvalidated` event bus channel so components can trigger reloads without prop drilling.

## What remains static

OAuth flows themselves are still provider-specific (codex device auth vs anthropic headless vs github-copilot device) and live in `useProviderSettingsState`'s `startBrowserLogin` / `startDeviceLogin`. These **cannot** be fully data-driven until Rhai scripts take ownership of OAuth (PROV-060 foundation work). Leave as-is with explicit `providerId` branches.

## Test plan

- Unit test: `loadProviderRegistry` returns a merged list of 17 built-ins + N discovered custom when NAPI is mocked.
- Integration test: with an `FSPEC_HOME` pointed at a tmpdir containing one custom provider JSON, `getProviderRegistry()` includes it.
- UI test: `useProviderSettingsState.reload` returns a `ProviderDisplayInfo` for the custom provider with populated `facade`/`models`/`configPath`.
- Regression: all 17 built-ins still render identically (status source, mask key, OAuth rows, profile rows for `openai`).

## Acceptance summary

- `src/utils/provider-registry.ts` either re-exports from the new module or is deleted.
- No file in `src/` imports `SUPPORTED_PROVIDERS` as a const.
- `getProviderRegistry` / `getProviderRegistryEntry` / `isOAuthProvider` are async.
- A tmpdir-backed test showing a custom provider JSON file appearing in `getProviderRegistry()`.

## Dependencies

- PROV-071 — shared `fspec_home()` helper
- PROV-072 — enriched `JsProviderInfo` + `rediscoverProviders()`

## References

- `src/utils/provider-registry.ts:18,38,43-217,222-240`
- `src/utils/provider-config.ts:124-133,198,233`
- `src/tui/hooks/useProviderSettingsState.ts:14-18,132-206,251-356`
- `src/tui/components/ProviderSettingsPanel.tsx:13`
- `src/tui/components/AgentView.tsx:193,1465,4934-4977`
- `src/tui/utils/oauthProviderLabels.ts`
- `src/tui/utils/oauthLoginLabels.ts`
