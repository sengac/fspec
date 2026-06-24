# PROV-099 — Anthropic api key never masked; wrongly shows "Logout from OAuth"

## Symptom
With `ANTHROPIC_API_KEY` set, the Rust screen shows `Anthropic` with a
`Logout from OAuth [Anthropic]` child row and NO masked key. The TS screen shows
`Anthropic ✓ sk-ant-•••••••SQAA [env]` plus an `API key ✓ … [env]` child row.

## Root cause (two interacting bugs)

1. **Masking gated on `AuthType::ApiKey` only.**
   `codelet/providers/src/catalog.rs`: anthropic has
   `env_var = "ANTHROPIC_API_KEY"` but `auth_type = AuthType::OAuth`.
   `management.rs::list_providers_info` (`:145-158`) derives `masked_key`/`source`
   ONLY for `AuthType::ApiKey`:
   ```rust
   let (masked_key, source) = match entry.auth_type {
       AuthType::ApiKey if available && !entry.env_var.is_empty() => { ...mask... }
       _ => (None, None),   // ← OAuth (anthropic) lands here
   };
   ```
   So anthropic always gets `(None, None)` even when the env key is present.

2. **`has_oauth_tokens` conflates env-key with OAuth login.**
   `projection.rs::project_one` (`:76`):
   ```rust
   let has_oauth_tokens = is_oauth && info.configured;
   ```
   `is_oauth_provider("anthropic", ...)` is true, and `configured` is true when
   `ANTHROPIC_API_KEY` is set (catalog `available` includes the env var). So a mere
   env API key makes `has_oauth_tokens = true` → the projection emits the
   `Logout from OAuth [Anthropic]` row instead of an api-key row.

## TS reference behaviour
`PROVIDER_ENV_VARS` (`src/utils/credentials.ts:65-84`) maps
`anthropic: ['ANTHROPIC_API_KEY', 'CLAUDE_CODE_OAUTH_TOKEN']`. `getProviderConfig`
reads the env var regardless of OAuth and reports a masked `[env]` key. OAuth
"logged-in" status is driven by an actual OAuth token / auth file
(`has_claude_auth()` analog), NOT by the api key.

## Fix direction
1. In `management.rs::list_providers_info`, derive `masked_key`/`source` for an
   OAuth provider that *also* declares a non-empty `env_var` when that env var is
   set (anthropic). I.e. mask whenever `available && !env_var.is_empty()` and the
   env var actually holds a value — independent of `AuthType`.
2. Distinguish "has env api key" from "has OAuth tokens". `has_oauth_tokens`
   should be true only when an actual OAuth token/auth file is present
   (`ProviderCredentials::has_claude_auth()` for anthropic), NOT merely because
   `configured` is true via the env var. Anthropic with only `ANTHROPIC_API_KEY`
   set must render the api-key row (`✓ … [env]`), not the logout row.
3. Anthropic should still expose the OAuth login rows ("Sign in with browser",
   "Sign in with code") AND the api-key row — matching the TS screenshot which
   shows both `Login with Claude (browser/headless)` and `API key ✓ … [env]`.

## Interaction with PROV-098
The masked-key render itself lands in PROV-098. PROV-099 ensures the data is
*produced* for anthropic (backend masking) and that the OAuth-vs-apikey
classification is correct.

## Files in play
- `codelet/providers/src/custom/management.rs:145-176`
- `codelet/providers/src/catalog.rs` (anthropic entry)
- `codelet/providers/src/credentials.rs` (`has_claude_auth`, `detect`)
- `codelet/fspec-tui/src/views/provider_settings/projection.rs:71-115`

## Acceptance pointers
- Anthropic with ONLY `ANTHROPIC_API_KEY` set → masked `[env]` api-key row, NO
  "Logout from OAuth" row.
- Anthropic with an OAuth auth file present → "Logout from OAuth" row.
- Deterministic: path-injectable auth-file dir + env var control; no network.
