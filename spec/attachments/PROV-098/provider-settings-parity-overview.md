# Provider Settings Parity — Research Overview

Two screenshots compared the original TypeScript Provider Settings screen
(`~/working-typescript.png`) against the Rust TUI port (`~/broken-rust.png`).

## Observed differences

| Aspect | TypeScript (working) | Rust (broken) |
|---|---|---|
| Provider status | `Anthropic ✓ sk-ant-•••••••SQAA [env]`, `Google Gemini ✓ AIza…H3Ck [env]`, `Z.AI ✓ 5fc6d5…NHC7 [env]` | bare provider names, **no** `✓`, **no** masked key, **no** `[env]` |
| Unconfigured | `Cohere (not configured)` etc. | **no** `(not configured)` annotation |
| OpenAI profiles | `fireworks → https://api.fireworks.ai/inference` | only `+ Add Profile` (fireworks missing) |
| Anthropic | masked env API key shown | wrongly shows `Logout from OAuth [Anthropic]` |

## Four distinct root causes (one bug card each)

1. **PROV-097** — The `fspec` binary never loads `.env` before the TUI reads env vars.
2. **PROV-098** — The rich nav-tree render drops `masked_key`/`source`, so no `✓ … [env]` / `(not configured)`.
3. **PROV-099** — Anthropic's API key is never masked (classed OAuth) and wrongly shows "Logout from OAuth".
4. **PROV-100** — Custom OpenAI profiles are never loaded from `~/.fspec/fspec-config.json`.

## Data flow (TS reference)

```
useProviderSettingsState.reload()      src/tui/hooks/useProviderSettingsState.ts
   ├─ getProviderRegistry()            src/utils/provider-registry.ts
   ├─ getProviderConfig(providerId)    src/utils/credentials.ts   → { apiKey, source }
   ├─ maskApiKey(apiKey)               src/utils/credentials.ts
   └─ loadProviderProfiles('openai')   src/utils/profile-management.ts
            ↓
   ProviderSettingsPanel.tsx renders:  ✓ {maskedKey} [{source}]
```

## Data flow (Rust port)

```
App::handle_open_provider_settings_view        fspec-tui/src/app/dispatch_provider_settings.rs
   → backend.list_provider_credentials()       sessions/src/handle_impl.rs:1163
       → codelet_providers::custom::list_providers_info()   providers/src/custom/management.rs:110
   → handle_provider_credentials_loaded()
       → project_display_infos(&list, &[])      fspec-tui/src/views/provider_settings/projection.rs
       → set_provider_display_infos()           rebuilds nav_items
            ↓
   list_nav_render.rs::row_kind_and_label()     renders provider rows
```

## Secondary findings (not separate cards, fold into PROV-098/099)

- `SOURCE_FILE` / `SOURCE_DOTENV` constants exist in `providers/src/credentials.rs`
  but `list_providers_info` only ever emits `SOURCE_ENV`. It never tags credentials
  read from `~/.fspec/credentials/credentials.json` or from `.env`.
- The credentials file (`credentials.json`) is not consulted for masking in
  `list_providers_info`. TS resolves file → env → dotenv (priority order).
